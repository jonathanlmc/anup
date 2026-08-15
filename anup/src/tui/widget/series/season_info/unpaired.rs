use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Flex, Layout, Rect},
    style::Style,
    text::Text,
    widgets::{Clear, Paragraph, Widget, Wrap},
};

pub fn render(area: Rect, buf: &mut Buffer) {
    render_info_text_with_hint(
        Text::styled(
            "This season has no remote pairing.",
            Style::default().bold(),
        ),
        Text::styled(
            "Episodes could not be matched to a remote series.",
            Style::default().dark_gray(),
        ),
        area,
        buf,
    );
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

    Clear.render(info_text_area, buf);
    info_text.render(info_text_area, buf);
}
