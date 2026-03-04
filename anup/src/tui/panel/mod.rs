pub mod main;

use crossterm::event::KeyEvent;
pub use main::MainPanel;

use crate::tui;

pub trait Panel {
    fn process_input(
        &mut self,
        event: KeyEvent,
        state: &mut tui::AppState,
        render_trigger: &tui::RenderTrigger,
    );

    fn render(&mut self, frame: &mut ratatui::Frame, state: &tui::AppState);
}
