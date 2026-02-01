// temporary
#![allow(unused)]

mod series;

use std::sync::LazyLock;

use anyhow::Context;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

static REQWEST_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);

fn main() -> anyhow::Result<()> {
    // avoid `#[tokio::main]` to avoid yet another dependency on `syn` and `quote`
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(main_async())
}

async fn main_async() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let mut args = std::env::args().skip(1);

    let dir = args.next().context("no path provided")?;

    let local_series = match anime_detect::TopLevelSeries::parse_dir(dir.into()) {
        Ok(series) => {
            println!("parsed series name: {}", series.parsed_name);
            series
        }
        Err(err) => anyhow::bail!("failed: {err:?}"),
    };

    let consolidated_series =
        match series::automatch::all_formats_and_seasons(local_series, &anime::api::AniList).await?
        {
            series::AutomatchResult::Complete(series) => series,
            series::AutomatchResult::Partial { series, .. } => {
                eprintln!("only obtained partial match for series");
                series
            }
            series::AutomatchResult::None(others) => {
                eprintln!("no confident match; other similar series:");

                for series in others {
                    eprintln!("{}", series.title.romaji);
                }

                anyhow::bail!("failed to detect series");
            }
        };

    println!("consolidated series: {consolidated_series:#?}");

    Ok(())
}
