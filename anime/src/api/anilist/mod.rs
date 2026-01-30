//! Integration with the AniList GraphQL API.

pub mod request;

use serde::Deserialize;
use serde_json::json;
use tap::{Pipe, Tap};

use crate::{
    Anime, MediaID, Title,
    api::{Result, Service},
    macros::include_from_root,
};

/// AniList API integration.
pub struct AniList;

impl Service for AniList {
    /// Get an anime by its AniList ID. [`None`] will be returned
    /// if the anime ID does not exist.
    async fn get_by_id(client: &reqwest::Client, id: u32) -> Result<Option<Anime>> {
        tracing::debug!(series_id = %id, "sending `search_by_id` request");

        request::send::<MediaItem>(
            client,
            include_from_root!("graphql/anilist/get_by_id.gql"),
            &json!({ "id": id }),
        )
        .await
        .pipe(|res| match res {
            Ok(m) => Ok(Some(m.media.into())),
            Err(err) if err.request_failed_with_status(404) => Ok(None),
            Err(err) => Err(err),
        })
        .tap(|r| {
            if !tracing::enabled!(target: "request", tracing::Level::DEBUG) {
                return;
            }

            match r {
                Ok(anime) => tracing::debug!(
                    target: "request",
                    series_id = ?anime.as_ref().map(|a: &Anime| a.id),
                    "`search_by_id` request finished successfully"
                ),
                Err(err) => tracing::debug!(
                    target: "request",
                    series_id = %id,
                    ?err,
                    "`search_by_id` request finished unsuccessfully"
                ),
            }
        })
    }
}

#[derive(Deserialize)]
struct MediaItem {
    #[serde(rename = "Media")]
    media: AnimeInfo,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnimeInfo {
    id: u32,
    title: Title,
    episodes: Option<u32>,
    next_airing_episode: Option<NextAiringEpisode>,
}

impl From<AnimeInfo> for crate::Anime {
    fn from(value: AnimeInfo) -> Self {
        let episodes = value.episodes.or_else(|| {
            value.next_airing_episode.map(|n| {
                // the query contains the *next* airing episode,
                // so subtract one to match the current count
                n.episode.saturating_sub(1)
            })
        });

        Self {
            id: MediaID {
                ani_list: Some(value.id),
            },
            title: value.title,
            episodes,
        }
    }
}

#[derive(Debug, Deserialize)]
struct NextAiringEpisode {
    episode: u32,
}
