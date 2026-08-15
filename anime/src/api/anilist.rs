//! Integration with the `AniList` GraphQL API.

#[cfg(feature = "rate_limit")]
pub mod rate_limit;

pub mod request;

use std::borrow::Cow;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use serde_with::{TimestampSeconds, serde_as};
use tap::{Pipe, Tap};

use crate::{
    CoverImage, Id, Info, PlainId, Title, UserListEntry, UserStatus,
    api::{AuthToken, Result, Service},
    macros::include_graphql,
};

/// `AniList` API integration.
pub struct AniList {
    pub client: reqwest::Client,
    pub client_id: u32,
}

impl AniList {
    #[inline]
    #[must_use]
    pub const fn new(client: reqwest::Client, client_id: u32) -> Self {
        Self { client, client_id }
    }
}

#[async_trait]
impl Service for AniList {
    async fn get_by_id(&self, id: PlainId, auth: Option<&AuthToken>) -> Result<Option<Info>> {
        tracing::debug!(series_id = %id, "sending `get_by_id` request");

        request::send::<MediaItem<AnimeEntry>>(
            &self.client,
            include_graphql!("anilist/get_by_id.gql"),
            &json!({ "id": id }),
            auth,
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
        partial_name: &str,
        auth: Option<&AuthToken>,
    ) -> Result<Vec<Info>> {
        tracing::debug!(%partial_name, "sending `search_by_name` request");

        request::send::<PagedResponse<PagedResponseMediaItems>>(
            &self.client,
            include_graphql!("anilist/search_by_name.gql"),
            &json!({ "search": partial_name }),
            auth,
        )
        .await
        .map(|r| -> Vec<_> { r.page.media.into_iter().map(Into::into).collect() })
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

    async fn sequel_id(&self, id: PlainId) -> Result<Option<Id>> {
        tracing::debug!(%id, "sending `sequel_id` request");

        request::send::<MediaItem<MediaRelations>>(
            &self.client,
            include_graphql!("anilist/relations.gql"),
            &json!({ "id": id }),
            None,
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

    #[inline]
    fn implicit_grant_oauth_url_str(&self) -> Option<Cow<'static, str>> {
        Some(
            format!(
                "https://anilist.co/api/v2/oauth/authorize?client_id={}&response_type=token",
                self.client_id,
            )
            .into(),
        )
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
    cover_image: CoverImage,
    episodes: Option<u32>,
    format: Option<SeriesFormat>,
    id: PlainId,
    media_list_entry: Option<MediaListEntry>,
    next_airing_episode: Option<NextAiringEpisode>,
    title: Title,
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
            user_list_entry: value.media_list_entry.map(Into::into),
        }
    }
}

#[serde_as]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MediaListEntry {
    completed_at: Option<FuzzyDate>,
    #[serde_as(as = "Option<TimestampSeconds>")]
    created_at: Option<jiff::Timestamp>,
    progress: Option<u32>,
    repeat: Option<u32>,
    score: Option<f32>,
    started_at: Option<FuzzyDate>,
    status: Option<MediaListStatus>,
    #[serde_as(as = "Option<TimestampSeconds>")]
    updated_at: Option<jiff::Timestamp>,
}

impl From<MediaListEntry> for UserListEntry {
    fn from(value: MediaListEntry) -> Self {
        Self {
            completed_at: value.completed_at.and_then(FuzzyDate::into_date),
            created_at: value.created_at,
            progress: value.progress,
            repeat: value.repeat,
            score: value.score,
            started_at: value.started_at.and_then(FuzzyDate::into_date),
            status: value.status.map(Into::into),
            updated_at: value.updated_at,
        }
    }
}

#[derive(Debug, Deserialize)]
struct FuzzyDate {
    day: Option<i8>,
    month: Option<i8>,
    year: Option<i16>,
}

impl FuzzyDate {
    fn into_date(self) -> Option<jiff::civil::Date> {
        jiff::civil::Date::new(self.year?, self.month?, self.day?).ok()
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
enum MediaListStatus {
    Current,
    Planning,
    Completed,
    Dropped,
    Paused,
    Repeating,
}

impl From<MediaListStatus> for UserStatus {
    fn from(value: MediaListStatus) -> Self {
        match value {
            MediaListStatus::Current => Self::Current,
            MediaListStatus::Planning => Self::Planning,
            MediaListStatus::Completed => Self::Completed,
            MediaListStatus::Dropped => Self::Dropped,
            MediaListStatus::Paused => Self::Paused,
            MediaListStatus::Repeating => Self::Repeating,
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
