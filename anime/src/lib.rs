use serde::Deserialize;

pub mod api;

pub type AnimeID = u32;

#[derive(Clone, Debug)]
pub struct Anime {
    pub id: MediaID,
    pub title: Title,
    pub cover_image_url: CoverImage,
    pub episodes: Option<u32>,
    pub format: Option<Format>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Title {
    pub english: Option<String>,
    pub romaji: String,
    pub native: String,
}

impl Title {
    #[inline]
    pub fn as_array(&self) -> [Option<&str>; 3] {
        [
            self.english.as_deref(),
            Some(&self.romaji),
            Some(&self.native),
        ]
    }
}

#[derive(Copy, Clone, Debug, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct MediaID {
    pub anilist: Option<AnimeID>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverImage {
    /// URL pointing to the extra large cover image for the series.
    pub extra_large: Option<String>,
}

#[derive(Debug, Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub enum Format {
    Tv,
    Movie,
    Special,
    Ona,
    Ova,
    Music,
    Other,
}

mod macros {
    macro_rules! include_str_from_root {
        ($relative_path:expr) => {
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", $relative_path))
        };
    }

    macro_rules! include_graphql {
        ($relative_path:expr) => {
            $crate::macros::include_str_from_root!(concat!("generated/graphql/", $relative_path))
        };
    }

    pub(crate) use include_graphql;
    pub(crate) use include_str_from_root;
}
