use crate::tui::{self, state};

#[derive(Debug)]
pub enum Payload {
    Create {
        state: state::series::EntryState,
        stable_index_reply: tokio::sync::oneshot::Sender<usize>,
    },
    Update {
        stable_index: usize,
        state: state::series::EntryState,
    },
}

pub(super) async fn process(payload: Payload, app_state: &mut tui::AppState) {
    match payload {
        Payload::Create {
            state,
            stable_index_reply,
        } => {
            let index = app_state.series.push(state);
            stable_index_reply.send(index).ok();
        }
        Payload::Update {
            stable_index,
            state,
        } => {
            app_state.series.set(stable_index, state);
        }
    }
}
