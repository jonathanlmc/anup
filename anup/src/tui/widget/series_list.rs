use std::borrow::Cow;

use derive_more::{Deref, DerefMut};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{self, Block, StatefulWidget},
};

use crate::{
    series,
    tui::{self, state},
};

pub struct SeriesList<'a> {
    series: &'a [tui::state::series_list::Entry],
    block: Option<Block<'a>>,
}

impl<'a> SeriesList<'a> {
    pub fn new(series: &'a [tui::state::series_list::Entry]) -> Self {
        Self {
            series,
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
}

impl<'a> ratatui::widgets::StatefulWidget for SeriesList<'a> {
    type State = State;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let (mut list, list_state) = match &mut state.view_state {
            ViewState::TopLevelSeries(list_state) => {
                let items = self.series.iter().map(|series| {
                    use state::series_list::EntryState;

                    let mut style = Style::default();

                    let (color, modifier, name) = match &series.state {
                        EntryState::Detected => (Color::DarkGray, None, "Detected.."),
                        EntryState::Scanning => {
                            (Color::DarkGray, Some(Modifier::ITALIC), "Scanning..")
                        }
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

                (widgets::List::new(items), list_state)
            }
            ViewState::SingleSeriesFormats {
                selected_series,
                current_list_state,
                top_level_series_state,
            } => {
                let entry = match self.series.get(*selected_series) {
                    Some(entry) => entry,
                    None => {
                        state.view_state = ViewState::TopLevelSeries(*top_level_series_state);
                        return self.render(area, buf, state);
                    }
                };

                let series_data = match entry.get_resolved() {
                    Some(data) => data,
                    None => {
                        state.view_state = ViewState::TopLevelSeries(*top_level_series_state);
                        return self.render(area, buf, state);
                    }
                };

                let items = series_data.pairings.iter().flat_map(|(format, seasons)| {
                    seasons.into_iter().map(|(season, season_data)| {
                        Self::rendered_single_series_format(*format, *season, season_data)
                    })
                });

                (widgets::List::new(items), current_list_state)
            }
        };

        // wrap the list selection index around the top or bottom
        if list_state.wrap_to_last {
            list_state.select_last();
            list_state.wrap_to_last = false;
        } else if let Some(sel_idx) = list_state.selected()
            && sel_idx >= list.len()
        {
            list_state.select_first();
        }

        if let Some(block) = self.block {
            list = list.block(block);
        }

        list = list
            .highlight_symbol(Line::styled(">", Style::default().light_cyan()))
            .highlight_style(Style::default().bold());

        StatefulWidget::render(list, area, buf, list_state);
    }
}

#[derive(Debug)]
pub struct State {
    pub view_state: ViewState,
}

impl State {
    pub fn new() -> Self {
        Self {
            view_state: ViewState::TopLevelSeries(ListState::default()),
        }
    }

    pub fn select_series_formats(&mut self) {
        match &mut self.view_state {
            ViewState::TopLevelSeries(existing_state) => {
                let selected_series = match existing_state.selected_mut() {
                    Some(idx) => *idx,
                    // ensure there's a selected series, since we'll be viewing
                    // the formats of one (which implies selection)
                    sel_val @ None => {
                        *sel_val = Some(0);
                        0
                    }
                };

                let mut new_state = widgets::ListState::default();
                new_state.select(Some(0));

                self.view_state = ViewState::SingleSeriesFormats {
                    selected_series,
                    current_list_state: new_state.into(),
                    top_level_series_state: *existing_state,
                }
            }
            ViewState::SingleSeriesFormats { .. } => (),
        }
    }

    pub fn select_top_level_series(&mut self) {
        match self.view_state {
            ViewState::TopLevelSeries(_) => (),
            ViewState::SingleSeriesFormats {
                top_level_series_state,
                ..
            } => self.view_state = ViewState::TopLevelSeries(top_level_series_state),
        }
    }

    pub fn select_next(&mut self) {
        self.current_list_state_mut().select_next();
    }

    pub fn select_previous(&mut self) {
        let state = self.current_list_state_mut();

        if let Some(0) = state.selected() {
            state.wrap_to_last = true;
        } else {
            state.select_previous();
        }
    }

    fn current_list_state_mut(&mut self) -> &mut ListState {
        match &mut self.view_state {
            ViewState::TopLevelSeries(state) => state,
            ViewState::SingleSeriesFormats {
                current_list_state, ..
            } => current_list_state,
        }
    }
}

#[derive(Debug)]
pub enum ViewState {
    TopLevelSeries(ListState),
    SingleSeriesFormats {
        selected_series: usize,
        current_list_state: ListState,
        top_level_series_state: ListState,
    },
}

#[derive(Copy, Clone, Debug, Default, Deref, DerefMut)]
pub struct ListState {
    /// `ratatui` uses a `usize` for its `ListState` selection index,
    /// so we need a way to know when the index needs to wrap
    /// around from the top since negative values can't be used
    wrap_to_last: bool,
    #[deref]
    #[deref_mut]
    state: widgets::ListState,
}

impl From<widgets::ListState> for ListState {
    fn from(value: widgets::ListState) -> Self {
        Self {
            wrap_to_last: false,
            state: value,
        }
    }
}
