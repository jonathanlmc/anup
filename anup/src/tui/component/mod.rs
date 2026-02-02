pub mod series_list;

pub use series_list::SeriesList;

use crate::tui::AppState;

pub trait Component {
    fn render(&mut self, frame: &mut ratatui::Frame, state: &AppState);
}
