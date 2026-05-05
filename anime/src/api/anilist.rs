//! Integration with the AniList GraphQL API.

#[cfg(feature = "rate_limit")]
pub mod rate_limit;

pub mod request;

use serde::Deserialize;
use serde_json::json;
use tap::{Pipe, Tap};

use crate::{
    MediaID, Title,
    api::{AnimeID, AnimeInfo, Result, Service},
    macros::include_graphql,
};

/// AniList API integration.
pub struct AniList;

impl Service for AniList {
    type AnimeData = AnimeEntry;

    async fn get_by_id(
        &self,
        client: &reqwest::Client,
        id: AnimeID,
    ) -> Result<Option<Self::AnimeData>> {
        tracing::debug!(series_id = %id, "sending `get_by_id` request");

        request::send::<MediaItem<AnimeEntry>>(
            client,
            include_graphql!("anilist/get_by_id.gql"),
            &json!({ "id": id }),
        )
        .await
        .pipe(|res| match res {
            Ok(m) => Ok(Some(m.media)),
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
                    series_id = ?anime.as_ref().map(|a| a.id),
                    "`get_by_id` request finished successfully"
                ),
                Err(err) => tracing::debug!(
                    target: "request",
                    series_id = %id,
                    ?err,
                    "`get_by_id` request finished unsuccessfully"
                ),
            }
        })
    }

    async fn search_by_name(
        &self,
        client: &reqwest::Client,
        partial_name: &str,
    ) -> Result<impl Iterator<Item = Self::AnimeData>> {
        tracing::debug!(%partial_name, "sending `search_by_name` request");

        request::send::<PagedResponse<PagedResponseMediaItems>>(
            client,
            include_graphql!("anilist/search_by_name.gql"),
            &json!({ "search": partial_name }),
        )
        .await
        .map(|r| r.page.media.into_iter())
        .tap(|r| {
            if !tracing::enabled!(target: "request", tracing::Level::DEBUG) {
                return;
            }

            match r {
                Ok(items) => tracing::debug!(
                    target: "request",
                    %partial_name,
                    found_items = %items.len(),
                    "`search_by_name` request finished successfully"
                ),
                Err(err) => tracing::debug!(
                    target: "request",
                    %partial_name,
                    ?err,
                    "`search_by_name` request finished unsuccessfully"
                ),
            }
        })
    }

    async fn sequel_id(&self, client: &reqwest::Client, id: AnimeID) -> Result<Option<AnimeID>> {
        tracing::debug!(%id, "sending `sequel_id` request");

        request::send::<MediaItem<MediaRelations>>(
            client,
            include_graphql!("anilist/relations.gql"),
            &json!({ "id": id }),
        )
        .await
        .map(|r| {
            r.media.relations.edges.into_iter().find_map(|edge| {
                (edge.relation_type == RelationType::Sequel).then_some(edge.node.id)
            })
        })
        .tap(|r| {
            if !tracing::enabled!(target: "request", tracing::Level::DEBUG) {
                return;
            }

            match r {
                Ok(sequel_id) => tracing::debug!(
                    target: "request",
                    %id,
                    ?sequel_id,
                    "`sequel_id` request finished successfully"
                ),
                Err(err) => tracing::debug!(
                    target: "request",
                    %id,
                    ?err,
                    "`sequel_id` request finished unsuccessfully"
                ),
            }
        })
    }
}

#[derive(Deserialize)]
struct MediaItem<T> {
    #[serde(rename = "Media")]
    media: T,
}

#[derive(Deserialize)]
struct MediaRelations {
    relations: MediaRelationEdges,
}

#[derive(Deserialize)]
struct MediaRelationEdges {
    edges: Vec<MediaRelation>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MediaRelation {
    relation_type: RelationType,
    node: MediaRelationNode,
}

#[derive(Deserialize)]
struct MediaRelationNode {
    id: AnimeID,
}

#[derive(Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
enum RelationType {
    Sequel,
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
struct PagedResponse<T> {
    #[serde(rename = "Page")]
    page: T,
}

#[derive(Deserialize)]
struct PagedResponseMediaItems {
    media: Vec<AnimeEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeEntry {
    pub id: AnimeID,
    pub title: Title,
    pub episodes: Option<u32>,
    pub next_airing_episode: Option<NextAiringEpisode>,
    pub format: Option<SeriesFormat>,
}

impl From<AnimeEntry> for crate::Anime {
    fn from(value: AnimeEntry) -> Self {
        let episodes = value.episodes.or_else(|| {
            value.next_airing_episode.map(|n| {
                // the query contains the *next* airing episode,
                // so subtract one to match the current count
                n.episode.saturating_sub(1)
            })
        });

        Self {
            id: MediaID {
                anilist: Some(value.id),
            },
            title: value.title,
            episodes,
            format: value.format.map(Into::into),
        }
    }
}

impl AnimeInfo for AnimeEntry {
    #[inline]
    fn id(&self) -> AnimeID {
        self.id
    }
}

#[derive(Debug, Deserialize)]
pub struct NextAiringEpisode {
    pub episode: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum SeriesFormat {
    Tv,
    Movie,
    Special,
    Ona,
    Ova,
    Music,
    #[serde(other)]
    Other,
}

impl From<SeriesFormat> for crate::Format {
    fn from(value: SeriesFormat) -> Self {
        match value {
            SeriesFormat::Tv => Self::Tv,
            SeriesFormat::Movie => Self::Movie,
            SeriesFormat::Special => Self::Special,
            SeriesFormat::Ona => Self::Ona,
            SeriesFormat::Ova => Self::Ova,
            SeriesFormat::Music => Self::Music,
            SeriesFormat::Other => Self::Other,
        }
    }
}
