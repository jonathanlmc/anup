use std::collections::HashMap;

use anime::api::AnimeInfo;
use anyhow::Context;
use indexmap::IndexMap;

use crate::series::{FormatData, SeasonMap, Series};

pub enum AutomatchResult {
    Matched(Series),
    None {
        local_series: anime_detect::TopLevelSeries,
        searched_anime: Vec<anime::Anime>,
    },
}

/// Automatically match all series formats and any continuous seasons within a local
/// series to one or more anime on a remote service.
///
/// Any episodes that could not be mapped to an anime will be placed into season 0
/// for their respective format in the returned [`Series`].
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
/// * `Title - 01.mkv` -> Start of season 1, or S01E01.
/// * `Title - 02.mkv` -> S01E02
/// * `Title - 03.mkv` -> S01E03
/// * `Title - 04.mkv` -> First episode of season 2. In other words, this is now S02E01.
/// * `Title - 05.mkv` -> S02E02
/// * `Title - 06.mkv` -> S02E03
/// * `Title - 07.mkv` -> First episode of season 3, or S03E01`.
/// * `Title - 08.mkv` -> S03E02
/// * `Title - 09.mkv` -> S03E03
pub async fn all_formats_and_seasons<S: anime::api::Service>(
    local_data: anime_detect::TopLevelSeries,
    anime_service: &S,
) -> anyhow::Result<AutomatchResult> {
    let searched_anime = anime_service
        .search_by_name(&crate::REQWEST_CLIENT, &local_data.parsed_name)
        .await?
        .map(|anime| (anime.id(), anime.into()))
        .collect::<HashMap<_, _>>();

    let scored_formats = ScoredFormats::from_searched_anime(local_data, &searched_anime);

    if scored_formats.is_empty() {
        return Ok(AutomatchResult::None {
            local_series: scored_formats.local_data,
            searched_anime: searched_anime.into_values().collect(),
        });
    }

    let mut pair_details = scored_formats
        .pair_with_searched_anime(searched_anime, anime_service)
        .await
        .context("pairing local series with searched anime failed")?;

    pair_details.paired_formats.sort_unstable_keys();

    let series = Series {
        parsed_local_name: pair_details.parsed_local_name,
        formats: pair_details.paired_formats,
        episodes_without_paired_format: pair_details.episodes_without_paired_format,
    };

    Ok(AutomatchResult::Matched(series))
}

type HighestScore = u32;

struct ScoredFormats {
    scores: HashMap<anime::Format, (HighestScore, anime::AnimeID)>,
    local_data: anime_detect::TopLevelSeries,
}

impl ScoredFormats {
    /// Score each local series format with all provided anime.
    ///
    /// Scores will only be recorded for formats that have a fairly strong (70%)
    /// similarity to one of the provided anime.
    fn from_searched_anime(
        local_data: anime_detect::TopLevelSeries,
        searched_anime: &HashMap<anime::AnimeID, anime::Anime>,
    ) -> Self {
        use std::collections::hash_map::Entry;

        const SCORE_SCALE: u32 = 10_000;
        const CONFIDENT_SCORE: u32 = 70 * SCORE_SCALE;
        const MATCHING_FORMAT_SCORE_ADJUSTMENT: u32 = 25 * SCORE_SCALE;

        let local_name_lowercase = local_data.parsed_name.to_ascii_lowercase();
        let mut format_scores = HashMap::with_capacity(local_data.episodes.len());

        for (anime_id, anime) in searched_anime {
            // compute similarity score between the local name and searched anime titles
            let score = [
                Some(&anime.title.romaji),
                Some(&anime.title.native),
                anime.title.english.as_ref(),
            ]
            .into_iter()
            .flatten()
            .map(|anime_title| {
                (strsim::jaro(&local_name_lowercase, &anime_title.to_ascii_lowercase())
                    * 100.
                    * SCORE_SCALE as f64) as u32
            })
            .max()
            .unwrap_or_default();

            // score each series format in the local data by its title & format similarity
            // to the current anime data
            //
            // todo: score by season hint as well
            for local_format in local_data.episodes.keys().copied().map(Into::into) {
                let format_score = if let Some(anime_fmt) = anime.format
                    && anime_fmt == local_format
                {
                    // todo: make adjustable
                    score + MATCHING_FORMAT_SCORE_ADJUSTMENT
                } else {
                    score.saturating_sub(MATCHING_FORMAT_SCORE_ADJUSTMENT)
                };

                tracing::trace!(
                    local_name = %local_data.parsed_name,
                    anime_titles = ?anime.title,
                    anime_format = ?anime.format,
                    score = %format_score,
                    "computed confidence score for automatch entry"
                );

                // fast path: avoid logging this series if it won't qualify anyway
                if format_score < CONFIDENT_SCORE {
                    continue;
                }

                // store / replace the score the for the format if it's the best one seen yet
                match format_scores.entry(local_format) {
                    Entry::Occupied(mut entry) => {
                        let (score, media_id) = entry.get_mut();

                        if format_score > *score {
                            tracing::trace!(
                                local_name = %local_data.parsed_name,
                                anime_titles = ?anime.title,
                                anime_format = ?anime.format,
                                score = %format_score,
                                old_score = %*score,
                                "new higher score for automatch entry"
                            );

                            *media_id = *anime_id;
                            *score = format_score;
                        }
                    }
                    Entry::Vacant(entry) => {
                        entry.insert((format_score, *anime_id));
                    }
                }
            }
        }

        Self {
            scores: format_scores,
            local_data,
        }
    }

