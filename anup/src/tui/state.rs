use std::{collections::VecDeque, fmt::Debug, path::PathBuf, sync::Arc};

pub mod series_list;

pub use series_list::SeriesList;

use crate::image_cache::ImageCache;

pub struct State {
    pub series_scan_dir: PathBuf,
    pub series_list: SeriesList,
    pub anime_service: Arc<dyn anime::api::Service>,
    pub anime_service_user_auth: Arc<Option<anime::api::AuthToken>>,
    pub log_message_buffer: VecDeque<String>,
    pub image_cache: Arc<ImageCache>,
}

impl Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State")
            .field("series_scan_dir", &self.series_scan_dir)
            .field("series_list", &self.series_list)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub struct Info {
    pub size: ratatui::layout::Size,
    pub image_protocol_picker: Arc<ratatui_image::picker::Picker>,
}
