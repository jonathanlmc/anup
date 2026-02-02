// temporary
#![allow(unused)]

mod series;
mod tui;

use std::sync::LazyLock;

use anyhow::Context;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::tui::state;

static REQWEST_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
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
