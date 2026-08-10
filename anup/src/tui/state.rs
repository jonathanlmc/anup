use std::{collections::VecDeque, path::PathBuf, sync::Arc};

pub mod series_list;

pub use series_list::SeriesList;

use crate::image_cache::ImageCache;

#[derive(Debug)]
pub struct State {
    pub series_scan_dir: PathBuf,
    pub series_list: SeriesList,
    pub log_message_buffer: VecDeque<String>,
    pub image_cache: Arc<ImageCache>,
}

#[derive(Debug)]
pub struct Info {
    pub size: ratatui::layout::Size,
    pub image_protocol_picker: Arc<ratatui_image::picker::Picker>,
}
