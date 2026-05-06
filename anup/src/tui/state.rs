use std::path::PathBuf;

pub mod series_list;

pub use series_list::SeriesList;

#[derive(Debug)]
pub struct AppState {
    pub series_scan_dir: PathBuf,
    pub series_list: SeriesList,
}
