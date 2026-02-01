pub mod automatch;

pub use automatch::AutomatchResult;

use std::collections::HashMap;

pub type SeasonMap = HashMap<u32, FormatData>;

#[derive(Debug)]
pub struct Series {
    pub formats: HashMap<anime::Format, SeasonMap>,
}

#[derive(Debug)]
pub struct FormatData {
    pub info: anime::Anime,
    pub episodes: anime_detect::EpisodeSet,
    pub in_sync: bool,
}
