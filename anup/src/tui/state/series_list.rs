pub mod resolve_dir;

use std::borrow::Cow;

use derive_more::Deref;

use crate::{series, tui};

#[derive(Debug, Deref)]
pub struct SeriesList(Vec<Entry>);

impl SeriesList {
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    pub fn push(&mut self, state: EntryState) -> usize {
        let stable_index = self.len();

        self.0.push(Entry {
            state,
            stable_index,
        });

        stable_index
    }

    pub fn get_mut(&mut self, stable_index: usize) -> Option<&mut EntryState> {
        self.0
            .iter_mut()
            .find_map(|s| (s.stable_index == stable_index).then_some(&mut s.state))
    }

    pub fn set(&mut self, stable_index: usize, state: EntryState) -> bool {
        if let Some(existing) = self.get_mut(stable_index) {
            *existing = state;
            return true;
        }

        false
    }

    pub fn process_event(&mut self, event: Event, render_trigger: &tui::RenderTrigger) {
        match event {
            Event::Create {
                state,
                stable_index_reply,
            } => {
                let inserted_index = self.push(state);
                stable_index_reply.send(inserted_index).ok();
                render_trigger.notify_one();
            }
            Event::Update {
                stable_index,
                state,
            } => {
                if self.set(stable_index, state) {
                    render_trigger.notify_one();
                }
            }
        }
    }
}

#[derive(Debug, derive_more::Deref)]
pub struct Entry {
    #[deref]
    pub state: EntryState,
    /// Stable index that can be used to identify the series in a [`Vec`],
    /// regardless of the [`Vec`]'s current sort order.
    pub stable_index: usize,
}

#[derive(Debug)]
pub enum EntryState {
    Detected,
    Scanning,
    Resolving(series::LocalRoot),
    Resolved(series::RootPairing),
    Unresolved(series::LocalRoot),
    Failure { name: String, error: EntryError },
}

impl EntryState {
    pub const fn get_resolved(&self) -> Option<&series::RootPairing> {
        match self {
            Self::Resolved(series) => Some(series),
            _ => None,
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum EntryError {
    #[error(transparent)]
    ParseError(#[from] series::root::local::ParseError),
    #[error("{0}")]
    Panic(Cow<'static, str>),
    #[error(transparent)]
    Automatch(#[from] series::automatch::Error),
}

#[derive(Debug)]
pub enum Event {
    Create {
        state: EntryState,
        stable_index_reply: tokio::sync::oneshot::Sender<usize>,
    },
    Update {
        stable_index: usize,
        state: EntryState,
    },
}

impl From<Event> for tui::AppEvent {
    fn from(value: Event) -> Self {
        Self::SeriesList(value)
    }
}
