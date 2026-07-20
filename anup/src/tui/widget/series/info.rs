use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Flex, Layout, Offset, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, BlockExt, Clear, Paragraph, Widget, Wrap},
};

use crate::{series, tui::state};

pub struct SeriesInfo<'a> {
    entry: Option<&'a state::series_list::Entry>,
    block: Option<Block<'a>>,
}

impl<'a> SeriesInfo<'a> {
    pub fn new(entry: Option<&'a state::series_list::Entry>) -> Self {
        Self { entry, block: None }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    fn render_info_text_with_hint(info_text: Text, hint_text: Text, area: Rect, buf: &mut Buffer) {
        let hint_text = Paragraph::new(hint_text)
            .wrap(Wrap { trim: false })
            .centered();

        let num_hint_lines = hint_text.line_count(area.width);

        let [hint_area] = Layout::vertical([Constraint::Length(num_hint_lines as u16)])
            .flex(Flex::End)
            .areas(area);

        hint_text.render(hint_area, buf);

        let info_text = Paragraph::new(info_text)
            .wrap(Wrap { trim: false })
            .centered();

        let num_info_lines = info_text.line_count(area.width);

        let info_text_area = area.centered_vertically(Constraint::Length(num_info_lines as u16));

        // render over the hint text if we are very limited on space
        Clear.render(info_text_area, buf);

        info_text.render(info_text_area, buf);
    }

    fn render_detected_series(area: Rect, buf: &mut Buffer) {
        Self::render_info_text_with_hint(
            Text::styled(
                "A series folder was detected, but it has not yet been scanned for episodes.",
                Style::new().bold(),
            ),
            Text::styled(
                "It is queued for automatic scanning.",
                Style::new().dark_gray(),
            ),
            area,
            buf,
        );
    }

    fn render_scanning_series(area: Rect, buf: &mut Buffer) {
        Self::render_info_text_with_hint(
            Text::styled(
                "This series folder is currently being scanned for episodes.",
                Style::new().bold(),
            ),
            Text::styled(
                "Once complete, it will start pairing to the configured service.",
                Style::new().dark_gray(),
            ),
            area,
            buf,
        );
    }
}

impl ratatui::widgets::Widget for SeriesInfo<'_> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        use state::series_list::EntryState;

        let inner_area = self.block.inner_if_some(area);
        self.block.render(area, buf);

        match self.entry.map(|e| &e.state) {
            Some(EntryState::Detected) => Self::render_detected_series(inner_area, buf),
            Some(EntryState::Scanning) => Self::render_scanning_series(inner_area, buf),
            Some(EntryState::Resolving(series)) => local_series_info::render(
                series,
                "Series Is Currently Resolving",
                "The series will be playable once it has finished resolving to the \
                configured anime API service.",
                inner_area,
                buf,
            ),
            Some(EntryState::Failure { error, .. }) => {
                failure_entry::render(error, inner_area, buf)
            }
            Some(EntryState::Unresolved(series)) => local_series_info::render(
                series,
                "Series Could Not Be Resolved",
                // todo: implement manual pairing
                "No match was found on the configured anime API service. Manually pair \
                it by pressing Ctrl + P (WIP).",
                inner_area,
                buf,
            ),
            // todo: support remaining variants
            _ => (),
        }
    }
}

mod local_series_info {
    use super::*;

