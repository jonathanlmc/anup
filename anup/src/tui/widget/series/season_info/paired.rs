use std::borrow::Cow;

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Offset, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};

use crate::tui;

use super::COVER_IMAGE_RESIZE_METHOD;

/// Build a `label: value` line, falling back to `label: N/A` when the value is absent.
fn build_optional_stat_line<'a, T, F>(
    label: &'static str,
    value: Option<T>,
    format: impl FnOnce(T) -> F,
) -> Line<'a>
where
    F: Into<Cow<'a, str>>,
{
    let mut line = Line::from(Span::styled(label, Style::default().bold()));

    let span = value.map_or_else(
        || Span::styled("N/A", Style::default().dark_gray().italic()),
        |value| Span::styled(format(value), Style::default()),
    );

    line.push_span(span);
    line
}

/// Basic series info shown at the top of the panel, next to the cover image.
mod info {
    use super::*;

    pub fn build_text(anime: &anime::Info) -> Text<'_> {
        let mut info_text = Text::default();

        info_text.push_line(build_optional_stat_line(
            "Episodes: ",
            anime.episodes,
            |count| count.to_string(),
        ));

        info_text.push_line(build_id_line(anime.id));

        info_text
    }

    fn build_id_line(id: anime::Id) -> Line<'static> {
        let bold = Style::default().bold();

        let mut line = Line::from(Span::styled("ID [", bold));
        line.push_span(Span::styled(id.source_name(), bold));
        line.push_span(Span::styled("]: ", bold));
        line.push_span(Span::styled(id.plain().to_string(), Style::default()));

        line
    }
}

/// The user's list entry fields, split into a relevant and less relevant half.
mod stats {
    use super::*;

    pub fn build_relevant_text<'a>(in_sync: bool, list_entry: &anime::UserListEntry) -> Text<'a> {
        let mut info_text = Text::default();

        info_text.push_line(build_optional_stat_line(
            "Progress: ",
            list_entry.progress,
            |progress| progress.to_string(),
        ));

        info_text.push_line(build_optional_stat_line(
            "Status: ",
            list_entry.status,
            anime::UserStatus::display_str,
        ));

        info_text.push_line(build_optional_stat_line(
            "Started: ",
            list_entry.started_at,
            |date| date.to_string(),
        ));

        info_text.push_line(build_sync_status_line(in_sync));

        info_text
    }

    pub fn build_other_text<'a>(list_entry: &anime::UserListEntry) -> Text<'a> {
        let mut info_text = Text::default();

        info_text.push_line(build_optional_stat_line(
            "Score: ",
            list_entry.score,
            |score| score.to_string(),
        ));

        info_text.push_line(build_repeat_line(list_entry.repeat));

        info_text.push_line(build_optional_stat_line(
            "Completed: ",
            list_entry.completed_at,
            |date| date.to_string(),
        ));

        info_text.push_line(build_optional_stat_line(
            "Updated: ",
            list_entry
                .updated_at
                .map(|ts| ts.to_zoned(crate::SYSTEM_TZ.clone()).date()),
            |date| date.to_string(),
        ));

        info_text.push_line(build_optional_stat_line(
            "Added: ",
            list_entry
                .created_at
                .map(|ts| ts.to_zoned(crate::SYSTEM_TZ.clone()).date()),
            |date| date.to_string(),
        ));

        info_text
    }

    fn build_sync_status_line(in_sync: bool) -> Line<'static> {
        let mut line = Line::styled("State: ", Style::default().bold());

        let status = if in_sync {
            Span::styled("Synced", Style::default().green())
        } else {
            Span::styled("Unsynced", Style::default().red())
        };

        line.push_span(status);
        line
    }

    fn build_repeat_line(repeat: Option<u32>) -> Line<'static> {
        let mut line = Line::styled("Rewatched: ", Style::default().bold());

        let span = match repeat {
            Some(0) | None => Span::styled("0x", Style::default()),
            Some(count) => Span::styled(format!("{count}x"), Style::default()),
        };

        line.push_span(span);
        line
    }
}

mod cover_image {
    use ratatui::widgets::StatefulWidget;

    use super::*;

