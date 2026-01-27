use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use tap::TapFallible;

use crate::{Episode, EpisodeSet, combinator};

#[derive(thiserror::Error, Debug)]
pub enum ParseSeriesError {
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
pub struct Series {
    pub path: PathBuf,
    pub parsed_name: String,
    pub season_episodes: HashMap<u32, EpisodeSet>,
}

impl Series {
    pub fn parse_dir(dir: PathBuf) -> Result<Self, ParseSeriesError> {
        let series_name = dir
            .file_name()
            .ok_or(ParseSeriesError::InvalidPath)
            .map(|name| name.to_string_lossy())
            .map(|name| {
                combinator::series::trim_name(&name)
                    .tap_err(|err| {
                        tracing::warn!(
                            %err,
                            "failed to parse series name at `{}`; \
                            using full directory name",
                            dir.display()
                        );
                    })
                    .unwrap_or_else(|_| name.into_owned())
            })?;

        let mut season_episodes = HashMap::new();
        collect_episodes_in_dir(&dir, &mut season_episodes, true)?;

        Ok(Self {
            path: dir,
            parsed_name: series_name,
            season_episodes,
        })
    }
}

fn collect_episodes_in_dir(
    dir: &Path,
    season_episodes: &mut HashMap<u32, EpisodeSet>,
    follow_nested_dirs: bool,
) -> Result<(), ParseSeriesError> {
    for entry in dir.read_dir()? {
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

        let metadata = match entry.metadata() {
            Ok(md) => md,
            Err(err) => {
                tracing::error!(
                    "failed to get metadata for directory entry in `{}`: {:?}",
                    dir.display(),
                    err
                );

                continue;
            }
        };

        tracing::debug!("analyzing `{}`", entry.path().display());

        let filename = entry.file_name();
        let filename = filename.to_string_lossy();

        if metadata.is_dir() {
            if !follow_nested_dirs {
                tracing::warn!("skipping nested directory in `{}`", dir.display());
                continue;
            }

            // we can assume that a nested directory is a separate season, so try to
            // parse episodes in it
            collect_episodes_in_dir(&entry.path(), season_episodes, false)?;
        } else {
            let episode = match Episode::parse_with_known_filename(entry.path(), &filename) {
                Some(ep) => ep,
                None => {
                    tracing::error!(
                        "failed to detect valid episode at `{}`",
                        entry.path().display(),
                    );

                    continue;
                }
            };

            tracing::debug!(
                num = %episode.number,
                season_hint = ?episode.season_hint,
                path = %episode.path.display(),
                "analyzed episode"
            );

            let season_eps = season_episodes
                .entry(episode.season_hint.unwrap_or(1))
                .or_default();

            if let Some(replaced) = season_eps.replace(episode) {
                // don't try to recover from this, as it indicates some weird path layout
                // is being used
                return Err(ParseSeriesError::DuplicateEpisodesDetected {
                    first_episode: replaced.path,
                    second_episode: entry.path(),
                });
            }
        }
    }

    Ok(())
}