    fn build_info_text<'a>(series: &'a series::LocalRoot) -> Text<'a> {
        let mut info_text = Text::from(Span::styled("Path: ", Style::default().bold()));

        info_text.push_span(Span::styled(
            series.path.to_string_lossy(),
            Style::default().italic(),
        ));

        let format_list_line = {
            let mut line = Line::from(Span::styled("Found formats: ", Style::default().bold()));
            build_format_desc_line(&mut line, &series.episodes);
            line
        };

        info_text.push_line(format_list_line);
        info_text
    }

    pub fn render(
        series: &series::LocalRoot,
        title: &str,
        hint: &str,
        area: Rect,
        buf: &mut Buffer,
    ) {
        let remaining_area = info_panel::render_title(title, Style::default(), area, buf);

        let hint_text = info_panel::build_hint(hint);
        let info_text = build_info_text(series);
        let info_para = Paragraph::new(info_text).wrap(Wrap { trim: false });

        info_panel::layout_and_render(info_para, hint_text, remaining_area, buf);
    }

    fn build_format_desc_line(line: &mut Line, episodes: &series::local::EpisodeMap) {
        for (i, (fmt, eps)) in episodes.iter().enumerate() {
            append_format_desc_entry(line, *fmt, eps.len());

            // separate each entry unless it's the last
            if i < episodes.len().saturating_sub(1) {
                line.push_span(Span::raw(", "));
            }
        }
    }

    fn append_format_desc_entry(line: &mut Line, fmt: series::Format, count: usize) {
        line.push_span(Span::styled(fmt.titlecase_str(), Style::default().cyan()));
        line.push_span(Span::raw(" ("));
        line.push_span(Span::styled(count.to_string(), Style::default().italic()));
        line.push_span(Span::raw(")"));
    }
}

mod info_panel {
    use super::*;

    pub fn render_title(title: &str, style: Style, area: Rect, buf: &mut Buffer) -> Rect {
        let title_para = Paragraph::new(Text::styled(title, style)).centered();

        let [title_area, remaining_area] =
            Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(area);

        title_para.render(title_area, buf);
        remaining_area
    }

    pub fn layout_and_render<'a>(
        info_text: Paragraph<'a>,
        hint_text: Paragraph<'a>,
        area: Rect,
        buf: &mut Buffer,
    ) {
        let offset_area = area.intersection(area.offset(Offset::new(2, 2)));

        let num_hint_lines = hint_text.line_count(offset_area.width);
        let num_info_lines = info_text.line_count(offset_area.width);

        let [info_area, hint_area] = Layout::vertical([
            Constraint::Min(num_info_lines as u16),
            Constraint::Length(num_hint_lines as u16),
        ])
        .areas(offset_area);

        hint_text.render(hint_area, buf);
        // the info text takes priority over the hint text; render over it if we're tight on space
        Clear.render(info_area, buf);
        info_text.render(info_area, buf);
    }

    pub fn build_hint<'a>(text: &'a str) -> Paragraph<'a> {
        Paragraph::new(Text::styled(text, Style::default().dark_gray()))
            .wrap(Wrap { trim: false })
            .centered()
    }
}

mod failure_entry {
    use super::*;

    fn build_info_text<'a>(error: &'a state::series_list::EntryError) -> Text<'a> {
        let mut info_text = Text::from(Span::styled("Type: ", Style::default().bold()));
        info_text.push_span(Span::styled(
            category_name(error),
            Style::default().italic(),
        ));

        let reason_line = {
            let mut line = Line::from(Span::styled("Reason: ", Style::default().bold()));
            line.push_span(Span::styled(error.to_string(), Style::default().italic()));
            line
        };
        info_text.push_line(reason_line);

        info_text
    }

    fn category_name(error: &state::series_list::EntryError) -> &'static str {
        use state::series_list::EntryError::*;

        match error {
            ParseError(_) => "Local Parsing Error",
            Panic(_) => "Internal Error",
            Automatch(_) => "Pairing Error",
        }
    }

    pub fn render(error: &state::series_list::EntryError, area: Rect, buf: &mut Buffer) {
        let remaining_area = info_panel::render_title(
            "Failed to Resolve Series",
            Style::default().red().bold(),
            area,
            buf,
        );

        let info_text = build_info_text(error);
        let info_para = Paragraph::new(info_text).wrap(Wrap { trim: false });

        // todo: implement series processing restarts
        let hint_text = info_panel::build_hint(
            "Restart the series resolving process by pressing Shift + R. (WIP)",
        );

        info_panel::layout_and_render(info_para, hint_text, remaining_area, buf);
    }
}
