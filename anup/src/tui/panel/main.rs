use crossterm::event::{KeyCode, KeyEvent};
use ratatui::widgets::Block;

use crate::tui::{
    self,
    widget::{SeriesList, series_list},
};

#[derive(Debug)]
pub struct Main {
    series_list_state: series_list::State,
}

impl Main {
    pub fn new() -> Self {
        Self {
            series_list_state: series_list::State::new(),
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
                self.series_list_state.select_top_level_series();
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
        let series_tree =
            SeriesList::new(&state.series_list).block(Block::bordered().title("Series List"));

        frame.render_stateful_widget(series_tree, frame.area(), &mut self.series_list_state);
    }
}
