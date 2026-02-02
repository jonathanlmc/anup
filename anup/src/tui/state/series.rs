use std::borrow::Cow;

use crate::series::Series;

#[derive(Debug, derive_more::Deref)]
pub struct List(Vec<Entry>);

impl List {
    pub fn new() -> Self {
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
    Resolved(Series),
    Resolving(String),
    Detected(Cow<'static, str>),
    Unmatched {
        local_series: anime_detect::TopLevelSeries,
        searched_anime: Vec<anime::Anime>,
    },
    Failure {
        name: String,
        error: anyhow::Error,
    },
}

impl EntryState {
    pub fn name(&self) -> &str {
        match self {
            Self::Resolved(series) => &series.parsed_local_name,
            Self::Resolving(name) => name,
            Self::Detected(name) => name,
            Self::Unmatched { local_series, .. } => &local_series.parsed_name,
            Self::Failure { name, .. } => name,
        }
    }
}
