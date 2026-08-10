mod image_cache;
mod series;
mod tui;
mod util;

use std::{
    collections::VecDeque,
    num::NonZeroUsize,
    sync::{Arc, LazyLock},
};

use anyhow::Context;
use tap::TapOptional;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{image_cache::ImageCache, tui::state};

static REQWEST_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    const DEFAULT_LOG_LEVEL: LevelFilter = if cfg!(debug_assertions) {
        LevelFilter::DEBUG
    } else {
        LevelFilter::WARN
    };

    // the `EnvFilter` layer for `tracing_subscriber` can do this for us but
    // it pulls in the large `regex-automata` crate which is overkill for our
    // purposes, so parse the env manually
    let tracing_target = std::env::var("RUST_LOG")
        .ok()
        .and_then(|val| val.parse::<tracing_subscriber::filter::Targets>().ok())
        .unwrap_or_else(|| {
            tracing_subscriber::filter::Targets::new().with_default(DEFAULT_LOG_LEVEL)
        });

    let (log_tx, log_rx) = tokio::sync::mpsc::channel(64);
    let log_transmitter = tui::LogTransmitter::new(log_tx);

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_writer(move || log_transmitter.clone()))
        .with(tracing_target)
        .init();

    let path = std::env::args().nth(1).context("missing path arg")?;

    let cache_dir = dirs::cache_dir().tap_some_mut(|dir| {
        dir.push(env!("CARGO_PKG_NAME"));
        dir.push("img");
    });

    if let Some(dir) = &cache_dir
        && !tokio::fs::try_exists(dir).await.unwrap_or(false)
    {
        tokio::fs::create_dir_all(dir).await?;
    }

    tui::App::init(
        tui::State {
            series_scan_dir: path.into(),
            series_list: state::SeriesList::new(),
            log_message_buffer: VecDeque::with_capacity(tui::MAX_LOG_MESSAGES),
            image_cache: Arc::new(ImageCache::new(
                cache_dir,
                // todo: make configurable
                // safety: 10 > 0
                NonZeroUsize::new(10).unwrap(),
                // todo: make configurable
                10 * 1024 * 1024, // 10 MiB
            )),
        },
        tui::panel::Stack::new(Box::new(tui::panel::Main::new())),
        log_rx,
    )?
    .run()
    .await;

    Ok(())
}
