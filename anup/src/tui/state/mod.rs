use std::path::PathBuf;

pub mod series;

#[derive(Debug)]
pub struct AppState {
    pub series_scan_dir: PathBuf,
    pub series: series::List,
}
