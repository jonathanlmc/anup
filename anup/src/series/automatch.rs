use std::collections::HashMap;

use anime::api::AnimeInfo;
use anyhow::Context;
use indexmap::IndexMap;

use crate::series::{self, episode};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("api error: {0}")]
    AnimeApi(#[from] anime::api::Error),
    #[error("format score for format not in local episode files was stored; this is a bug")]
    InvalidFormatScoreStored,
}

pub type Result<T> = std::result::Result<T, Error>;

pub enum AutomatchResult {
    Paired(series::RootPairing),
    Unpaired(series::LocalRoot),
}

/// Automatically match all series formats and any continuous seasons within a local
/// series to one or more anime on a remote service.
///
/// Any episodes that could not be mapped to an anime will be placed into season 0
/// for their respective format in the returned [`series::RootPairing`].
///
/// ## Continuous Seasons
///
/// For each provided series format, this function will detect any episodes belonging
/// to a separate series / season that are laid out sequentially and add them to a new
/// season for the format appropriately. This process involves making multiple API calls
/// to the provided `anime_service`, so this function can take a long time to complete
/// depending on the number of sequels that need to be analyzed.
///
/// For additional context, here is an example of some episode files that contain
/// a continuous season. Assume each season has 3 episodes:
///
/// * `Title - 01` -> Start of season 1, or S01E01.
/// * `Title - 02` -> S01E02
/// * `Title - 03` -> S01E03
/// * `Title - 04` -> First episode of season 2. In other words, this is now S02E01.
/// * `Title - 05` -> S02E02
/// * `Title - 06` -> S02E03
/// * `Title - 07` -> First episode of season 3, or S03E01.
/// * `Title - 08` -> S03E02
/// * `Title - 09` -> S03E03
pub async fn all_formats_and_seasons<S: anime::api::Service>(
    local_series: series::LocalRoot,
    anime_service: &S,
) -> Result<AutomatchResult> {
    let searched_anime = anime_service
        .search_by_name(&crate::REQWEST_CLIENT, &local_series.parsed_name)
        .await?
        .map(|info| (info.id(), info.into()))
        .collect::<IndexMap<_, _>>();

    let scored_formats = ScoredFormats::from_remote_anime(&local_series, searched_anime.iter());

    if scored_formats.is_empty() {
        return Ok(AutomatchResult::Unpaired(local_series));
    }

    let series_root = scored_formats
        .pair_formats_to_new_series_root(local_series, searched_anime, anime_service)
        .await?;

    Ok(AutomatchResult::Paired(series_root))
}

#[derive(Debug)]
struct ScoredAnimeInfo {
    score: u32,
    remote_id: anime::AnimeID,
}

struct ScoredFormats {
    best_format_matches: HashMap<series::Format, ScoredAnimeInfo>,
}

impl ScoredFormats {
    /// Score each local series format with all provided anime.
    ///
    /// Scores will only be recorded for formats that have a fairly strong (70%)
    /// similarity to one of the provided anime.
    fn from_remote_anime<'a>(
        local_series: &series::LocalRoot,
        anime_entries: impl Iterator<Item = (&'a anime::AnimeID, &'a anime::Anime)>,
    ) -> Self {
        use std::collections::hash_map::Entry;

        const SCORE_SCALE: u32 = 10_000;
        const CONFIDENT_SCORE: u32 = 70 * SCORE_SCALE;
        const MATCHING_FORMAT_SCORE_ADJUSTMENT: u32 = 25 * SCORE_SCALE;

        let local_name_lower = local_series.parsed_name.to_ascii_lowercase();
        let mut format_scores = HashMap::with_capacity(local_series.episodes.len());

        for (anime_id, anime_info) in anime_entries {
            // compute similarity score between the local name and searched anime titles
            let score = anime_info
                .title
                .as_array()
                .into_iter()
                .flatten()
                .map(|anime_title| {
                    (strsim::jaro(&local_name_lower, &anime_title.to_ascii_lowercase())
                        * 100.
                        * SCORE_SCALE as f64) as u32
                })
                .max()
                .unwrap_or_default();

            // score each series format in the local data by its title & format similarity
            // to the current anime data
            //
            // todo: score by season hint as well
            for local_format in local_series.episodes.keys().copied() {
                let format_score = if anime_info.format == Some(local_format.into()) {
                    // todo: make adjustable
                    score + MATCHING_FORMAT_SCORE_ADJUSTMENT
                } else {
                    score.saturating_sub(MATCHING_FORMAT_SCORE_ADJUSTMENT)
                };

                tracing::trace!(
                    local_name = %local_series.parsed_name,
                    anime_titles = ?anime_info.title,
                    anime_format = ?anime_info.format,
                    score = %format_score,
                    "computed confidence score for automatch entry"
                );

                // fast path: avoid logging this series if it won't qualify anyway
                if format_score < CONFIDENT_SCORE {
                    continue;
                }

                Self::store_best_format_entry(
                    *anime_id,
                    local_format,
                    format_score,
                    &mut format_scores,
                );
            }
        }

        Self {
            best_format_matches: format_scores,
        }
    }

    /// Store the anime entry for the given format, and update the existing entry if the given
    /// score is better.
    fn store_best_format_entry(
        anime_id: anime::AnimeID,
        local_format: series::Format,
        score: u32,
        format_scores: &mut HashMap<series::Format, ScoredAnimeInfo>,
    ) {
        use std::collections::hash_map::Entry;

        match format_scores.entry(local_format) {
            Entry::Occupied(mut entry) => {
                let entry = entry.get_mut();

                if score > entry.score {
                    *entry = ScoredAnimeInfo {
                        score,
                        remote_id: anime_id,
                    };
                }
            }
            Entry::Vacant(entry) => {
                entry.insert(ScoredAnimeInfo {
                    score,
                    remote_id: anime_id,
                });
            }
        }
    }

    /// Returns true if no scores were recorded for any formats.
    fn is_empty(&self) -> bool {
        self.best_format_matches.is_empty()
    }

    /// Link each scored format with its best matching anime.
    ///
    /// This will resolve any continuous seasons detected in any
    /// of the local series formats as well, and may take a while to complete.
    async fn pair_formats_to_new_series_root(
        mut self,
        mut local_series: series::LocalRoot,
        mut anime_entries: IndexMap<anime::AnimeID, anime::Anime>,
        anime_service: &impl anime::api::Service,
    ) -> Result<series::RootPairing> {
        let mut series = series::RootPairing::new(local_series.parsed_name);

        for (format, entry) in self.best_format_matches {
            let anime_id = entry.remote_id;

            // each format should only be able to reference one unique series
            let Some(anime) = anime_entries.swap_remove(&anime_id) else {
                tracing::debug!(
                    ?format,
                    %anime_id,
                    local_name = %series.name,
                    "encountered duplicate anime for format; not pairing format to any anime series"
                );

                continue;
            };

            let episodes = local_series
                .episodes
                .remove(&format)
                .ok_or(Error::InvalidFormatScoreStored)?;

            let resolved_seasons =
                pair_local_episodes_to_remote_seasons(anime_id, anime, episodes, anime_service)
                    .await?;

            series.pairings.insert(format, resolved_seasons);
        }

        series.pairings.sort_unstable_keys();

        Ok(series)
    }
}

