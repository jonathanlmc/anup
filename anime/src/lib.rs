use serde::Deserialize;

pub mod api;

#[derive(Clone, Debug)]
pub struct Anime {
    pub id: MediaID,
    pub title: Title,
    pub episodes: Option<u32>,
    pub format: Option<Format>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Title {
    pub english: Option<String>,
    pub romaji: String,
    pub native: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct MediaID {
    pub ani_list: Option<u32>,
}

#[derive(Debug, Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub enum Format {
    TV,
    Movie,
    Special,
    ONA,
    OVA,
    Music,
    Other,
}

mod macros {
    macro_rules! include_from_root {
        ($relative_path:expr) => {
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", $relative_path))
        };
    }

    pub(crate) use include_from_root;
}
