use std::{
    collections::HashSet,
    ffi::OsStr,
    hash::Hash,
    path::{Path, PathBuf},
};

use crate::series::{self, LocalEpisodeInfo};

pub type Set = HashSet<Episode>;

const CONTAINER_EXTENSIONS: [&str; 17] = [
    "avi", "flv", "m2ts", "mts", "m4v", "mkv", "mov", "mp4", "mpg", "mpeg", "ogv", "ts", "webm",
    "wmv", "3gp", "3g2", "f4v",
];

#[derive(thiserror::Error, Debug)]
pub enum ParseError {
    #[error("filename could not be parsed for media information")]
    Unmatched,
    #[error("filename does not have a known video file extension")]
    InvalidExtension,
    #[error("provided path does not point to a file")]
    NotAFile,
    #[error("filename contains invalid utf8")]
    InvalidUtf8,
}

#[derive(Debug)]
pub struct Episode {
    pub info: LocalEpisodeInfo,
    pub path: PathBuf,
}

impl PartialEq for Episode {
    fn eq(&self, other: &Self) -> bool {
        self.info == other.info
    }
}

impl Eq for Episode {}

impl Hash for Episode {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.info.hash(state);
    }
}

impl Episode {
    pub fn parse(path: impl Into<PathBuf> + AsRef<Path>) -> Result<Self, ParseError> {
        if !path.as_ref().extension().is_some_and(|ext| {
            CONTAINER_EXTENSIONS
                .iter()
                .any(|valid_ext| ext == OsStr::new(valid_ext))
        }) {
            return Err(ParseError::InvalidExtension);
        }

        let stem = path
            .as_ref()
            .file_stem()
            .ok_or(ParseError::NotAFile)?
            .to_str()
            .ok_or(ParseError::InvalidUtf8)?;

        let parsed =
            LocalEpisodeInfo::parse_with_known_filename(stem).map_err(|err| match err {
                anime_detect::Error::Unmatched => ParseError::Unmatched,
            })?;

        Ok(Self {
            info: parsed,
            path: path.into(),
        })
    }
}