/// Link a local set of episodes to an anime season, and resolve any
/// continuous seasons located in the episodes.
///
/// All resolved seasons will be inserted into the provided `resolved_seasons` map.
///
/// If any extra episodes are present that do not map to a season, they will
/// be inserted into season 0 within the map.
async fn pair_local_episodes_to_remote_seasons(
    anime_id: anime::AnimeID,
    anime: anime::Anime,
    mut episodes: episode::Set,
    anime_service: &impl anime::api::Service,
) -> Result<series::root::SeasonMap> {
    let highest_episode_num = episodes.iter().map(|ep| ep.info.number).max().unwrap_or(0);

    // calculate an episode offset if the highest episode number exceeds
    // the provided anime's episode count
    //
    // (i.e. there is at least one continuous season in the episodes)
    let episode_offset = anime
        .episodes
        .and_then(|season_eps| (highest_episode_num > season_eps).then_some(season_eps));

    let mut season_num = 1;
    let mut season_map = series::root::SeasonMap::with_capacity(1);

    // fast path: if no episode offset was calculated, we can assume there is
    // no continuous season and return all episodes as the first season
    let Some(mut episode_offset) = episode_offset else {
        season_map.insert(
            season_num,
            series::RemoteSeasonPairing::Paired {
                remote_info: anime,
                local_episodes: episodes,
                unpaired_local_episodes: Default::default(),
                in_sync: false,
            },
        );

        return Ok(season_map);
    };

    // otherwise, find all episodes *within* the first season
    // for the first resolved season
    season_map.insert(
        season_num,
        series::RemoteSeasonPairing::Paired {
            remote_info: anime,
            local_episodes: episodes
                .extract_if(|ep| ep.info.number <= episode_offset)
                .collect(),
            unpaired_local_episodes: Default::default(),
            in_sync: false,
        },
    );

    let mut current_sequel = anime_id;

    // now loop over each sequel to the first series and extract
    // its episode range into a new season
    while let Some(sequel_id) = anime_service
        .sequel_id(&crate::REQWEST_CLIENT, current_sequel)
        .await?
    {
        let Some(sequel) = anime_service
            .get_by_id(&crate::REQWEST_CLIENT, sequel_id)
            .await?
        else {
            tracing::warn!(
                root_anime_id = %anime_id,
                %sequel_id,
                "found a series sequel, but its anime id does not exist"
            );

            break;
        };

        current_sequel = sequel_id;
        season_num += 1;

        let sequel = sequel.into();
        let num_sequel_eps = sequel.episodes;

        tracing::debug!(%sequel_id, ?num_sequel_eps, "found sequel for series");

        let sequel_episodes = episodes
            .extract_if(|ep| {
                let offset_ep = ep.info.number.saturating_sub(episode_offset);
                offset_ep <= num_sequel_eps.unwrap_or(offset_ep)
            })
            .collect::<episode::Set>();

        if sequel_episodes.is_empty() {
            // more episodes remaining indicates that there is likely a gap
            // in the continuous seasons present locally, so try the next sequel
            if !episodes.is_empty() {
                tracing::debug!(
                    top_level_anime_id = %anime_id,
                    %sequel_id,
                    ?num_sequel_eps,
                    "no local episodes present for current sequel; trying next sequel"
                );

                continue;
            }

            tracing::debug!(
                top_level_anime_id = %anime_id,
                %sequel_id,
                ?num_sequel_eps,
                "no local episodes left for sequel; ending season splitting"
            );

            break;
        }

        season_map.insert(
            season_num,
            series::RemoteSeasonPairing::Paired {
                remote_info: sequel,
                local_episodes: sequel_episodes,
                unpaired_local_episodes: Default::default(),
                in_sync: false,
            },
        );

        episode_offset += num_sequel_eps.unwrap_or(0);
    }

    // any remaining episodes can go in to a "special" season 0 since all matched seasons start at 1
    if !episodes.is_empty() {
        season_map.insert(0, series::RemoteSeasonPairing::Unpaired(episodes));
    }

    Ok(season_map)
}
