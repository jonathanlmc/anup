fn main() {
    tracing_subscriber::fmt().init();

    let dir = std::env::args().nth(1).expect("no path provided");

    match anime_detect::Series::parse_dir(dir.into()) {
        Ok(series) => println!("{series:#?}"),
        Err(err) => eprintln!("failed: {err:?}"),
    }
}
