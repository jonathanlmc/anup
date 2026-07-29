pub mod local;

pub use local::LocalRoot;

use indexmap::IndexMap;

use crate::series::episode;

pub type FormatMap = IndexMap<super::Format, SeasonMap>;
pub type SeasonMap = IndexMap<u16, RemoteSeasonPairing>;

#[derive(Debug)]
pub struct RootPairing {
    pub name: String,
    pub pairings: FormatMap,
}

impl RootPairing {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            pairings: FormatMap::default(),
        }
    }

    /// Return an iterator over all formats and their seasons.
    ///
    /// Items are returned by format, and then by season number.
    pub fn all_format_seasons(&self) -> impl Iterator<Item = FormatSeasonRef<'_>> {
        self.pairings.iter().flat_map(|(format, seasons)| {
            seasons.iter().map(|(season, pairing)| FormatSeasonRef {
                format,
                season,
                pairing,
            })
        })
    }
}

pub struct FormatSeasonRef<'a> {
    pub format: &'a super::Format,
    pub season: &'a u16,
    pub pairing: &'a RemoteSeasonPairing,
}

#[derive(Debug)]
pub enum RemoteSeasonPairing {
    Paired {
        remote_info: anime::Anime,
        // todo: display in interface
        #[allow(unused)]
        local_episodes: episode::Set,
        // todo: display in interface
        #[allow(unused)]
        unpaired_local_episodes: episode::Set,
        // todo: hook up to remote api
        in_sync: bool,
    },
    Unpaired(
        // todo: display in interface
        #[allow(unused)] episode::Set,
    ),
}
