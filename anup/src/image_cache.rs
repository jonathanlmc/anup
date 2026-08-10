use std::{
    collections::HashMap,
    fmt::Debug,
    io,
    num::NonZeroUsize,
    path::{Path, PathBuf},
    sync::Arc,
};

use futures::{
    FutureExt, TryFutureExt,
    future::{self, BoxFuture},
};
use lru::LruCache;
use parking_lot::Mutex;
use tap::Tap;

use crate::util;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    ImageError(#[from] image::ImageError),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("fetch from url failed with status {0}")]
    UrlFetchFailed(reqwest::StatusCode),
    #[error("image download did not complete: {0}")]
    UrlFetchIncomplete(reqwest::Error),
    #[error("fetched image exceeds max size of {max_size_bytes} bytes")]
    UrlFetchTooLarge { max_size_bytes: usize },
    #[error("url does not contain a valid image")]
    UrlNotImage,
    #[error("the inferred image format `{inferred_extension}` is not supported")]
    UnsupportedInferredFormat { inferred_extension: &'static str },
    #[error("task encountered panic or was cancelled")]
    TaskFailed,
    #[error("internal reqwest error while fetching image: {0}")]
    Reqwest(#[from] reqwest::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

const INITIAL_URL_FETCH_CAPACITY: usize = 128 * 1024; // 128 KiB

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Hash(u128);

impl Hash {
    pub fn new(bytes: &[u8]) -> Self {
        Self(xxhash_rust::xxh3::xxh3_128(bytes))
    }
}

impl From<&reqwest::Url> for Hash {
    fn from(value: &reqwest::Url) -> Self {
        Self::new(value.as_str().as_bytes())
    }
}

#[derive(Debug)]
pub struct ImageCache {
    cached: Arc<Mutex<CachedItems>>,
    storage_dir: Option<PathBuf>,
    max_img_size_bytes: usize,
}

impl ImageCache {
    /// Initialize a new image cache that writes fetched images to the given [`storage_dir`]
    /// and stores [`MAX_MEM_CACHE_ITEMS`] in memory.
    ///
    /// If [`storage_dir`] is [`None`], only the in-memory cache will be used.
    pub fn new(
        storage_dir: impl Into<Option<PathBuf>>,
        max_mem_cache_items: NonZeroUsize,
        max_img_size_bytes: usize,
    ) -> Self {
        Self {
            cached: Arc::new(Mutex::new(CachedItems::new(max_mem_cache_items))),
            storage_dir: storage_dir.into(),
            max_img_size_bytes,
        }
    }

    // todo: support offline mode (i.e. add option to avoid making a request to the url)
    pub async fn ensure_cached(
        &self,
        client: reqwest::Client,
        url: reqwest::Url,
    ) -> std::result::Result<Arc<image::DynamicImage>, Arc<Error>> {
        use std::collections::hash_map::Entry;

        let hash = Hash::from(&url);

        let resolve_future = {
            let mut cached = self.cached.lock();

            if let Some(img) = cached.items.get(&hash).cloned() {
                return Ok(img);
            }

            match cached.in_flight.entry(hash) {
                // another caller may have started caching the same image, in
                // which case we can just await the first future that's processing it
                Entry::Occupied(entry) => entry.get().clone(),
                // otherwise we can create a future to start processing it and store it
                // for subsequent overlapping requests
                Entry::Vacant(entry) => {
                    let future = Self::cache_from_fallback_sources(
                        hash,
                        url,
                        self.storage_dir.clone(),
                        client,
                        self.max_img_size_bytes,
                    )
                    // errors must be cloneable, since the future is shared
                    .map_err(Arc::new)
                    .boxed()
                    .shared();

                    entry.insert(future).clone()
                }
            }
        };

        let result = resolve_future.await;
        let mut cached = self.cached.lock();

        cached.in_flight.remove(&hash);

        if let Ok(img) = &result {
            cached.items.put(hash, img.clone());
        }

        result
    }

    async fn cache_from_fallback_sources(
        hash: Hash,
        url: reqwest::Url,
        storage_dir: Option<PathBuf>,
        client: reqwest::Client,
        max_fetch_size_bytes: usize,
    ) -> Result<Arc<image::DynamicImage>> {
        let fetched = Self::fetch_from_fallback_sources(
            storage_dir.as_deref(),
            &client,
            url,
            hash,
            max_fetch_size_bytes,
        )
        .await?;

        if let Some(path) = fetched.save_path {
            let res = Self::save_to_disk(fetched.image.clone(), fetched.format, path.clone()).await;

            if let Err(err) = res {
                // non-fatal; we can still use the in-memory cache
                tracing::warn!(?path, ?err, "failed to save cached image to disk");
            }
        }

        Ok(fetched.image)
    }

    async fn save_to_disk(
        image: Arc<image::DynamicImage>,
        format: image::ImageFormat,
        path: PathBuf,
    ) -> Result<()> {
        tokio::task::spawn_blocking(move || image.save_with_format(path, format))
            .await
            .map_err(|_| Error::TaskFailed)?
            .map_err(Into::into)
    }

    async fn fetch_from_fallback_sources(
        storage_dir: Option<&Path>,
        client: &reqwest::Client,
        url: reqwest::Url,
        hash: Hash,
        max_fetch_size_bytes: usize,
    ) -> Result<FallbackFetch> {
        // first, try to fetch from disk (if configured)
        let img_storage_path = if let Some(storage_dir) = storage_dir {
            let path = storage_dir.to_owned().tap_mut(|d| {
                d.push(format!("{:x}", hash.0));
            });

            match Self::fetch_from_disk(path.clone()).await {
                Ok(Some(AnyImageWithFormat { image, format })) => {
                    return Ok(FallbackFetch {
                        image: Arc::new(image),
                        format,
                        // returning a path implies that the image needs to be saved to disk; skip it here
                        save_path: None,
                    });
                }
                Ok(None) => {
                    tracing::trace!(?hash, ?url, "image not stored on disk; fetching from url");
                }
                Err(err) => {
                    tracing::warn!(
                        ?hash,
                        ?url,
                        ?err,
                        "failed to load cached image from disk; fetching from url"
                    );
                }
            }

            Some(path)
        } else {
            None
        };

        // fetching from the url is our last resort; failures should be propagated to indicate that
        // the image could not be obtained at all
        Self::fetch_from_url(client, url, max_fetch_size_bytes)
            .await
            .map(|i| FallbackFetch {
                image: Arc::new(i.image),
                format: i.format,
                save_path: img_storage_path,
            })
    }

    async fn fetch_from_disk(path: PathBuf) -> Result<Option<AnyImageWithFormat>> {
        if !tokio::fs::try_exists(&path).await? {
            return Ok(None);
        }

        tokio::task::spawn_blocking(move || -> _ {
            let reader = image::ImageReader::open(path)
                .map_err(image::ImageError::IoError)?
                .with_guessed_format()
                .map_err(image::ImageError::IoError)?;

            let format = reader.format().ok_or_else(|| {
                use image::error::{DecodingError, ImageFormatHint};

                image::ImageError::Decoding(DecodingError::from_format_hint(
                    ImageFormatHint::Unknown,
                ))
            })?;

            let image = reader.decode()?;

            Ok(Some(AnyImageWithFormat { image, format }))
        })
        .await
        .map_err(|_| Error::TaskFailed)?
    }

    async fn fetch_from_url(
        client: &reqwest::Client,
        url: reqwest::Url,
        max_size_bytes: usize,
    ) -> Result<AnyImageWithFormat> {
        let unverified_image_bytes = {
            let resp = client.get(url).send().await?;
            let status = resp.status();

            if !status.is_success() {
                return Err(Error::UrlFetchFailed(status));
            }

            // stream the response body up to the configured max image size
            //
            // although unlikely for this kind of application, this is intended to stop
            // attacks (and bugs) from requests that stream very large / infinite responses
            let bytes = util::stream::read_up_to_n_bytes(
                &mut resp.bytes_stream(),
                max_size_bytes,
                INITIAL_URL_FETCH_CAPACITY,
            )
            .await
            .map_err(|err| {
                if err.is_request() {
                    Error::UrlFetchIncomplete(err)
                } else {
                    err.into()
                }
            })?;

            match bytes {
                util::stream::LimitedReadResult::UnderLimit(bytes) => bytes,
                // a partial response won't produce a valid image; bail early with a more helpful error
                // than one that just says the image failed to load
                util::stream::LimitedReadResult::Limited(_) => {
                    return Err(Error::UrlFetchTooLarge { max_size_bytes });
                }
            }
        };

        // don't blindly trust the fetched content; verify that it contains an image header
        let inferred_extension = infer::get(&unverified_image_bytes)
            .filter(|t| t.matcher_type() == infer::MatcherType::Image)
            .map(|t| t.extension())
            .ok_or(Error::UrlNotImage)?;

        let format = image::ImageFormat::from_extension(inferred_extension)
            .ok_or_else(|| Error::UnsupportedInferredFormat { inferred_extension })?;

        let image = tokio::task::spawn_blocking(move || {
            image::load_from_memory_with_format(&unverified_image_bytes, format)
        })
        .await
        .map_err(|_| Error::TaskFailed)??;

        Ok(AnyImageWithFormat { image, format })
    }
}

type InFlightFuture =
    future::Shared<BoxFuture<'static, std::result::Result<Arc<image::DynamicImage>, Arc<Error>>>>;

struct CachedItems {
    items: LruCache<Hash, Arc<image::DynamicImage>>,
    in_flight: HashMap<Hash, InFlightFuture>,
}

impl Debug for CachedItems {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CachedItems")
            .field("items", &self.items)
            .finish_non_exhaustive()
    }
}

impl CachedItems {
    fn new(item_cap: NonZeroUsize) -> Self {
        Self {
            items: LruCache::new(item_cap),
            in_flight: HashMap::new(),
        }
    }
}

struct FallbackFetch {
    image: Arc<image::DynamicImage>,
    format: image::ImageFormat,
    save_path: Option<PathBuf>,
}

struct AnyImageWithFormat {
    image: image::DynamicImage,
    format: image::ImageFormat,
}
