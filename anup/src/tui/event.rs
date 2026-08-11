use std::sync::Arc;

use tap::TapFallible;
use tokio::sync::{mpsc, oneshot};

use crate::tui;

#[derive(Debug, derive_more::From)]
pub enum AppEvent {
    #[from]
    Terminal(crossterm::event::Event),
    SeriesList(tui::state::series_list::Event),
    LogMessage(String),
    PushPanel(Box<dyn tui::Panel>),
    PopPanel,
}

impl AppEvent {
    pub fn process(
        self,
        info: &mut tui::state::Info,
        state: &mut tui::State,
        panel_stack: &mut tui::panel::Stack,
        render_trigger: Arc<tui::RenderTrigger>,
    ) -> Result {
        // don't log log message events, as it could put us in an infinite loop if the user
        // is actively viewing them
        if tracing::enabled!(tracing::Level::TRACE) && !matches!(self, Self::LogMessage(_)) {
            tracing::trace!(?self, "processing application event");
        }

        let (result, notif_event) = match self {
            Self::Terminal(event) => Self::process_terminal_event(
                event,
                info,
                state,
                panel_stack.current(),
                render_trigger.clone(),
            ),
            Self::SeriesList(event) => {
                state.series_list.process_event(event, &render_trigger);
                (Result::Continue(None), None)
            }
            Self::LogMessage(msg) => {
                if state.log_message_buffer.len() >= tui::MAX_LOG_MESSAGES {
                    state.log_message_buffer.pop_front();
                }

                state.log_message_buffer.push_back(msg);

                (
                    Result::Continue(None),
                    Some(AppEventNotification::LogMessage),
                )
            }
            Self::PushPanel(new_panel) => {
                panel_stack.push(new_panel);
                render_trigger.notify_one();
                (Result::Continue(None), None)
            }
            Self::PopPanel => {
                panel_stack.pop();
                render_trigger.notify_one();
                (Result::Continue(None), None)
            }
        };

        if let Some(notif_event) = notif_event {
            // the current panel may have changed; fetch it again
            panel_stack.current().process_event_notification(
                notif_event,
                info,
                state,
                &render_trigger,
            );
        }

        result
    }

    fn process_terminal_event(
        event: crossterm::event::Event,
        info: &mut tui::state::Info,
        state: &mut tui::State,
        panel: &mut dyn tui::Panel,
        render_trigger: Arc<tui::RenderTrigger>,
    ) -> (Result, Option<AppEventNotification>) {
        use crossterm::event::Event;

        match event {
            Event::Resize(width, height) => {
                info.size = ratatui::layout::Size { width, height };
                render_trigger.notify_one();

                (Result::Continue(None), None)
            }
            Event::Key(key) => (panel.process_input(key, info, state, render_trigger), None),
            _ => (Result::Continue(None), None),
        }
    }
}

#[derive(Debug)]
pub enum AppEventNotification {
    LogMessage,
}

pub enum Result {
    Continue(Option<AppEvent>),
    Quit,
}

pub struct EventsChannel {
    sender: EventSender,
    receiver: mpsc::Receiver<AppEvent>,
}

impl EventsChannel {
    pub fn new(buffer_size: usize) -> Self {
        let (tx, rx) = mpsc::channel(buffer_size);

        Self {
            sender: EventSender(tx),
            receiver: rx,
        }
    }

    pub fn new_sender(&self) -> EventSender {
        self.sender.clone()
    }

    pub async fn recv(&mut self) -> Option<AppEvent> {
        self.receiver.recv().await
    }
}

#[derive(Clone, derive_more::Deref)]
pub struct EventSender(mpsc::Sender<AppEvent>);

impl EventSender {
    pub async fn send_with_reply<T, R: Into<AppEvent>>(
        &self,
        payload_fn: impl FnOnce(oneshot::Sender<T>) -> R,
    ) -> Option<T> {
        let (tx, rx) = oneshot::channel();

        if let Err(err) = self.0.send(payload_fn(tx).into()).await {
            tracing::error!(?err, "event channel was closed");
        }

        rx.await
            .tap_err(|err| {
                tracing::error!(?err, "event did not send a reply when one was expected");
            })
            .ok()
    }
}
