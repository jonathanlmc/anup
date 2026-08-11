use std::collections::VecDeque;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::Text,
    widgets::{Paragraph, Wrap},
};

pub struct Log<'a> {
    lines: &'a VecDeque<String>,
    scroll_offset: usize,
}

impl<'a> Log<'a> {
    pub const fn new(lines: &'a VecDeque<String>) -> Self {
        Self {
            lines,
            scroll_offset: 0,
        }
    }

    pub const fn scroll(mut self, offset: usize) -> Self {
        self.scroll_offset = offset;
        self
    }
}

impl ratatui::widgets::Widget for Log<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        use ansi_to_tui::IntoText;

        let lines = {
            let offset = self.scroll_offset;

            let end_offset = if offset < area.height as usize {
                area.height as usize
            } else {
                0
            };

            let visible_range = offset.saturating_sub(area.height as usize)
                ..(offset + end_offset).min(self.lines.len());

            self.lines
                .range(visible_range)
                .map(std::string::String::as_str)
                .collect::<String>()
        };

        // cannot move `lines` with `map_or_else`
        #[allow(clippy::option_if_let_else)]
        let text = match lines.to_text() {
            Ok(text) => text,
            Err(_) => Text::from(lines),
        };

        let mut list = Paragraph::new(text).wrap(Wrap { trim: false });
        let num_lines = list.line_count(area.width);

        list = list.scroll(((num_lines as u16).saturating_sub(area.height), 0));

        list.render(area, buf);
    }
}
