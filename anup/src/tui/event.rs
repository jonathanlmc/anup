pub mod series;

use tap::TapFallible;
use tokio::sync::{mpsc, oneshot};

use crate::tui;

#[derive(Debug, derive_more::From)]
pub enum AppEvent {
    #[from]
    Terminal(crossterm::event::Event),
    Series(series::Payload),
}

impl AppEvent {
    pub async fn process(
        self,
        state: &mut tui::AppState,
        panel: &mut dyn tui::Panel,
        render_trigger: &tui::RenderTrigger,
    ) -> Result {
        tracing::trace!(?self, "processing application event");

        match self {
            Self::Terminal(event) => {
                Self::process_terminal_event(event, state, panel, render_trigger).await
            }
            Self::Series(payload) => {
                series::process(payload, state, render_trigger).await;
                Result::Continue
            }
        }
    }

    async fn process_terminal_event(
        event: crossterm::event::Event,
        state: &mut tui::AppState,
        panel: &mut dyn tui::Panel,
        render_trigger: &tui::RenderTrigger,
    ) -> Result {
        use crossterm::event::Event;

        match event {
            Event::Resize(_, _) => {
                render_trigger.notify_one();
                Result::Continue
            }
            Event::Key(key) => panel.process_input(key, state, render_trigger),
            _ => Result::Continue,
        }
    }
}

#[derive(PartialEq, Eq)]
pub enum Result {
    Continue,
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
    pub async fn send_with_reply<T>(
        &self,
        payload_fn: impl FnOnce(oneshot::Sender<T>) -> AppEvent,
    ) -> Option<T> {
        let (tx, rx) = oneshot::channel();

        if let Err(err) = self.0.send(payload_fn(tx)).await {
            tracing::error!(?err, "event channel was closed");
        }

        rx.await
            .tap_err(|err| {
                tracing::error!(?err, "event did not send a reply when one was expected")
            })
            .ok()
    }
}
