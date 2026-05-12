use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Flex, Layout, Rect},
    style::Style,
    text::Text,
    widgets::{Block, BlockExt, Clear, Paragraph, Widget, Wrap},
};

use crate::tui::state;

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

        #[allow(clippy::single_match)]
        match self.entry.map(|e| &e.state) {
            Some(EntryState::Detected) => Self::render_detected_series(inner_area, buf),
            Some(EntryState::Scanning) => Self::render_scanning_series(inner_area, buf),
            // todo: support remaining variants
            _ => (),
        }
    }
}
