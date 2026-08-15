use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Block, BlockExt, Widget},
};

use crate::{series, tui};

mod paired;
mod unpaired;

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
            Some(series::RemoteSeasonPairing::Paired(season)) => paired::render(
                &season.remote_info,
                season.in_sync,
                self.cover_image,
                inner_area,
                buf,
            ),
            Some(series::RemoteSeasonPairing::Unpaired(_)) => {
                unpaired::render(inner_area, buf);
            }
            None => {}
        }
    }
}
