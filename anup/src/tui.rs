pub mod panel;
pub mod state;

mod event;
mod widget;

use std::sync::Arc;

use anyhow::Context;
use futures::StreamExt;

pub use crate::tui::state::AppState;

use crate::tui::{
    event::{AppEvent, EventsChannel},
    panel::Panel,
};

type RenderTrigger = tokio::sync::Notify;

pub struct App<'a> {
    terminal: ratatui::DefaultTerminal,
    state: AppState,
    root_panel: &'a mut dyn Panel,
    render_trigger: Arc<RenderTrigger>,
    app_events: EventsChannel,
}

impl<'a> App<'a> {
    pub fn init(state: AppState, root_panel: &'a mut dyn Panel) -> anyhow::Result<Self> {
        let terminal = ratatui::try_init().context("failed to initialize tui interface")?;

        let render_trigger = Arc::new(RenderTrigger::new());
        // trigger the first render immediately
        render_trigger.notify_one();

        let app_events = EventsChannel::new(64);

        tokio::spawn(state::series_list::resolve_dir::resolve_all(
            app_events.new_sender(),
            state.series_scan_dir.clone(),
        ));

        Ok(Self {
            terminal,
            state,
            root_panel,
            render_trigger,
            app_events,
        })
    }

    pub async fn run(mut self) {
        let mut terminal_event_stream = crossterm::event::EventStream::new();

        loop {
            tokio::select! {
                Some(Ok(event)) = terminal_event_stream.next() => {
                    let event = AppEvent::from(event);

                    let result = event
                        .process(&mut self.state, self.root_panel, &self.render_trigger)
                        .await;

                    if result == event::Result::Quit {
                        break;
                    }
                }
                Some(app_event) = self.app_events.recv() => {
                    let result = app_event
                        .process(&mut self.state, self.root_panel, &self.render_trigger)
                        .await;

                    if result == event::Result::Quit {
                        break;
                    }
                }
                _ = self.render_trigger.notified() => {
                    let draw_result = self
                        .terminal
                        .draw(|frame| self.root_panel.render(frame, &self.state));

                    if let Err(err) = draw_result {
                        tracing::error!("failed to render tui frame: {err}");
                        break;
                    }
                },
            }
        }
    }
}

impl Drop for App<'_> {
    fn drop(&mut self) {
        ratatui::restore();
    }
}
