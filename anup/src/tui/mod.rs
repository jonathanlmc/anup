mod state;

use std::sync::Arc;

use anyhow::Context;
use crossterm::event::{KeyCode, KeyModifiers};
use futures::StreamExt;
use tap::TapFallible;

pub use crate::tui::state::AppState;

use crate::tui::state::RenderState;

pub struct App {
    terminal: ratatui::DefaultTerminal,
    state: AppState,
    render_trigger: Arc<tokio::sync::Notify>,
}

impl App {
    pub fn init(state: AppState) -> anyhow::Result<Self> {
        let terminal = ratatui::try_init().context("failed to initialize tui interface")?;

        Ok(Self {
            terminal,
            state,
            render_trigger: Arc::new(tokio::sync::Notify::new()),
        })
    }

    pub async fn run(mut self) {
        let mut event_stream = crossterm::event::EventStream::new();

        loop {
            self.terminal
                .draw(|frame| {
                    render(
                        frame,
                        RenderState::new(&mut self.state, &self.render_trigger),
                    )
                })
                .tap_err(|err| tracing::error!("failed to render tui frame: {err}"));

            tokio::select! {
                Some(Ok(event)) = event_stream.next() => {
                    if process_event(event, &mut self.state).await {
                        break;
                    }
                }
                _ = self.render_trigger.notified() => (),
            }
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        ratatui::restore();
    }
}

fn render(frame: &mut ratatui::Frame, state: RenderState) {
    //
}

async fn process_event(event: crossterm::event::Event, state: &mut AppState) -> bool {
    if let Some(key) = event.as_key_press_event()
        && key.modifiers.contains(KeyModifiers::SHIFT)
        && key.code.is_char('Q')
    {
        return true;
    }

    false
}
