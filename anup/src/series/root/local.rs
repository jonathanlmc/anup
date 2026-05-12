use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use tap::TapFallible;
use walkdir::WalkDir;

use crate::series::{self, Episode, LocalSeriesInfo, episode};

#[derive(thiserror::Error, Debug)]
pub enum ParseError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("invalid series path provided")]
    InvalidPath,
    #[error("duplicate episodes detected at `{0}` and `{1}`", first_episode.display(), second_episode.display())]
    DuplicateEpisodesDetected {
        first_episode: PathBuf,
        second_episode: PathBuf,
    },
}

#[derive(Debug)]
pub struct LocalRoot {
    // todo: display in interface
    #[allow(unused)]
    pub path: PathBuf,
    pub parsed_name: String,
    pub episodes: HashMap<series::Format, episode::Set>,
}

impl LocalRoot {
    pub fn parse_dir(dir: PathBuf) -> Result<Self, ParseError> {
        let filename = dir
            .file_name()
            .ok_or(ParseError::InvalidPath)?
            .to_string_lossy();

        let parsed_name = LocalSeriesInfo::parse(&filename)
            .map(|s| s.trimmed_name)
            .tap_err(|err| {
                tracing::warn!(
                    %err,
                    "failed to parse series name at `{}`; \
                    using full directory name",
                    dir.display()
                );
            })
            .unwrap_or(&filename);

        let episodes = Self::parse_dir_episodes(&dir)?;

        Ok(Self {
            parsed_name: parsed_name.to_owned(),
            path: dir,
            episodes,
        })
    }

    fn parse_dir_episodes(dir: &Path) -> Result<HashMap<series::Format, episode::Set>, ParseError> {
        let mut episodes = HashMap::<_, episode::Set>::new();

        for entry in WalkDir::new(dir) {
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    tracing::error!(
                        "failed to read directory entry in `{}`: {:?}",
                        dir.display(),
                        err
                    );

                    continue;
                }
            };

            if entry.file_type().is_dir() {
                continue;
            }

            tracing::debug!("analyzing `{}`", entry.path().display());

            let mut episode = match Episode::parse(entry.path()) {
                Ok(ep) => ep,
                Err(err) => {
                    tracing::warn!(
                        "failed to detect valid episode at `{}`: {}",
                        entry.path().display(),
                        err,
                    );

                    continue;
                }
            };

            tracing::debug!(
                num = %episode.info.number,
                season = ?episode.info.season,
                format = ?episode.info.format,
                path = %episode.path.display(),
                "analyzed episode"
            );

            let format = episode.info.format.get_or_insert_default();

            let format_episodes = episodes.entry(*format).or_default();

            if let Some(replaced) = format_episodes.replace(episode) {
                // don't try to recover from this, as it indicates some weird path layout
                // is being used
                return Err(ParseError::DuplicateEpisodesDetected {
                    first_episode: replaced.path,
                    second_episode: entry.into_path(),
                });
            }
        }

        Ok(episodes)
    }
}
