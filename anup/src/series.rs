pub mod automatch;
pub mod episode;
pub mod root;

pub use automatch::AutomatchResult;
pub use episode::Episode;
pub use root::{LocalRoot, RemoteSeasonPairing, RootPairing};

use indexmap::IndexMap;

use std::collections::HashMap;

type LocalSeriesInfo<'a> = anime_detect::Series<'a>;
type LocalEpisodeInfo = anime_detect::Episode<Format>;

#[derive(Debug, Copy, Clone, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Format {
    #[default]
    Tv,
    Special,
    Movie,
    Ona,
    Ova,
    Music,
}

impl anime_detect::Format for Format {
    const VARIANT_MAPPINGS: &[(&'static str, Self)] = &[
        ("tv", Self::Tv),
        ("special", Self::Special),
        ("specials", Self::Special),
        ("movie", Self::Movie),
        ("ona", Self::Ona),
        ("ova", Self::Ova),
        ("music", Self::Music),
    ];
}

impl From<Format> for anime::Format {
    fn from(value: Format) -> Self {
        match value {
            Format::Tv => Self::Tv,
            Format::Special => Self::Special,
            Format::Movie => Self::Movie,
            Format::Ona => Self::Ona,
            Format::Ova => Self::Ova,
            Format::Music => Self::Music,
        }
    }
}
