//! Common functionality for all supported anime tracking services.

pub mod anilist;

use serde::Deserialize;

pub use anilist::AniList;

use crate::{Id, Info, PlainId};

pub type Result<T> = std::result::Result<T, Error>;

/// Unified error type for all API interactions.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Underlying HTTP error while sending an API request.
    #[error("internal http error while sending request: {0}")]
    HttpError(
        #[from]
        #[source]
        reqwest::Error,
    ),
    /// The API responded with an error status and message.
    #[error(
        "request failed with status `{}`: {}",
        .0.status,
        .0.message.as_deref().unwrap_or("unknown error")
    )]
    RequestFailed(RequestError),
    /// Failed to parse the response JSON from an API call.
    #[error("failed to parse request response: {0}")]
    InvalidResponseData(serde_json::Error),
}

impl Error {
    /// Returns `true` if this error was a request failure with the given status.
    #[inline]
    #[must_use]
    pub const fn request_failed_with_status(&self, status: u16) -> bool {
        matches!(
            self,
            Self::RequestFailed(RequestError { status: err_status, .. })
                if *err_status == status
        )
    }
}

/// A failure response from a [`Service`] API call.
#[serde_with::serde_as]
#[derive(Clone, Debug, Deserialize)]
pub struct RequestError {
    /// Human‑readable error message, if any.
    #[serde_with(as = "NoneAsEmptyString")]
    pub message: Option<String>,
    /// HTTP status code associated with the error.
    pub status: u16,
}

/// High-level API calls for all supported anime tracking services.
pub trait Service {
    /// Get an anime by its ID. [`None`] will be returned
    /// if the anime ID does not exist on the service.
    fn get_by_id(
        &self,
        client: &reqwest::Client,
        id: PlainId,
    ) -> impl Future<Output = Result<Option<Info>>> + Send;

    /// Search for an anime by a partial name. An iterator with all matching entries
    /// will be returned on success.
    fn search_by_name(
        &self,
        client: &reqwest::Client,
        partial_name: &str,
    ) -> impl Future<Output = Result<impl Iterator<Item = Info>>> + Send;

    fn sequel_id(
        &self,
        client: &reqwest::Client,
        id: PlainId,
    ) -> impl Future<Output = Result<Option<Id>>> + Send;
}
