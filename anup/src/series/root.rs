pub mod local;

pub use local::LocalRoot;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use indexmap::IndexMap;
use tap::TapFallible;

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
}

#[derive(Debug)]
pub enum RemoteSeasonPairing {
    Paired {
        remote_info: anime::Anime,
        local_episodes: episode::Set,
        unpaired_local_episodes: episode::Set,
        in_sync: bool,
    },
    Unpaired(episode::Set),
}
