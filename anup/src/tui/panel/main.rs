use std::{fmt::Debug, num::NonZeroUsize, sync::Arc};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Layout},
    widgets::Block,
};

use crate::{
    series,
    tui::{
        self,
        widget::{
            self, SeriesList,
            series::{SeasonInfo, SeriesInfo},
        },
    },
};

pub struct Main {
    series_list_state: widget::series::list::ViewState,
    series_cover_images: tui::ImageProtocolCache<anime::MediaID>,
}

impl Main {
    pub fn new() -> Self {
        Self {
            series_list_state: widget::series::list::ViewState::new(),
            series_cover_images: tui::ImageProtocolCache::new(
                // todo: make configurable
                // safety: 5 > 0
                NonZeroUsize::new(5).unwrap(),
            ),
        }
    }

    fn dispatch_cover_image_fetch(
        &self,
        info: &tui::state::Info,
        state: &tui::State,
        frame_data: &widget::series::list::FrameData,
        render_trigger: Arc<tui::RenderTrigger>,
    ) {
        let Some(season) = frame_data.get_selected_paired_season() else {
            return;
        };

        let Some(cover_url) = season
            .remote_info
            .cover_image_url
            .extra_large
            .as_deref()
            .and_then(|url| reqwest::Url::parse(url).ok())
        else {
            return;
        };

        let picker = info.image_protocol_picker.clone();
        let image_cache = state.image_cache.clone();
        let series_cover_images = self.series_cover_images.clone();
        let id = season.remote_info.id;

        tokio::spawn(async move {
            series_cover_images
                .ensure_cached(id, cover_url, image_cache, &picker, render_trigger.clone())
                .await;

            render_trigger.notify_one();
        });
    }
}

impl Debug for Main {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Main")
            .field("series_list_state", &self.series_list_state)
            .finish_non_exhaustive()
    }
}

impl tui::Panel for Main {
    fn process_input(
        &mut self,
        event: KeyEvent,
        info: &tui::state::Info,
        state: &mut tui::State,
        render_trigger: Arc<tui::RenderTrigger>,
    ) -> tui::event::Result {
        if !event.is_press() {
            return tui::event::Result::Continue(None);
        }

        match event.code {
            KeyCode::Esc => {
                return tui::event::Result::Quit;
            }
            KeyCode::Char('s' | 'S') | KeyCode::Down => {
                self.series_list_state.select_next();
            }
            KeyCode::Char('w' | 'W') | KeyCode::Up => {
                self.series_list_state.select_previous();
            }
            KeyCode::Char('d' | 'D') | KeyCode::Right => {
                self.series_list_state.select_series_formats();
            }
            KeyCode::Char('a' | 'A') | KeyCode::Left => {
                self.series_list_state.select_root_series();
            }
            KeyCode::Char('~') => {
                let panel = Box::new(tui::panel::Log::new());
                let event = tui::AppEvent::PushPanel(panel);
                return tui::event::Result::Continue(Some(event));
            }
            _ => return tui::event::Result::Continue(None),
        }

        let frame_data = self.series_list_state.frame_data(&state.series_list);

        // if we made it this far then we selected a new series list entry; fetch
        // the cover image for the selected series (when applicable)
        self.dispatch_cover_image_fetch(info, state, &frame_data, render_trigger.clone());

        render_trigger.notify_one();
        tui::event::Result::Continue(None)
    }

    fn render(&mut self, frame: &mut ratatui::Frame, state: &tui::State) {
        use widget::series::list::FrameData;

        let [left_area, right_area] =
            Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
                .areas(frame.area());

        let frame_data = self.series_list_state.frame_data(&state.series_list);

        let series_list = {
            let title = match frame_data {
                FrameData::RootSeries { .. } => "Series List",
                FrameData::SeriesFormats { .. } => "Series Format Selection",
            };

            SeriesList::new(&frame_data).block(Block::bordered().title(title))
        };

        frame.render_widget(series_list, left_area);

        match &frame_data {
            FrameData::RootSeries { selected_entry, .. } => {
                let info_panel =
                    SeriesInfo::new(*selected_entry).block(Block::bordered().title("Info"));

                frame.render_widget(info_panel, right_area);
            }
            FrameData::SeriesFormats {
                selected_season, ..
            } => {
                let mut series_cover_images = self.series_cover_images.lock();

                let cover_image = match selected_season {
                    series::RemoteSeasonPairing::Paired(season) => {
                        series_cover_images.get_mut(&season.remote_info.id)
                    }
                    _ => None,
                };

                let info_panel = SeasonInfo::new(Some(*selected_season), cover_image)
                    .block(Block::bordered().title("Season Info"));

                frame.render_widget(info_panel, right_area);
            }
        }

        self.series_list_state.apply_frame_data(frame_data);
    }
}
