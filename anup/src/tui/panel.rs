pub mod log;
pub mod main;

use std::{fmt::Debug, sync::Arc};

use crossterm::event::KeyEvent;

pub use log::Log;
pub use main::Main;

use crate::tui;

pub trait Panel: Debug + Send + Sync {
    fn process_input(
        &mut self,
        _event: KeyEvent,
        _info: &tui::state::Info,
        _state: &mut tui::State,
        _render_trigger: Arc<tui::RenderTrigger>,
    ) -> tui::event::Result {
        tui::event::Result::Continue(None)
    }

    fn process_event_notification(
        &mut self,
        _notification: tui::AppEventNotification,
        _info: &tui::state::Info,
        _state: &tui::State,
        _render_trigger: &tui::RenderTrigger,
    ) {
    }

    fn render(&mut self, frame: &mut ratatui::Frame, state: &tui::State);
}

#[derive(Debug)]
pub struct Stack(Vec<Box<dyn tui::Panel>>);

impl Stack {
    pub fn new(current: Box<dyn tui::Panel>) -> Self {
        Self(vec![current])
    }

    pub fn pop(&mut self) {
        if self.0.len() <= 1 {
            return;
        }

        self.0.pop();
    }

    pub fn push(&mut self, panel: Box<dyn tui::Panel>) {
        self.0.push(panel);
    }

    pub fn current(&mut self) -> &mut dyn tui::Panel {
        let Some(current) = self.0.last_mut() else {
            unreachable!()
        };

        current.as_mut()
    }
}