    pub fn render(
        state: Option<&mut tui::image_protocol::ProtocolState>,
        area: Rect,
        buf: &mut Buffer,
    ) {
        match state {
            Some(tui::image_protocol::ProtocolState::Loading) | None => {
                render_placeholder(false, area, buf);
            }
            Some(tui::image_protocol::ProtocolState::Failed) => {
                render_placeholder(true, area, buf);
            }
            Some(tui::image_protocol::ProtocolState::Loaded(protocol)) => {
                ratatui_image::StatefulImage::new()
                    .resize(COVER_IMAGE_RESIZE_METHOD)
                    .render(area, buf, &mut **protocol);
            }
        }
    }

    fn render_placeholder(failed: bool, area: Rect, buf: &mut Buffer) {
        let style = if failed {
            Style::default().red()
        } else {
            Style::default().dark_gray()
        };

        let cover_block = Block::bordered().border_style(style);
        let inner_area = cover_block.inner(area);
        cover_block.render(area, buf);

        let placeholder_text = if failed { "Cover Error" } else { "Loading.." };

        let placeholder = Paragraph::new(Text::styled(placeholder_text, style.italic()))
            .centered()
            .wrap(Wrap { trim: false });

        placeholder.render(inner_area, buf);
    }
}

pub fn render(
    anime: &anime::Info,
    in_sync: bool,
    cover_image: Option<&mut tui::image_protocol::ProtocolState>,
    area: Rect,
    buf: &mut Buffer,
) {
    // todo: implement episode playback
    let hint_para = Paragraph::new(Text::styled(
        "Press enter to play the next episode. (WIP)",
        Style::default().dark_gray(),
    ))
    .wrap(Wrap { trim: false })
    .centered();

    let num_hint_lines = hint_para.line_count(area.width);

    let [content_area, hint_area] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(num_hint_lines as u16),
    ])
    .areas(area);

    hint_para.render(hint_area, buf);
    Clear.render(content_area, buf);

    let [top_area, center_line_area, bottom_area] = Layout::vertical([
        Constraint::Percentage(30),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .areas(content_area);

    Block::new()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(Color::DarkGray))
        .render(center_line_area, buf);

    render_top(anime, cover_image, top_area, buf);
    render_bottom(anime, in_sync, bottom_area, buf);
}

/// Renders the cover image alongside the basic series info.
fn render_top(
    anime: &anime::Info,
    cover_image: Option<&mut tui::image_protocol::ProtocolState>,
    area: Rect,
    buf: &mut Buffer,
) {
    let [cover_area, _spacer, series_info_area] = Layout::horizontal([
        Constraint::Percentage(20),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .areas(area);

    let info_para = Paragraph::new(info::build_text(anime)).wrap(Wrap { trim: false });
    info_para.render(series_info_area, buf);

    cover_image::render(cover_image, cover_area, buf);
}

/// Renders the user's list entry stats, or a message indicating the series isn't on their list.
fn render_bottom(anime: &anime::Info, in_sync: bool, area: Rect, buf: &mut Buffer) {
    let content_area = area.intersection(area.offset(Offset::new(1, 0)));

    match anime.user_list_entry.as_ref() {
        Some(list_entry) => render_list_entry_stats(in_sync, list_entry, content_area, buf),
        None => render_not_on_list_message(content_area, buf),
    }
}

fn render_list_entry_stats(
    in_sync: bool,
    list_entry: &anime::UserListEntry,
    area: Rect,
    buf: &mut Buffer,
) {
    const LONGEST_SPAN: u16 = ("Completed: ".len() + "0000-00-00".len()) as u16;

    let [left_stats_area, _, right_stats_area] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Fill(1),
        Constraint::Length(LONGEST_SPAN),
    ])
    .areas(area);

    let relevant_stats_para =
        Paragraph::new(stats::build_relevant_text(in_sync, list_entry)).wrap(Wrap { trim: false });

    let other_stats_para =
        Paragraph::new(stats::build_other_text(list_entry)).wrap(Wrap { trim: false });

    relevant_stats_para.render(left_stats_area, buf);
    other_stats_para.render(right_stats_area, buf);
}

fn render_not_on_list_message(area: Rect, buf: &mut Buffer) {
    let message = Paragraph::new(Text::styled(
        "Series is not on your anime list.",
        Style::default().bold(),
    ))
    .wrap(Wrap { trim: false })
    .centered();

    let num_lines = message.line_count(area.width);
    let message_area = area.centered_vertically(Constraint::Length(num_lines as u16));

    message.render(message_area, buf);
}
