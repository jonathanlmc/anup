//! Request helper for the `AniList` GraphQL API.

use serde::{Deserialize, de::DeserializeOwned};
use serde_json::json;

use crate::api::{Error, RequestError, Result};

#[cfg(feature = "rate_limit")]
use super::rate_limit;

/// Base URL for the `AniList` GraphQL endpoint.
pub const BASE_URL: &str = "https://graphql.anilist.co";

/// Send a GraphQL request to the `AniList` API and deserialize the response
/// to the given `T`.
///
/// For high-level API queries, the [`AniList`](crate::api::anilist::AniList)
/// struct with its [`Service`] trait implementation
/// should generally be used instead.
///
/// This function is only intended to be used for any API functionality
/// not already covered by the high-level API in the [`Service`] trait.
///
/// [`Service`]: crate::api::Service
pub async fn send<T: DeserializeOwned>(
    client: &reqwest::Client,
    query: &str,
    variables: &serde_json::Value,
) -> Result<T> {
    // minimize the amount of monomorphisation
    let resp = send_request_impl(client, query, variables).await?;
    serde_json::from_value(resp).map_err(Error::InvalidResponseData)
}

async fn send_request_impl(
    client: &reqwest::Client,
    query: &str,
    variables: &serde_json::Value,
) -> Result<serde_json::Value> {
    #[derive(Debug, Deserialize)]
    struct Response {
        data: serde_json::Value,
        #[serde(default)]
        errors: Vec<RequestError>,
    }

    #[cfg(feature = "rate_limit")]
    rate_limit::acquire_permit().await;

    let body = json!({
        "query": query,
        "variables": variables
    });

    tracing::trace!(%query, %variables, "sending request to anilist");

    let resp = client
        .post(BASE_URL)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header(reqwest::header::ACCEPT, "application/json")
        .json(&body)
        .send()
        .await?;

    let status = resp.status();
    let mut resp_json: Response = resp.json().await?;

    tracing::trace!(%status, json = ?resp_json, "received response from anilist");

    if !resp_json.errors.is_empty() {
        let error = resp_json
            .errors
            // safety: length checked above
            .swap_remove(0);

        return Err(Error::RequestFailed(error));
    }

    Ok(resp_json.data)
}
