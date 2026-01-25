use std::{collections::HashSet, hash::Hash, path::PathBuf};

pub type EpisodeSet = HashSet<Episode>;

#[derive(Debug, Eq, PartialEq)]
pub struct Episode {
    pub number: u32,
    pub season_hint: Option<u32>,
    pub path: PathBuf,
}

impl Episode {
    pub fn parse_with_known_filename(path: PathBuf, _filename: &str) -> Option<Self> {
        // TODO
        let (number, season_hint) = (0, None);

        Some(Self {
            number,
            season_hint,
            path,
        })
    }
}

impl Hash for Episode {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.number.hash(state);
    }
}
