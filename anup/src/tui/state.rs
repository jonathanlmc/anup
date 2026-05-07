use std::{collections::VecDeque, path::PathBuf};

pub mod series_list;

pub use series_list::SeriesList;

#[derive(Debug)]
pub struct State {
    pub series_scan_dir: PathBuf,
    pub series_list: SeriesList,
    pub log_message_buffer: VecDeque<String>,
}

#[derive(Debug)]
pub struct Info {
    pub size: ratatui::layout::Size,
}
