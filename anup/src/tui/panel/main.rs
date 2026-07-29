use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Layout},
    widgets::Block,
};

use crate::tui::{
    self,
    widget::{
        SeriesList,
        series::{self, SeriesInfo},
    },
};

#[derive(Debug)]
pub struct Main {
    series_list_state: series::list::ViewState,
}

impl Main {
    pub fn new() -> Self {
        Self {
            series_list_state: series::list::ViewState::new(),
        }
    }
}

impl tui::Panel for Main {
    fn process_input(
        &mut self,
        event: KeyEvent,
        _info: &tui::state::Info,
        _state: &mut tui::State,
        render_trigger: &tui::RenderTrigger,
    ) -> tui::event::Result {
        if !event.is_press() {
            return tui::event::Result::Continue(None);
        }

        match event.code {
            KeyCode::Esc => {
                return tui::event::Result::Quit;
            }
            KeyCode::Char('s' | 'S') | KeyCode::Down => {
                self.series_list_state.select_next();
            }
            KeyCode::Char('w' | 'W') | KeyCode::Up => {
                self.series_list_state.select_previous();
            }
            KeyCode::Char('d' | 'D') | KeyCode::Right => {
                self.series_list_state.select_series_formats();
            }
            KeyCode::Char('a' | 'A') | KeyCode::Left => {
                self.series_list_state.select_root_series();
            }
            KeyCode::Char('~') => {
                let panel = Box::new(tui::panel::Log::new());
                let event = tui::AppEvent::PushPanel(panel);
                return tui::event::Result::Continue(Some(event));
            }
            _ => return tui::event::Result::Continue(None),
        }

        render_trigger.notify_one();
        tui::event::Result::Continue(None)
    }

    fn render(&mut self, frame: &mut ratatui::Frame, state: &tui::State) {
        let [left_area, right_area] =
            Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
                .areas(frame.area());

        let frame_data = self.series_list_state.frame_data(&state.series_list);

        let series_list = {
            let title = match frame_data {
                series::list::FrameData::RootSeries { .. } => "Series List",
                series::list::FrameData::SeriesFormats { .. } => "Series Format Selection",
            };

            SeriesList::new(&frame_data).block(Block::bordered().title(title))
        };

        frame.render_widget(series_list, left_area);

        if let series::list::FrameData::RootSeries { selected_entry, .. } = &frame_data {
            let info_panel =
                SeriesInfo::new(*selected_entry).block(Block::bordered().title("Info"));

            frame.render_widget(info_panel, right_area);
        }

        self.series_list_state.apply_frame_data(frame_data);
    }
}
