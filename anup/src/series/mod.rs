pub mod automatch;

pub use automatch::AutomatchResult;

use std::collections::HashMap;

pub type SeasonMap = HashMap<u32, FormatData>;

#[derive(Debug)]
pub struct Series {
    pub formats: HashMap<anime::Format, SeasonMap>,
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
