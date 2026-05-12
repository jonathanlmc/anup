use std::borrow::Cow;

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{self, Block, ListState, StatefulWidget},
};

use crate::{series, tui::state};

/// List many series, or the formats of one.
///
/// Selection and listing options can be controlled by persisting a [`ViewState`]
/// and calling its associated methods when appropriate.
pub struct SeriesList<'a> {
    frame_state: FrameViewState<'a>,
    block: Option<Block<'a>>,
}

impl<'a> SeriesList<'a> {
    /// Create a new series list.
    ///
    /// The state for the current frame can be obtained from the persisted [`ViewState`] by
    /// calling its [`ViewState::frame_state`] method.
    pub fn new(frame_state: FrameViewState<'a>) -> Self {
        Self {
            frame_state,
            block: None,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    fn rendered_single_series_format<'b>(
        format: series::Format,
        season_num: u16,
        season_data: &'b series::RemoteSeasonPairing,
    ) -> Line<'b> {
        let mut format_line = Line::default();

        format_line.push_span(Span::styled(
            Self::format_prefix_str(format, season_num),
            Style::default().dark_gray().bold(),
        ));

        match &season_data {
            series::RemoteSeasonPairing::Paired { remote_info, .. } => {
                // todo: use configured title
                format_line.push_span(Span::styled(
                    &remote_info.title.romaji,
                    Style::default().gray(),
                ));
            }
            series::RemoteSeasonPairing::Unpaired { .. } => {
                format_line.push_span(Span::styled(
                    "Unmatched Episodes",
                    Style::default().dark_gray().italic(),
                ));
            }
        }

        format_line
    }

    fn format_prefix_str(format: series::Format, season_num: u16) -> Cow<'static, str> {
        use series::Format::*;

        match format {
            Tv => format!("TV{season_num} ").into(),
            Special => "SP ".into(),
            Movie => "MV ".into(),
            Ona => "ONA ".into(),
            Ova => "OVA ".into(),
            Music => "MU ".into(),
        }
    }

    fn rendered_root_series(series_list: &[state::series_list::Entry]) -> widgets::List<'_> {
        let items = series_list.iter().map(|series| {
            use state::series_list::EntryState;

            let mut style = Style::default();

            let (color, modifier, name) = match &series.state {
                EntryState::Detected => (Color::DarkGray, None, "Detected.."),
                EntryState::Scanning => (Color::DarkGray, Some(Modifier::ITALIC), "Scanning.."),
                EntryState::Resolving(name) => (Color::Green, None, name.as_str()),
                EntryState::Resolved(pairing) => (Color::Gray, None, pairing.name.as_str()),
                EntryState::Unresolved(local) => (
                    Color::DarkGray,
                    Some(Modifier::ITALIC | Modifier::BOLD),
                    local.parsed_name.as_str(),
                ),
                EntryState::Failure { name, .. } => {
                    (Color::Red, Some(Modifier::ITALIC), name.as_str())
                }
            };

            style = style.fg(color);

            if let Some(modifier) = modifier {
                style = style.add_modifier(modifier);
            }

            Line::styled(name, style)
        });

        widgets::List::new(items)
    }

    fn rendered_series_formats(series_data: &series::RootPairing) -> widgets::List<'_> {
        let items = series_data.pairings.iter().flat_map(|(format, seasons)| {
            seasons.into_iter().map(|(season, season_data)| {
                Self::rendered_single_series_format(*format, *season, season_data)
            })
        });

        widgets::List::new(items)
    }
}

impl<'a> ratatui::widgets::Widget for SeriesList<'a> {
    fn render(mut self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let mut list = match self.frame_state.data {
            FrameData::RootSeries { series_list, .. } => Self::rendered_root_series(series_list),
            FrameData::SeriesFormats { series_data, .. } => {
                Self::rendered_series_formats(series_data)
            }
        };

        let mut list_state = {
            let mut list_state = ListState::default();
            let selected_index = self.frame_state.selected_index_mut();

            if let Some(idx) = selected_index {
                // wrap the list selection index around the top or bottom
                let len = list.len().min(i32::MAX as usize) as i32;
                let rem = *idx % len;
                *idx = if rem < 0 { rem + len } else { rem };

                list_state.select(Some(*idx as usize));
            }

            list_state
        };

        if let Some(block) = self.block {
            list = list.block(block);
        }

        list = list
            .highlight_symbol(Line::styled(">", Style::default().light_cyan()))
            .highlight_style(Style::default().bold());

        StatefulWidget::render(list, area, buf, &mut list_state);
    }
}

