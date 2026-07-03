pub mod automatch;
pub mod episode;
pub mod root;

pub use automatch::AutomatchResult;
pub use episode::Episode;
pub use root::{LocalRoot, RemoteSeasonPairing, RootPairing, local};

type LocalSeriesInfo<'a> = medinpar::Series<'a>;
type LocalEpisodeInfo = medinpar::Episode<Format>;

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

impl Format {
    /// Return a string representation of the format in titlecase format.
    pub const fn titlecase_str(self) -> &'static str {
        match self {
            Self::Tv => "TV",
            Self::Special => "Special",
            Self::Movie => "Movie",
            Self::Ona => "ONA",
            Self::Ova => "OVA",
            Self::Music => "Music",
        }
    }
}

impl medinpar::Format for Format {
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
