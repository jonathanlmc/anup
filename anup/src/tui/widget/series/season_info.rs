use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Flex, Layout, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, BlockExt, Clear, Paragraph, Widget, Wrap},
};

use crate::{series, tui};

const COVER_IMAGE_RESIZE_METHOD: ratatui_image::Resize =
    ratatui_image::Resize::Scale(Some(ratatui_image::FilterType::CatmullRom));

pub struct SeasonInfo<'a> {
    pairing: Option<&'a series::RemoteSeasonPairing>,
    cover_image: Option<&'a mut tui::image_protocol::ProtocolState>,
    block: Option<Block<'a>>,
}

impl<'a> SeasonInfo<'a> {
    pub const fn new(
        pairing: Option<&'a series::RemoteSeasonPairing>,
        cover_image: Option<&'a mut tui::image_protocol::ProtocolState>,
    ) -> Self {
        Self {
            pairing,
            cover_image,
            block: None,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }
}

impl Widget for SeasonInfo<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let inner_area = self.block.inner_if_some(area);
        self.block.render(area, buf);

        match self.pairing {
            Some(series::RemoteSeasonPairing::Paired(season)) => paired_season_info::render(
                &season.remote_info,
                season.in_sync,
                self.cover_image,
                inner_area,
                buf,
            ),
            Some(series::RemoteSeasonPairing::Unpaired(_)) => {
                unpaired_season_info::render(inner_area, buf);
            }
            None => {}
        }
    }
}

mod paired_season_info {
    use ratatui::{
        layout::Offset,
        style::Color,
        widgets::{Borders, StatefulWidget},
    };

    use super::*;

    fn build_info_text(anime: &anime::Anime) -> Text<'_> {
        let mut info_text = Text::default();

        info_text.push_line(build_episodes_line(anime.episodes));
        info_text.push_line(build_anilist_id_line(anime.id.anilist));

        info_text
    }

    fn build_episodes_line(episodes: Option<u32>) -> Line<'static> {
        let mut line = Line::from(Span::styled("Episodes: ", Style::default().bold()));

        match episodes {
            Some(count) => {
                line.push_span(Span::styled(count.to_string(), Style::default()));
            }
            None => {
                line.push_span(Span::styled(
                    "Unknown",
                    Style::default().dark_gray().italic(),
                ));
            }
        }

        line
    }

    fn build_anilist_id_line(anilist_id: Option<anime::AnimeID>) -> Line<'static> {
        let mut line = Line::from(Span::styled("ID [AniList]: ", Style::default().bold()));

        match anilist_id {
            Some(id) => {
                line.push_span(Span::styled(id.to_string(), Style::default()));
            }
            None => {
                line.push_span(Span::styled("N/A", Style::default().dark_gray().italic()));
            }
        }

        line
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

    fn build_user_stats_text<'a>(in_sync: bool) -> Text<'a> {
        let mut info_text = Text::default();

        // todo: add stats from user list
        info_text.push_line(build_sync_status_line(in_sync));

        info_text
    }

    pub fn render(
        anime: &anime::Anime,
        in_sync: bool,
        cover_image: Option<&mut tui::image_protocol::ProtocolState>,
        area: Rect,
        buf: &mut Buffer,
    ) {
        // todo: implement episode playback
        let hint_text = Paragraph::new(Text::styled(
            "Press enter to play the next episode. (WIP)",
            Style::default().dark_gray(),
        ))
        .wrap(Wrap { trim: false })
        .centered();

        let num_hint_lines = hint_text.line_count(area.width);

        let [content_area, hint_area] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(num_hint_lines as u16),
        ])
        .areas(area);

        hint_text.render(hint_area, buf);
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

        let [cover_area, _spacer, series_info_area] = Layout::horizontal([
            Constraint::Percentage(20),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .areas(top_area);

        let info_text = build_info_text(anime);
        let info_para = Paragraph::new(info_text).wrap(Wrap { trim: false });

        info_para.render(series_info_area, buf);

        let user_stats_para =
            Paragraph::new(build_user_stats_text(in_sync)).wrap(Wrap { trim: false });

        user_stats_para.render(
            bottom_area.intersection(bottom_area.offset(Offset::new(1, 0))),
            buf,
        );

        match cover_image {
            Some(tui::image_protocol::ProtocolState::Loading) | None => {
                render_cover_image_placeholder(false, cover_area, buf);
            }
            Some(tui::image_protocol::ProtocolState::Failed) => {
                render_cover_image_placeholder(true, cover_area, buf);
            }
            Some(tui::image_protocol::ProtocolState::Loaded(protocol)) => {
                ratatui_image::StatefulImage::new()
                    .resize(COVER_IMAGE_RESIZE_METHOD)
                    .render(cover_area, buf, &mut **protocol);
            }
        }
    }

    fn render_cover_image_placeholder(failed: bool, area: Rect, buf: &mut Buffer) {
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

mod unpaired_season_info {
    use super::*;

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
}
