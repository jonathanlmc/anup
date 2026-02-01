use std::ops::{Deref, DerefMut};

#[derive(Debug)]
pub struct AppState {}

#[derive(derive_more::Constructor)]
pub struct RenderState<'a> {
    pub app: &'a mut AppState,
    pub render_trigger: &'a tokio::sync::Notify,
}

impl RenderState<'_> {
    pub fn trigger_render(&self) {
        self.render_trigger.notify_one();
    }
}

impl Deref for RenderState<'_> {
    type Target = AppState;

    fn deref(&self) -> &Self::Target {
        self.app
    }
}

impl DerefMut for RenderState<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.app
    }
}
