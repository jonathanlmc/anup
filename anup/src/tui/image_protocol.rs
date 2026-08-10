use std::{fmt::Debug, hash::Hash, mem, num::NonZeroUsize, sync::Arc};

use lru::LruCache;
use parking_lot::{Mutex, MutexGuard};

use crate::image_cache::ImageCache;

type LruImageCache<K> = LruCache<K, ProtocolState>;
type SharedImageCache<K> = Arc<Mutex<LruImageCache<K>>>;

#[derive(Clone)]
pub struct ImageProtocolCache<K> {
    images: SharedImageCache<K>,
}

impl<K> ImageProtocolCache<K>
where
    K: Clone + Debug + Hash + Eq + Send + 'static,
{
    pub fn new(max_size: NonZeroUsize) -> Self {
        Self {
            images: Arc::new(Mutex::new(LruCache::new(max_size))),
        }
    }

    pub async fn ensure_cached(
        &self,
        key: K,
        url: reqwest::Url,
        image_cache: Arc<ImageCache>,
        image_protocol_picker: &ratatui_image::picker::Picker,
        resize_state_change_trigger: Arc<tokio::sync::Notify>,
    ) {
        {
            let mut images = self.images.lock();

            // this method counts as using the image; if we can promote `key`
            // to the top of the LRU cache, then the image is already cached
            if images.promote(&key) {
                return;
            }

            images.put(key.clone(), ProtocolState::Loading);
        }

        let image = match image_cache
            .ensure_cached(crate::REQWEST_CLIENT.clone(), url)
            .await
        {
            Ok(img) => img,
            Err(err) => {
                tracing::warn!(?err, "failed to fetch image");
                // failures are a valid cache state here; we don't want to potentially spam
                // requests to the image url if this method is called often
                self.images.lock().push(key, ProtocolState::Failed);

                return;
            }
        };

        let protocol = image_protocol_picker
            // requires a deep clone of the image data; this can be expensive
            .new_resize_protocol((*image).clone());

        let (resize_tx, resize_rx) = tokio::sync::mpsc::unbounded_channel();

        let threaded_protocol =
            ratatui_image::thread::ThreadProtocol::new(resize_tx, Some(protocol));

        self.images.lock().push(
            key.clone(),
            ProtocolState::Loaded(Box::new(threaded_protocol)),
        );

        tokio::spawn(Self::process_resize_events(
            resize_rx,
            key,
            self.images.clone(),
            resize_state_change_trigger,
        ));
    }

    async fn process_resize_events(
        mut resize_rx: tokio::sync::mpsc::UnboundedReceiver<ratatui_image::thread::ResizeRequest>,
        key: K,
        images: SharedImageCache<K>,
        // wrapped in `Arc` to avoid having to wrap this function in its own async block
        // when used with `tokio::spawn`
        state_change_trigger: Arc<tokio::sync::Notify>,
    ) {
        while let Some(req) = resize_rx.recv().await {
            tracing::debug!(?key, "received resize request for image");

            let mut image_protocol = match images.lock().get_mut(&key) {
                Some(ProtocolState::Loading | ProtocolState::Failed) | None => {
                    // these cases will likely only be encountered if the cache is under a *lot* of pressure
                    // with images containing identical keys, since this resize event handler should no longer
                    // process new events (i.e. get dropped) when an image is purged from the cache
                    tracing::debug!(
                        "image was either missing from cache, loading, or in failed state after performing \
                            resize for it; resizing work was wasted"
                    );

                    continue;
                }
                Some(state @ ProtocolState::Loaded(_)) => {
                    let protocol = match mem::replace(state, ProtocolState::Loading) {
                        ProtocolState::Loaded(protocol) => protocol,
                        // safety: the outer match explicitly captures the loaded variant
                        _ => unreachable!(),
                    };

                    state_change_trigger.notify_one();
                    protocol
                }
            };

            // perform the (expensive) resize
            let resized = match tokio::task::spawn_blocking(move || req.resize_encode()).await {
                Ok(Ok(resized)) => Some(resized),
                Ok(Err(err)) => {
                    tracing::warn!(?key, ?err, "resize event failed for image");
                    None
                }
                Err(err) => {
                    tracing::error!(?key, ?err, "panic while processing resize event for image");
                    None
                }
            };

            let Some(resized) = resized else {
                images.lock().put(key.clone(), ProtocolState::Failed);
                state_change_trigger.notify_one();
                continue;
            };

            image_protocol.update_resized_protocol(resized);

            images
                .lock()
                .put(key.clone(), ProtocolState::Loaded(image_protocol));

            state_change_trigger.notify_one();
        }
    }

    pub fn lock(&self) -> MutexGuard<'_, LruImageCache<K>> {
        self.images.lock()
    }
}

pub enum ProtocolState {
    // boxing recommended by clippy
    Loaded(Box<ratatui_image::thread::ThreadProtocol>),
    Loading,
    Failed,
}
