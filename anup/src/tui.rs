pub mod panel;
pub mod state;

mod event;
mod image_protocol;
mod widget;

use std::{
    io::{self, Write},
    ops::ControlFlow,
    sync::Arc,
};

use anyhow::Context;
use futures::StreamExt;

pub use event::{AppEvent, AppEventNotification, EventsChannel};
pub use image_protocol::ImageProtocolCache;
pub use panel::Panel;
pub use state::State;

pub const MAX_LOG_MESSAGES: usize = 500;

type RenderTrigger = tokio::sync::Notify;

pub struct App {
    terminal: ratatui::DefaultTerminal,
    info: state::Info,
    state: State,
    panel_stack: panel::Stack,
    render_trigger: Arc<RenderTrigger>,
    app_events: EventsChannel,
}

impl App {
    pub fn init(
        state: State,
        panel_stack: panel::Stack,
        log_message_rx: tokio::sync::mpsc::Receiver<String>,
    ) -> anyhow::Result<Self> {
        let terminal = ratatui::try_init().context("failed to initialize tui interface")?;

        let info = state::Info {
            size: terminal
                .size()
                .context("failed to query size of terminal")?,
            image_protocol_picker: ratatui_image::picker::Picker::from_query_stdio()
                .context("failed to determine terminal graphics capabilities")
                .map(Arc::new)?,
        };

        let render_trigger = Arc::new(RenderTrigger::new());
        // trigger the first render immediately
        render_trigger.notify_one();

        let app_events = EventsChannel::new(64);

        tokio::spawn(App::process_log_events(
            app_events.new_sender(),
            log_message_rx,
        ));

        tokio::spawn(state::series_list::resolve_dir::resolve_all(
            app_events.new_sender(),
            state.series_scan_dir.clone(),
        ));

        Ok(Self {
            terminal,
            info,
            state,
            panel_stack,
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

                    if self.process_app_event(event).await == ControlFlow::Break(()) {
                        break;
                    }
                }
                Some(app_event) = self.app_events.recv() => {
                    if self.process_app_event(app_event).await == ControlFlow::Break(()) {
                        break;
                    }
                }
                _ = self.render_trigger.notified() => {
                    let draw_result = self
                        .terminal
                        .draw(|frame| self.panel_stack.current().render(frame, &self.state));

                    if let Err(err) = draw_result {
                        tracing::error!("failed to render tui frame: {err}");
                        break;
                    }
                },
            }
        }
    }

    async fn process_app_event(&mut self, app_event: AppEvent) -> ControlFlow<()> {
        let result = app_event
            .process(
                &mut self.info,
                &mut self.state,
                &mut self.panel_stack,
                self.render_trigger.clone(),
            )
            .await;

        match result {
            event::Result::Continue(event) => {
                if let Some(event) = event
                    && self.app_events.new_sender().send(event).await.is_err()
                {
                    tracing::error!(
                        "app event channel was closed when trying to send follow up event"
                    );
                }

                ControlFlow::Continue(())
            }
            event::Result::Quit => ControlFlow::Break(()),
        }
    }

    async fn process_log_events(
        event_sender: event::EventSender,
        mut log_message_rx: tokio::sync::mpsc::Receiver<String>,
    ) {
        while let Some(msg) = log_message_rx.recv().await {
            if event_sender.send(AppEvent::LogMessage(msg)).await.is_err() {
                break;
            }
        }

        tracing::trace!("log message processor finished");
    }
}

impl Drop for App {
    fn drop(&mut self) {
        ratatui::restore();
    }
}

#[derive(Clone)]
pub struct LogTransmitter {
    buffer: Vec<u8>,
    pub tx: Arc<tokio::sync::mpsc::Sender<String>>,
}

impl LogTransmitter {
    pub fn new(tx: tokio::sync::mpsc::Sender<String>) -> Self {
        Self {
            buffer: Vec::new(),
            tx: Arc::new(tx),
        }
    }
}

impl io::Write for LogTransmitter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buffer.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        let buffer = std::mem::take(&mut self.buffer);
        let content = String::from_utf8(buffer).map_err(|_| io::ErrorKind::InvalidData)?;

        let tx_clone = self.tx.clone();

        tokio::spawn(async move {
            _ = tx_clone.send(content).await;
        });

        Ok(())
    }
}

impl Drop for LogTransmitter {
    fn drop(&mut self) {
        _ = self.flush();
    }
}
