// temporary
#![allow(unused)]

mod series;
mod tui;

use std::sync::LazyLock;

use anyhow::Context;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::tui::state;

static REQWEST_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    // the `EnvFilter` layer for `tracing_subscriber` can do this for us but
    // it pulls in the large `regex-automata` crate which is overkill for our
    // purposes, so parse the env manually
    let tracing_target = std::env::var("RUST_LOG")
        .ok()
        .and_then(|val| val.parse::<tracing_subscriber::filter::Targets>().ok())
        .unwrap_or(tracing_subscriber::filter::Targets::new().with_default(LevelFilter::ERROR));

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .with(tracing_target)
        .init();

    let path = std::env::args().nth(1).context("missing path arg")?;

    tui::App::init(tui::AppState {
        series_scan_dir: path.into(),
        series: state::series::List::new(),
    })?
    .run()
    .await;

    Ok(())
}
