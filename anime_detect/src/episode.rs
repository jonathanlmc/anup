use std::{collections::HashSet, hash::Hash, path::PathBuf};

use crate::{combinator, series::SeriesType};

pub type EpisodeSet = HashSet<Episode>;

#[derive(Debug)]
pub struct Episode {
    pub number: u32,
    pub season_hint: Option<u32>,
    pub series_type_hint: SeriesType,
    pub path: PathBuf,
}

impl Episode {
    pub fn parse_with_known_filename(path: PathBuf, filename: &str) -> Option<Self> {
        let parsed = combinator::episode::parse_filename(filename).ok()?;

        Some(Self {
            number: parsed.number,
            season_hint: parsed.season_hint,
            series_type_hint: parsed.series_type_hint,
            path,
        })
    }
}

impl PartialEq for Episode {
    fn eq(&self, other: &Self) -> bool {
        self.season_hint == other.season_hint
            && self.number == other.number
            && self.series_type_hint == other.series_type_hint
    }
}

impl Eq for Episode {}

impl Hash for Episode {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.season_hint.hash(state);
        self.number.hash(state);
        self.series_type_hint.hash(state);
    }
}