    /// Returns true if no scores were recorded for any formats.
    fn is_empty(&self) -> bool {
        self.scores.is_empty()
    }

    /// Link each scored format with its best matching anime.
    ///
    /// This will resolve any continuous seasons detected in any
    /// of the local series formats as well, and may take a while to complete.
    async fn pair_with_searched_anime(
        mut self,
        mut searched_anime: HashMap<anime::AnimeID, anime::Anime>,
        anime_service: &impl anime::api::Service,
    ) -> anyhow::Result<PairingDetails> {
        let mut paired_formats = IndexMap::with_capacity(self.scores.len());

        for (format, (_, anime_id)) in self.scores {
            // each format should only be able to reference one unique series
            let Some(anime) = searched_anime.remove(&anime_id) else {
                tracing::debug!(
                    ?format,
                    %anime_id,
                    "encountered duplicate anime for format; not pairing format to any anime series"
                );

                continue;
            };

            let Some(episodes) = self.local_data.episodes.remove(&format.into()) else {
                anyhow::bail!(
                    "stored a format score for a format not contained in local episode files; this is a bug"
                );
            };

            let mut resolved_seasons = SeasonMap::with_capacity(1);

            pair_anime_seasons_from_local_episodes(
                anime_id,
                anime,
                episodes,
                &mut resolved_seasons,
                anime_service,
            )
            .await?;

            paired_formats.insert(format, resolved_seasons);
        }

        Ok(PairingDetails {
            parsed_local_name: self.local_data.parsed_name,
            paired_formats,
            episodes_without_paired_format: self.local_data.episodes,
        })
    }
}

struct PairingDetails {
    parsed_local_name: String,
    paired_formats: IndexMap<anime::Format, SeasonMap>,
    episodes_without_paired_format: HashMap<anime_detect::series::Format, anime_detect::EpisodeSet>,
}

/// Link a local set of episodes to an anime season, and resolve any
/// continuous seasons located in the episodes.
///
/// All resolved seasons will be inserted into the provided `resolved_seasons` map.
///
/// If any extra episodes are present that do not map to a season, they will
/// be inserted into season 0 within the map.
async fn pair_anime_seasons_from_local_episodes(
    anime_id: anime::AnimeID,
    anime: anime::Anime,
    mut episodes: anime_detect::EpisodeSet,
    resolved_seasons: &mut SeasonMap,
    anime_service: &impl anime::api::Service,
) -> anyhow::Result<bool> {
    let highest_episode_num = episodes.iter().map(|ep| ep.number).max().unwrap_or(0);

    // calculate an episode offset if the highest episode number exceeds
    // the provided anime's episode count
    //
    // (i.e. there is at least one continuous season in the episodes)
    let episode_offset = anime
        .episodes
        .and_then(|season_eps| (highest_episode_num > season_eps).then_some(season_eps));

    let mut season_num = 1;

    // fast path: if no episode offset was calculated, we can assume there is
    // no continuous season and return all episodes as the first season
    let Some(mut episode_offset) = episode_offset else {
        resolved_seasons.insert(
            season_num,
            FormatData::Matched {
                info: anime,
                episodes,
                in_sync: false,
            },
        );

        return Ok(false);
    };

    // otherwise, find all episodes *within* the first season
    // for the first resolved season
    resolved_seasons.insert(
        season_num,
        FormatData::Matched {
            info: anime,
            episodes: episodes
                .extract_if(|ep| ep.number <= episode_offset)
                .collect(),
            in_sync: false,
        },
    );

    let mut any_season_missing = false;
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
            tracing::warn!("found a series sequel, but its anime id does not exist");
            any_season_missing = true;
            break;
        };

        current_sequel = sequel_id;
        season_num += 1;

        let sequel = sequel.into();
        let num_sequel_eps = sequel.episodes;

        tracing::debug!(%sequel_id, ?num_sequel_eps, "found sequel for series");

        let sequel_episodes = episodes
            .extract_if(|ep| {
                let offset_ep = ep.number.saturating_sub(episode_offset);
                offset_ep <= num_sequel_eps.unwrap_or(offset_ep)
            })
            .collect::<anime_detect::EpisodeSet>();

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

        resolved_seasons.insert(
            season_num,
            FormatData::Matched {
                info: sequel,
                episodes: sequel_episodes,
                in_sync: false,
            },
        );

        episode_offset += num_sequel_eps.unwrap_or(0);
    }

    // any remaining episodes can go in to a "special" season 0 since all matched seasons start at 1
    if !episodes.is_empty() {
        resolved_seasons.insert(0, FormatData::Unmatched { episodes });
    }

    Ok(any_season_missing)
}
