pub mod state;

mod component;
mod event;
mod task;

use std::sync::Arc;

use anyhow::Context;
use futures::StreamExt;

pub use crate::tui::state::AppState;

use crate::tui::{
    component::Component,
    event::{AppEvent, EventsChannel},
};

type AllComponents = [Box<dyn Component>; 1];
type RenderTrigger = tokio::sync::Notify;

pub struct App {
    terminal: ratatui::DefaultTerminal,
    state: AppState,
    components: AllComponents,
    render_trigger: Arc<RenderTrigger>,
    app_events: EventsChannel,
}

impl App {
    pub fn init(state: AppState) -> anyhow::Result<Self> {
        let terminal = ratatui::try_init().context("failed to initialize tui interface")?;

        let components = [Box::new(component::SeriesList::new()) as _];

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
            components,
            render_trigger,
            app_events,
        })
    }

    pub async fn run(mut self) {
        let mut terminal_event_stream = crossterm::event::EventStream::new();

        loop {
            tokio::select! {
                Some(Ok(event)) = terminal_event_stream.next() => {
                    if AppEvent::from(event).process(&mut self.state, &self.render_trigger).await == event::Result::Quit {
                        break;
                    }
                }
                Some(app_event) = self.app_events.recv() => {
                    if app_event.process(&mut self.state, &self.render_trigger).await == event::Result::Quit {
                        break;
                    }
                }
                _ = self.render_trigger.notified() => {
                    let draw_result = self
                        .terminal
                        .draw(|frame| render(frame, &mut self.components, &self.state));

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

fn render(frame: &mut ratatui::Frame, components: &mut AllComponents, state: &AppState) {
    for component in components {
        component.render(frame, state);
    }
}
