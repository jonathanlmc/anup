use crossterm::event::KeyCode;
use ratatui::widgets::{Block, Scrollbar, ScrollbarOrientation, ScrollbarState};

use crate::tui;

const SCROLL_AMOUNT: usize = 10;

#[derive(Debug)]
pub struct Log {
    scroll_offset: Option<usize>,
}

impl Log {
    pub fn new() -> Self {
        Self {
            scroll_offset: None,
        }
    }
}

impl tui::Panel for Log {
    fn process_input(
        &mut self,
        event: crossterm::event::KeyEvent,
        info: &tui::state::Info,
        state: &mut tui::State,
        _render_trigger: &tui::RenderTrigger,
    ) -> tui::event::Result {
        if !event.is_press() {
            return tui::event::Result::Continue(None);
        }

        match event.code {
            KeyCode::Esc => {
                return tui::event::Result::Continue(Some(tui::AppEvent::PopPanel));
            }
            KeyCode::PageUp | KeyCode::Up => {
                let offset = self
                    .scroll_offset
                    .get_or_insert(state.log_message_buffer.len());

                *offset = offset
                    .saturating_sub(SCROLL_AMOUNT)
                    .max(info.size.height as usize);
            }
            KeyCode::PageDown | KeyCode::Down => {
                let new_offset =
                    self.scroll_offset.unwrap_or(state.log_message_buffer.len()) + SCROLL_AMOUNT;

                self.scroll_offset = if new_offset >= state.log_message_buffer.len() {
                    None
                } else {
                    Some(new_offset)
                };
            }
            KeyCode::Home => self.scroll_offset = Some(info.size.height as usize),
            KeyCode::End => self.scroll_offset = None,
            _ => (),
        }

        tui::event::Result::Continue(None)
    }

    fn process_event_notification(
        &mut self,
        notification: tui::AppEventNotification,
        info: &tui::state::Info,
        state: &tui::State,
        render_trigger: &tui::RenderTrigger,
    ) {
        match notification {
            tui::AppEventNotification::LogMessage => {
                if let Some(offset) = self.scroll_offset.as_mut()
                    && state.log_message_buffer.len() == tui::MAX_LOG_MESSAGES
                {
                    *offset = offset.saturating_sub(1).max(info.size.height as usize);
                }

                render_trigger.notify_one();
            }
        }
    }

    fn render(&mut self, frame: &mut ratatui::Frame, state: &tui::State) {
        let block = Block::bordered().title("Log");
        let area = block.inner(frame.area());

        let scroll_offset = self.scroll_offset.unwrap_or(state.log_message_buffer.len());

        let log = tui::widget::Log::new(&state.log_message_buffer).scroll(scroll_offset);

        let mut scrollbar_state = ScrollbarState::new(
            state
                .log_message_buffer
                .len()
                .saturating_sub(area.height as usize),
        )
        .position(scroll_offset.saturating_sub(area.height as usize));

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);

        frame.render_widget(block, frame.area());
        frame.render_widget(log, area);
        frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}
