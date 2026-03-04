pub mod state;

mod event;
mod panel;
mod task;
mod widget;

use std::sync::Arc;

use anyhow::Context;
use futures::StreamExt;

pub use crate::tui::state::AppState;

use crate::tui::{
    event::{AppEvent, EventsChannel},
    panel::Panel,
};

type AllPanels = [Box<dyn Panel>; 1];
type RenderTrigger = tokio::sync::Notify;

pub struct App {
    terminal: ratatui::DefaultTerminal,
    state: AppState,
    panels: AllPanels,
    render_trigger: Arc<RenderTrigger>,
    app_events: EventsChannel,
}

impl App {
    pub fn init(state: AppState) -> anyhow::Result<Self> {
        let terminal = ratatui::try_init().context("failed to initialize tui interface")?;

        let panels = [Box::new(panel::MainPanel::new()) as _];

        let render_trigger = Arc::new(RenderTrigger::new());
        // trigger the first render immediately
        render_trigger.notify_one();

        let app_events = EventsChannel::new(64);

        tokio::spawn(task::series::scan_and_resolve_all_in_dir(
            app_events.new_sender(),
            state.series_scan_dir.clone(),
        ));

        Ok(Self {
            terminal,
            state,
            panels,
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
                        .process(&mut self.state, &mut self.panels, &self.render_trigger)
                        .await;

                    if result == event::Result::Quit {
                        break;
                    }
                }
                Some(app_event) = self.app_events.recv() => {
                    let result = app_event
                        .process(&mut self.state, &mut self.panels, &self.render_trigger)
                        .await;

                    if result == event::Result::Quit {
                        break;
                    }
                }
                _ = self.render_trigger.notified() => {
                    let draw_result = self
                        .terminal
                        .draw(|frame| render(frame, &mut self.panels, &self.state));

                    if let Err(err) = draw_result {
                        tracing::error!("failed to render tui frame: {err}");
                        break;
                    }
                },
            }
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        ratatui::restore();
    }
}

fn render(frame: &mut ratatui::Frame, panels: &mut AllPanels, state: &AppState) {
    for component in panels {
        component.render(frame, state);
    }
}
