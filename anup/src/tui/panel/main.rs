use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::Block;

use crate::tui::{
    self,
    panel::Panel,
    widget::series_list::{self, SeriesList},
};

pub struct MainPanel {
    series_list_state: series_list::State,
}

impl MainPanel {
    pub fn new() -> Self {
        Self {
            series_list_state: series_list::State::new(),
        }
    }
}

impl Panel for MainPanel {
    fn process_input(
        &mut self,
        event: KeyEvent,
        _state: &mut tui::AppState,
        render_trigger: &tui::RenderTrigger,
    ) -> tui::event::Result {
        if !event.is_press() {
            return tui::event::Result::Continue;
        }

        match event.code {
            KeyCode::Char('Q') if event.modifiers.contains(KeyModifiers::SHIFT) => {
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
            _ => (),
        }

        render_trigger.notify_one();
        tui::event::Result::Continue
    }

    fn render(&mut self, frame: &mut ratatui::Frame, state: &tui::AppState) {
        let series_tree =
            SeriesList::new(&state.series_list).block(Block::bordered().title("Series List"));

        frame.render_stateful_widget(series_tree, frame.area(), &mut self.series_list_state);
    }
}
