use ratatui::{
    style::{Color, Style},
    widgets::{Block, List, ListItem, ListState},
};

use crate::tui::{AppState, component::Component, state};

pub struct SeriesList {
    state: ListState,
}

impl SeriesList {
    pub fn new() -> Self {
        Self {
            state: ListState::default(),
        }
    }
}

impl Component for SeriesList {
    fn render(&mut self, frame: &mut ratatui::Frame, state: &AppState) {
        let list_items = state.series.iter().map(|series| {
            use state::series::EntryState;

            let mut item = ListItem::new(series.name());

            match series.state {
                EntryState::Resolved(_) => (),
                EntryState::Resolving(_) => item = item.style(Style::default().fg(Color::Green)),
                EntryState::Detected(_) => item = item.style(Style::default().fg(Color::DarkGray)),
                EntryState::Unmatched { .. } => {
                    item = item.style(Style::default().fg(Color::DarkGray).italic())
                }
                EntryState::Failure { .. } => {
                    item = item.style(Style::default().fg(Color::Red).italic())
                }
            }

            item
        });

        let list = List::new(list_items).block(Block::bordered().title("Series"));

        frame.render_stateful_widget(list, frame.area(), &mut self.state);
    }
}
