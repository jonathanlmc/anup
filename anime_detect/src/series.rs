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
pub struct TopLevelSeries {
    pub path: PathBuf,
    pub parsed_name: String,
    pub episodes: HashMap<Format, EpisodeSet>,
}

impl TopLevelSeries {
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

        let mut episodes = HashMap::new();
        collect_episodes_in_dir(&dir, &mut episodes, true)?;

        Ok(Self {
            path: dir,
            parsed_name: series_name,
            episodes,
        })
    }
}

#[derive(Debug, Copy, Clone, Default, Hash, PartialEq, Eq)]
pub enum Format {
    #[default]
    TV,
    Special,
    Movie,
    ONA,
    OVA,
    Music,
}

#[cfg(feature = "anime_integration")]
impl PartialEq<anime::Format> for Format {
    fn eq(&self, other: &anime::Format) -> bool {
        use anime::Format::*;

        matches!(
            (self, other),
            (Self::TV, TV)
                | (Self::Special, Special)
                | (Self::Movie, Movie)
                | (Self::ONA, ONA)
                | (Self::OVA, OVA)
        )
    }
}

#[cfg(feature = "anime_integration")]
impl From<anime::Format> for Format {
    fn from(value: anime::Format) -> Self {
        use anime::Format::*;

        match value {
            TV | Other => Self::TV,
            Special => Self::Special,
            Movie => Self::Movie,
            ONA => Self::ONA,
            OVA => Self::OVA,
            Music => Self::Music,
        }
    }
}

#[cfg(feature = "anime_integration")]
impl From<Format> for anime::Format {
    fn from(value: Format) -> Self {
        match value {
            Format::TV => Self::TV,
            Format::Special => Self::Special,
            Format::Movie => Self::Movie,
            Format::ONA => Self::ONA,
            Format::OVA => Self::OVA,
            Format::Music => Self::Music,
        }
    }
}

fn collect_episodes_in_dir(
    dir: &Path,
    episodes: &mut HashMap<Format, EpisodeSet>,
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
            collect_episodes_in_dir(&entry.path(), episodes, false)?;
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

            let type_episodes = episodes.entry(episode.series_type_hint).or_default();

            if let Some(replaced) = type_episodes.replace(episode) {
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
