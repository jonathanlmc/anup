// temporary
#![allow(unused)]

mod series;
mod tui;

use std::sync::LazyLock;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

static REQWEST_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tui::App::init(tui::AppState {})?.run().await;

    Ok(())
}
