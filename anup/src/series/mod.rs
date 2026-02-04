pub mod automatch;

pub use automatch::AutomatchResult;
use indexmap::IndexMap;

use std::collections::HashMap;

pub type SeasonMap = IndexMap<u32, FormatData>;

#[derive(Debug)]
pub struct Series {
    pub parsed_local_name: String,
    pub formats: IndexMap<anime::Format, SeasonMap>,
    pub episodes_without_paired_format:
        HashMap<anime_detect::series::Format, anime_detect::EpisodeSet>,
}

#[derive(Debug)]
pub enum FormatData {
    Matched {
        info: anime::Anime,
        episodes: anime_detect::EpisodeSet,
        in_sync: bool,
    },
    Unmatched {
        episodes: anime_detect::EpisodeSet,
    },
}
