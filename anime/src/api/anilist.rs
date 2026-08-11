//! Integration with the `AniList` GraphQL API.

#[cfg(feature = "rate_limit")]
pub mod rate_limit;

pub mod request;

use serde::Deserialize;
use serde_json::json;
use tap::{Pipe, Tap};

use crate::{
    CoverImage, Id, Info, PlainId, Title,
    api::{Result, Service},
    macros::include_graphql,
};

/// `AniList` API integration.
pub struct AniList;

impl Service for AniList {
    async fn get_by_id(&self, client: &reqwest::Client, id: PlainId) -> Result<Option<Info>> {
        tracing::debug!(series_id = %id, "sending `get_by_id` request");

        request::send::<MediaItem<AnimeEntry>>(
            client,
            include_graphql!("anilist/get_by_id.gql"),
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
                    series_id = ?anime.as_ref().map(|i: &Info| i.id),
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
    ) -> Result<impl Iterator<Item = Info>> {
        tracing::debug!(%partial_name, "sending `search_by_name` request");

        request::send::<PagedResponse<PagedResponseMediaItems>>(
            client,
            include_graphql!("anilist/search_by_name.gql"),
            &json!({ "search": partial_name }),
        )
        .await
        .map(|r| r.page.media.into_iter().map(Into::into))
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

    async fn sequel_id(&self, client: &reqwest::Client, id: PlainId) -> Result<Option<Id>> {
        tracing::debug!(%id, "sending `sequel_id` request");

        request::send::<MediaItem<MediaRelations>>(
            client,
            include_graphql!("anilist/relations.gql"),
            &json!({ "id": id }),
        )
        .await
        .map(|r| {
            r.media.relations.edges.into_iter().find_map(|edge| {
                (edge.relation_type == RelationType::Sequel)
                    .then_some(edge.node.id)
                    .map(Id::AniList)
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
    id: PlainId,
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
struct AnimeEntry {
    id: PlainId,
    title: Title,
    cover_image: CoverImage,
    episodes: Option<u32>,
    next_airing_episode: Option<NextAiringEpisode>,
    format: Option<SeriesFormat>,
}

impl From<AnimeEntry> for crate::Info {
    fn from(value: AnimeEntry) -> Self {
        let episodes = value.episodes.or_else(|| {
            value.next_airing_episode.map(|n| {
                // the query contains the *next* airing episode,
                // so subtract one to match the current count
                n.episode.saturating_sub(1)
            })
        });

        Self {
            id: Id::AniList(value.id),
            title: value.title,
            cover_image_url: value.cover_image,
            episodes,
            format: value.format.map(Into::into),
        }
    }
}

#[derive(Debug, Deserialize)]
struct NextAiringEpisode {
    episode: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
enum SeriesFormat {
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
