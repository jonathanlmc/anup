use std::path::PathBuf;

use anyhow::{Context, anyhow};
use futures::StreamExt;

use crate::{
    series,
    tui::{
        event::{self, AppEvent},
        state,
    },
};

pub async fn scan_and_resolve_all_in_dir(event_chan: event::EventSender, dir: PathBuf) {
    // borrowing here avoids the need to clone in async blocks
    let event_chan = &event_chan;

    let series_candidates = match add_series_dir_candidates(dir.clone(), event_chan).await {
        Ok(candidates) => candidates,
        Err(err) => {
            tracing::error!(path = %dir.display(), ?err, "failed to scan for series candidates in series directory");
            return;
        }
    };

    if series_candidates.is_empty() {
        return;
    }

    let parse_series_iter = series_candidates.into_iter().map(|candidate| async move {
        let local_series = parse_local_series_with_state(candidate.path, candidate.filename).await;

        event_chan
            .send(AppEvent::Series(event::series::Payload::Update {
                stable_index: candidate.stable_index,
                state: local_series.state,
            }))
            .await
            .ok();

        (local_series.series, candidate.stable_index)
    });

    futures::stream::iter(parse_series_iter)
        .buffered(50)
        .for_each_concurrent(2, |(series, stable_index)| async move {
            if let Some(series) = series {
                automatch_series_and_update_state(series, stable_index, event_chan).await
            }
        })
        .await;
}

struct SeriesCandidate {
    stable_index: usize,
    path: PathBuf,
    filename: String,
}

async fn add_series_dir_candidates(
    dir: PathBuf,
    event_chan: &event::EventSender,
) -> anyhow::Result<Vec<SeriesCandidate>> {
    let mut dir_iter = tokio::fs::read_dir(&dir)
        .await
        .context("failed to scan series directory")?;

    let mut candidates = Vec::new();

    loop {
        let entry = match dir_iter.next_entry().await {
            Ok(Some(entry)) => entry,
            Ok(None) => break,
            Err(err) => {
                tracing::warn!(path = %dir.display(), "failed to read entry in series directory: {err}");
                continue;
            }
        };

        let file_type = match entry.file_type().await {
            Ok(kind) => kind,
            Err(err) => {
                tracing::warn!(path = %entry.path().display(), "failed to read file type for entry in series directory: {err}");
                continue;
            }
        };

        if !file_type.is_dir() {
            continue;
        }

        let entry_path = entry.path();

        let Some(filename) = entry_path.file_name() else {
            tracing::warn!(path = %entry_path.display(), "entry in series directory does not have file name");
            continue;
        };

        let filename = filename.to_string_lossy();
        let filename = filename.into_owned();

        let stable_index = event_chan
            .send_with_reply(|reply_tx| {
                AppEvent::Series(event::series::Payload::Create {
                    state: state::series::EntryState::Detected("Scanning..".into()),
                    stable_index_reply: reply_tx,
                })
            })
            .await
            .context("application event channel was closed")?;

        candidates.push(SeriesCandidate {
            stable_index,
            path: entry_path,
            filename,
        });
    }

    Ok(candidates)
}

struct LocalSeriesResult {
    series: Option<anime_detect::TopLevelSeries>,
    state: state::series::EntryState,
}

async fn parse_local_series_with_state(entry_path: PathBuf, filename: String) -> LocalSeriesResult {
    let entry_path_clone = entry_path.clone();

    let result = tokio::task::spawn_blocking(move || {
        anime_detect::TopLevelSeries::parse_dir(entry_path_clone)
    })
    .await;

    match result {
        Ok(Ok(series)) => {
            let state = state::series::EntryState::Detected(series.parsed_name.clone().into());

            LocalSeriesResult {
                series: Some(series),
                state,
            }
        }
        Ok(Err(err)) => {
            tracing::warn!(path = %entry_path.display(), "failed to parse series in series directory: {err}");

            LocalSeriesResult {
                series: None,
                state: state::series::EntryState::Failure {
                    name: filename,
                    error: anyhow!("failed to parse series in series directory: {err}"),
                },
            }
        }
        Err(err) => {
            let error = if err.is_panic() {
                tracing::error!(path = %entry_path.display(), "panic occurred while parsing series in series directory: {err}");
                anyhow!("panic occurred while parsing series in series directory: {err}")
            } else {
                anyhow!("series parsing was cancelled: {err}")
            };

            LocalSeriesResult {
                series: None,
                state: state::series::EntryState::Failure {
                    name: filename,
                    error,
                },
            }
        }
    }
}

async fn automatch_series_and_update_state(
    local_series: anime_detect::TopLevelSeries,
    stable_index: usize,
    event_chan: &event::EventSender,
) {
    let parsed_local_name = local_series.parsed_name.clone();

    event_chan
        .send(AppEvent::Series(event::series::Payload::Update {
            stable_index,
            state: state::series::EntryState::Resolving(parsed_local_name.clone()),
        }))
        .await
        .ok();

    let series_result =
        series::automatch::all_formats_and_seasons(local_series, &anime::api::AniList).await;

    let series = match series_result {
        Ok(series::AutomatchResult::Matched(series)) => series,
        Ok(series::AutomatchResult::None {
            local_series,
            searched_anime,
        }) => {
            event_chan
                .send(AppEvent::Series(event::series::Payload::Update {
                    stable_index,
                    state: state::series::EntryState::Unmatched {
                        local_series,
                        searched_anime,
                    },
                }))
                .await
                .ok();

            return;
        }
        Err(err) => {
            event_chan
                .send(AppEvent::Series(event::series::Payload::Update {
                    stable_index,
                    state: state::series::EntryState::Failure {
                        name: parsed_local_name,
                        error: anyhow!("failed to resolve series in series directory: {err}"),
                    },
                }))
                .await
                .ok();

            return;
        }
    };

    event_chan
        .send(AppEvent::Series(event::series::Payload::Update {
            stable_index,
            state: state::series::EntryState::Resolved(series),
        }))
        .await
        .ok();
}
