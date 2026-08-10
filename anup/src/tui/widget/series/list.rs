use std::borrow::Cow;

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{self, Block, ListState, StatefulWidget, Widget},
};
use tap::TapFallible;

use crate::{series, tui::state};

/// List many series, or the formats of one.
///
/// Selection and listing options can be controlled by persisting a [`ViewState`]
/// and calling its associated methods when appropriate.
pub struct SeriesList<'a> {
    frame_data: &'a FrameData<'a>,
    block: Option<Block<'a>>,
}

impl<'a> SeriesList<'a> {
    /// Create a new series list.
    ///
    /// The frame data can be obtained from the persisted [`ViewState`] by calling
    /// its [`ViewState::frame_data`] method.
    pub fn new(frame_data: &'a FrameData<'a>) -> Self {
        Self {
            frame_data,
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
            series::RemoteSeasonPairing::Paired(season) => {
                // todo: use configured title
                format_line.push_span(Span::styled(
                    &season.remote_info.title.romaji,
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
                EntryState::Resolving(series) => (Color::Green, None, series.parsed_name.as_str()),
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
        let items = series_data
            .all_format_seasons()
            .map(|s| Self::rendered_single_series_format(s.format, s.season, s.pairing));

        widgets::List::new(items)
    }
}

impl Widget for SeriesList<'_> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let (mut list, selected_index) = match self.frame_data {
            FrameData::RootSeries {
                series_list,
                selected_index,
                ..
            } => (Self::rendered_root_series(series_list), selected_index),
            FrameData::SeriesFormats {
                series_data,
                selected_index,
                ..
            } => (Self::rendered_series_formats(series_data), selected_index),
        };

        if let Some(block) = self.block {
            list = list.block(block);
        }

        list = list
            .highlight_symbol(Line::styled(">", Style::default().light_cyan()))
            .highlight_style(Style::default().bold());

        let mut list_state = ListState::default();
        list_state.select(Some(*selected_index));

        StatefulWidget::render(list, area, buf, &mut list_state);
    }
}

#[derive(Debug)]
pub enum ViewState {
    RootSeries { selected: i32 },
    SeriesFormats { root: i32, selected: i32 },
}

impl ViewState {
    pub fn new() -> Self {
        Self::RootSeries { selected: 0 }
    }

    /// Produce a snapshot from the view state that can be used to render a frame.
    ///
    /// After using the frame data for rendering, the [`ViewState::apply_frame_data`] method
    /// should be called to apply state changes produced from the rendered frame.
    pub fn frame_data<'a>(&self, series_list: &'a [state::series_list::Entry]) -> FrameData<'a> {
        FrameData::try_from_view_state(self, series_list).unwrap_or_else(|| {
            // some sort of invalid state was hit; reset to the top-level `RootSeries` view
            let index = self.root_series_index().max(0) as usize;

            FrameData::RootSeries {
                series_list,
                selected_index: index,
                selected_entry: series_list.get(index),
            }
        })
    }

    pub fn apply_frame_data(&mut self, data: FrameData<'_>) {
        match data {
            FrameData::RootSeries { selected_index, .. } => {
                *self = Self::RootSeries {
                    selected: selected_index as i32,
                }
            }
            FrameData::SeriesFormats {
                root_series_index,
                selected_index,
                ..
            } => {
                *self = Self::SeriesFormats {
                    root: root_series_index as i32,
                    selected: selected_index as i32,
                }
            }
        }
    }

    pub fn select_root_series(&mut self) {
        match self {
            Self::RootSeries { .. } => (),
            Self::SeriesFormats { root, .. } => {
                *self = Self::RootSeries { selected: *root };
            }
        }
    }

    pub fn select_series_formats(&mut self) {
        match self {
            Self::RootSeries { selected } => {
                *self = Self::SeriesFormats {
                    root: *selected,
                    selected: 0,
                };
            }
            Self::SeriesFormats { .. } => (),
        }
    }

    pub fn select_next(&mut self) {
        *self.selected_mut() += 1;
    }

    pub fn select_previous(&mut self) {
        *self.selected_mut() -= 1;
    }

    fn root_series_index(&self) -> i32 {
        match self {
            Self::RootSeries { selected } => *selected,
            Self::SeriesFormats { root, .. } => *root,
        }
    }

    fn selected_mut(&mut self) -> &mut i32 {
        match self {
            Self::RootSeries { selected } => selected,
            Self::SeriesFormats { selected, .. } => selected,
        }
    }
}

/// Data snapshot for rendering the series list.
///
/// Created by [`ViewState::frame_data`] each frame. Contains the list data
/// and a pre-clamped selection index.
pub enum FrameData<'a> {
    RootSeries {
        series_list: &'a [state::series_list::Entry],
        selected_index: usize,
        selected_entry: Option<&'a state::series_list::Entry>,
    },
    SeriesFormats {
        root_series_index: usize,
        series_data: &'a series::RootPairing,
        selected_index: usize,
        // todo: use in season info panel
        #[allow(unused)]
        selected_season: &'a series::RemoteSeasonPairing,
    },
}

impl<'a> FrameData<'a> {
    fn try_from_view_state(
        view_state: &ViewState,
        series_list: &'a [state::series_list::Entry],
    ) -> Option<Self> {
        match view_state {
            ViewState::RootSeries { selected } => {
                let selected = Self::wrap_index_around(*selected, series_list.len());
                let series_data = series_list.get(selected)?;

                Some(Self::RootSeries {
                    series_list,
                    selected_index: selected,
                    selected_entry: Some(series_data),
                })
            }
            ViewState::SeriesFormats {
                root,
                selected: unwrapped_selected,
            } => {
                let root: usize = (*root)
                    .try_into()
                    .tap_err(|_| {
                        tracing::warn!(
                            "stored root index for series format listing is not a valid usize"
                        )
                    })
                    .ok()?;

                let series_data = series_list.get(root)?;
                let resolved_entry = series_data.get_resolved()?;

                let len = resolved_entry
                    .pairings
                    .values()
                    .map(|seasons| seasons.len())
                    .sum();

                let selected_index = Self::wrap_index_around(*unwrapped_selected, len);

                let selected_season = resolved_entry
                    .all_format_seasons()
                    .nth(selected_index)
                    .map(|s| s.pairing)?;

                Some(Self::SeriesFormats {
                    root_series_index: root,
                    series_data: resolved_entry,
                    selected_index,
                    selected_season,
                })
            }
        }
    }

    fn wrap_index_around(index: i32, len: usize) -> usize {
        let len = len.min(i32::MAX as usize) as i32;

        if len == 0 {
            return 0;
        }

        let rem = index % len;

        if rem < 0 {
            (rem + len) as usize
        } else {
            rem as usize
        }
    }

    pub fn get_selected_paired_season(&self) -> Option<&series::PairedSeason> {
        let Self::SeriesFormats {
            selected_season, ..
        } = self
        else {
            return None;
        };

        let series::RemoteSeasonPairing::Paired(season) = selected_season else {
            return None;
        };

        Some(season)
    }
}
