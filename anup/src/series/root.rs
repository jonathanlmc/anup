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
        self.pairings.iter().flat_map(|(&format, seasons)| {
            seasons
                .iter()
                .map(move |(&season, pairing)| FormatSeasonRef {
                    format,
                    season,
                    pairing,
                })
        })
    }
}

pub struct FormatSeasonRef<'a> {
    pub format: super::Format,
    pub season: u16,
    pub pairing: &'a RemoteSeasonPairing,
}

#[derive(Debug)]
pub enum RemoteSeasonPairing {
    Paired(PairedSeason),
    Unpaired(
        // todo: display in interface
        #[allow(unused)] episode::Set,
    ),
}

#[derive(Debug)]
pub struct PairedSeason {
    pub remote_info: anime::Info,
    // todo: display in interface
    #[allow(unused)]
    pub local_episodes: episode::Set,
    // todo: hook up to remote api
    pub in_sync: bool,
}