#[derive(Debug)]
pub enum ViewState {
    RootSeries {
        selected_index: Option<i32>,
    },
    SeriesFormats {
        root_series_index: i32,
        selected_format_index: i32,
    },
}

impl ViewState {
    pub fn new() -> Self {
        Self::RootSeries {
            selected_index: None,
        }
    }

    /// Piece together data required to render a single frame of the view state.
    ///
    /// The view state may be altered if it does not contain valid data for a frame.
    pub fn frame_state<'a>(
        &'a mut self,
        series_list: &'a [state::series_list::Entry],
    ) -> FrameViewState<'a> {
        FrameViewState::with_view_state(series_list, self)
    }

    pub fn select_root_series(&mut self) {
        match self {
            Self::RootSeries { .. } => (),
            &mut Self::SeriesFormats {
                root_series_index, ..
            } => {
                *self = Self::RootSeries {
                    selected_index: Some(root_series_index),
                }
            }
        }
    }

    pub fn select_series_formats(&mut self) {
        match self {
            Self::RootSeries { selected_index } => {
                // ensure there's a selected series, since we'll be viewing
                // the formats of one (which implies selection)
                let selected_series = *selected_index.get_or_insert(0);

                *self = Self::SeriesFormats {
                    root_series_index: selected_series,
                    selected_format_index: 0,
                };
            }
            Self::SeriesFormats { .. } => (),
        }
    }

    pub fn select_next(&mut self) {
        self.modify_selected_index(|idx| idx.map(|i| i + 1).unwrap_or(0));
    }

    pub fn select_previous(&mut self) {
        self.modify_selected_index(|idx| idx.map(|i| i - 1).unwrap_or(0));
    }

    pub fn root_series_index(&self) -> Option<i32> {
        match self {
            Self::RootSeries { selected_index } => *selected_index,
            Self::SeriesFormats {
                root_series_index, ..
            } => Some(*root_series_index),
        }
    }

    fn current_selected_index_mut(&mut self) -> Option<&mut i32> {
        match self {
            Self::RootSeries { selected_index } => selected_index.as_mut(),
            Self::SeriesFormats {
                selected_format_index,
                ..
            } => Some(selected_format_index),
        }
    }

    fn modify_selected_index(&mut self, func: impl FnOnce(Option<i32>) -> i32) {
        match self {
            Self::RootSeries { selected_index } => *selected_index = Some(func(*selected_index)),
            Self::SeriesFormats {
                selected_format_index,
                ..
            } => *selected_format_index = func(Some(*selected_format_index)),
        }
    }
}

/// The view state for an individual render frame.
pub struct FrameViewState<'a> {
    view_state: &'a mut ViewState,
    data: FrameData<'a>,
}

impl<'a> FrameViewState<'a> {
    /// Construct a frame view state.
    ///
    /// If the conditions required to render a frame for the current view state could not
    /// be met, the view state will be set to [`ViewState::RootSeries`], and the returned frame
    /// state data will also be [`FrameData::RootSeries`].
    fn with_view_state(
        series_list: &'a [state::series_list::Entry],
        view_state: &'a mut ViewState,
    ) -> Self {
        match view_state {
            ViewState::RootSeries { .. } => Self {
                view_state,
                data: FrameData::RootSeries { series_list },
            },
            ViewState::SeriesFormats {
                root_series_index, ..
            } => {
                'blk: {
                    let Some(root_index_usize) = (*root_series_index).try_into().ok() else {
                        break 'blk;
                    };

                    let entry = match series_list.get::<usize>(root_index_usize) {
                        Some(entry) => entry,
                        None => break 'blk,
                    };

                    match entry.get_resolved() {
                        Some(series_data) => {
                            return Self {
                                view_state,
                                data: FrameData::SeriesFormats { series_data },
                            };
                        }
                        None => break 'blk,
                    }
                };

                *view_state = ViewState::RootSeries {
                    selected_index: Some(*root_series_index),
                };

                Self {
                    view_state,
                    data: FrameData::RootSeries { series_list },
                }
            }
        }
    }

    pub fn data(&self) -> &FrameData<'_> {
        &self.data
    }

    fn selected_index_mut(&mut self) -> Option<&mut i32> {
        // `view_state` is kept in sync with the frame data variant, so we can just
        // return the selected index regardless of its variant
        self.view_state.current_selected_index_mut()
    }
}

pub enum FrameData<'a> {
    RootSeries {
        series_list: &'a [state::series_list::Entry],
    },
    SeriesFormats {
        series_data: &'a series::RootPairing,
    },
}
