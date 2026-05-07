use std::collections::VecDeque;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::Text,
    widgets::{Block, Paragraph, Wrap},
};

pub struct Log<'a> {
    lines: &'a VecDeque<String>,
    block: Option<Block<'a>>,
    scroll_offset: usize,
}

impl<'a> Log<'a> {
    pub fn new(lines: &'a VecDeque<String>) -> Self {
        Self {
            lines,
            block: None,
            scroll_offset: 0,
        }
    }

    pub fn scroll(mut self, offset: usize) -> Self {
        self.scroll_offset = offset;
        self
    }
}

impl<'a> ratatui::widgets::Widget for Log<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        use ansi_to_tui::IntoText;

        let inner_area = if let Some(block) = self.block.as_ref() {
            block.inner(area)
        } else {
            area
        };

        let lines = {
            let offset = self.scroll_offset;

            let visible_range = offset.saturating_sub(inner_area.height as usize)
                ..offset.max(inner_area.height as usize);

            self.lines
                .range(visible_range)
                .map(|line| line.as_str())
                .collect::<String>()
        };

        let text = match lines.to_text() {
            Ok(text) => text,
            Err(_) => Text::from(lines),
        };

        let mut list = Paragraph::new(text).wrap(Wrap { trim: false });
        let num_lines = list.line_count(inner_area.width);

        list = list.scroll(((num_lines as u16).saturating_sub(inner_area.height), 0));

        if let Some(block) = self.block {
            list = list.block(block);
        }

        list.render(area, buf);
    }
}
