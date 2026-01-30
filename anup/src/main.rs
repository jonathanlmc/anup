use anime::api::Service;
use anyhow::Context;

fn main() -> anyhow::Result<()> {
    // avoid `#[tokio::main]` to avoid yet another dependency on `syn` and `quote`
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(main_async())
}

async fn main_async() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    let mut args = std::env::args().skip(1);

    let dir = args.next().context("no path provided")?;
    let series_id = args
        .next()
        .context("no series id provided")?
        .parse()
        .context("invalid series id")?;

    match anime_detect::TopLevelSeries::parse_dir(dir.into()) {
        Ok(series) => println!("{series:#?}"),
        Err(err) => eprintln!("failed: {err:?}"),
    }

    let client = reqwest::Client::new();

    let media: anime::Anime = anime::api::AniList::get_by_id(&client, series_id)
        .await?
        .context("unknown anime id provided")?;

    println!("anilist media:\n{media:#?}");

    Ok(())
}
