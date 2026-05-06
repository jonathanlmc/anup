use std::{borrow::Cow, path::PathBuf, sync::Arc};

use anyhow::{Context, anyhow};
use futures::StreamExt;

use crate::{
    series,
    tui::{
        event::{self, AppEvent},
        state::{self, series::EntryState},
    },
};

pub async fn resolve_all(event_chan: event::EventSender, dir: PathBuf) -> anyhow::Result<()> {
    const MAX_BUFFERED_DETECTIONS: usize = 10;
    const MAX_CONCURRENT_RESOLVES: usize = 3;

    let mut dir_iter = tokio::fs::read_dir(&dir)
        .await
        .context("failed to scan series directory")?;

    let mut detection_semaphore = Arc::new(tokio::sync::Semaphore::new(MAX_BUFFERED_DETECTIONS));
    let mut resolve_semaphore = Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_RESOLVES));
    let mut task_set = tokio::task::JoinSet::new();

    loop {
        // limit the number of folder detections that can be buffered before a task to resolve a folder executes
        let detection_permit = detection_semaphore.clone().acquire_owned().await?;

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

        let Some(filename) = entry_path
            .file_name()
            .and_then(|f| f.to_str())
            .map(ToOwned::to_owned)
        else {
            tracing::warn!(path = %entry_path.display(), "entry in series directory does not have file name");
            continue;
        };

        let inserted_series_index = event_chan
            .send_with_reply(|reply_tx| {
                AppEvent::Series(event::series::Payload::Create {
                    state: EntryState::Detected,
                    stable_index_reply: reply_tx,
                })
            })
            .await
            .context("application event channel was closed")?;

        let resolve_sema_clone = resolve_semaphore.clone();
        let event_chan = event_chan.clone();

        // resolve the folder to a series
        task_set.spawn(async move {
            // limit the number of concurrent resolves
            let Ok(_permit) = resolve_sema_clone.acquire().await else {
                return;
            };

            // allow another folder to be buffered
            drop(detection_permit);

            let res = resolve_new_series(
                event_chan,
                entry_path.clone(),
                filename,
                inserted_series_index,
            )
            .await;

            if let Err(err) = res {
                tracing::warn!(path = %entry_path.display(), "failed to resolve series: {}", err);
            }
        });
    }

    while let Some(res) = task_set.join_next().await {
        if let Err(err) = res {
            if err.is_panic() {
                tracing::warn!("failed to resolve series: {:?}", err.into_panic());
            } else {
                continue;
            }
        }
    }

    Ok(())
}

async fn resolve_new_series(
    event_chan: event::EventSender,
    path: PathBuf,
    filename: String,
    inserted_series_index: usize,
) -> anyhow::Result<()> {
    event_chan
        .send(AppEvent::Series(event::series::Payload::Update {
            stable_index: inserted_series_index,
            state: EntryState::Scanning,
        }))
        .await?;

    let local_series = 'blk: {
        let res = tokio::task::spawn_blocking(move || series::LocalRoot::parse_dir(path)).await;

        let entry_error = match res {
            Ok(Ok(series)) => break 'blk series,
            Ok(Err(err)) => err.into(),
            Err(err) if err.is_panic() => {
                let err = err.into_panic();

                let msg = if let Some(msg) = err.downcast_ref::<String>().cloned() {
                    Cow::Owned(msg)
                } else if let Some(&msg) = err.downcast_ref::<&str>() {
                    msg.into()
                } else {
                    "unknown".into()
                };

                state::series::EntryError::Panic(msg)
            }
            Err(err) => {
                tracing::trace!(%filename, "received cancel signal for series resolve");
                return Ok(());
            }
        };

        let failure_msg = format!("{entry_error}");

        event_chan
            .send(AppEvent::Series(event::series::Payload::Update {
                stable_index: inserted_series_index,
                state: EntryState::Failure {
                    name: filename,
                    error: entry_error,
                },
            }))
            .await?;

        anyhow::bail!(failure_msg);
    };

    event_chan
        .send(AppEvent::Series(event::series::Payload::Update {
            stable_index: inserted_series_index,
            state: EntryState::Resolving(local_series.parsed_name.clone()),
        }))
        .await?;

    let local_name = local_series.parsed_name.clone();

    // todo: make api configurable
    let series_result =
        series::automatch::all_formats_and_seasons(local_series, &anime::api::AniList).await;

    let series = match series_result {
        Ok(series::AutomatchResult::Paired(series)) => series,
        Ok(series::AutomatchResult::Unpaired(local_series)) => {
            event_chan
                .send(AppEvent::Series(event::series::Payload::Update {
                    stable_index: inserted_series_index,
                    state: EntryState::Unresolved(local_series),
                }))
                .await?;

            return Ok(());
        }
        Err(err) => {
            let err = err.into();
            let failure_msg = format!("{err}");

            event_chan
                .send(AppEvent::Series(event::series::Payload::Update {
                    stable_index: inserted_series_index,
                    state: EntryState::Failure {
                        name: local_name,
                        error: err,
                    },
                }))
                .await
                .ok();

            anyhow::bail!(failure_msg);
        }
    };

    event_chan
        .send(AppEvent::Series(event::series::Payload::Update {
            stable_index: inserted_series_index,
            state: EntryState::Resolved(series),
        }))
        .await
        .ok();

    Ok(())
}
