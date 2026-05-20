//! Render composition for the main chat widget surface.

use super::*;

impl ChatWidget {
    pub(super) fn as_renderable(&self) -> RenderableItem<'_> {
        status_header::as_renderable(self)
    }
}

struct BottomPaneComposerReserveRenderable<'a> {
    bottom_pane: &'a BottomPane,
    right_reserve: u16,
}

impl Renderable for BottomPaneComposerReserveRenderable<'_> {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.bottom_pane
            .render_with_composer_right_reserve(area, buf, self.right_reserve);
    }

    fn desired_height(&self, width: u16) -> u16 {
        self.bottom_pane
            .desired_height_with_composer_right_reserve(width, self.right_reserve)
    }

    fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        self.bottom_pane
            .cursor_pos_with_composer_right_reserve(area, self.right_reserve)
    }

    fn cursor_style(&self, area: Rect) -> crossterm::cursor::SetCursorStyle {
        self.bottom_pane
            .cursor_style_with_composer_right_reserve(area, self.right_reserve)
    }
}

struct TranscriptAreaRenderable<'a> {
    child: &'a dyn HistoryCell,
    top: u16,
    right: u16,
}

impl Renderable for TranscriptAreaRenderable<'_> {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        let area = self.child_area(area);
        let lines = self.child.display_lines(area.width);
        let paragraph = Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false });
        let y = if area.height == 0 {
            0
        } else {
            let overflow = paragraph
                .line_count(area.width)
                .saturating_sub(usize::from(area.height));
            u16::try_from(overflow).unwrap_or(u16::MAX)
        };
        Clear.render(area, buf);
        paragraph.scroll((y, 0)).render(area, buf);
    }

    fn desired_height(&self, width: u16) -> u16 {
        let child_width = width.saturating_sub(self.right).max(1);
        HistoryCell::desired_height(self.child, child_width) + self.top
    }
}

impl TranscriptAreaRenderable<'_> {
    fn child_area(&self, area: Rect) -> Rect {
        let y = area.y.saturating_add(self.top);
        let height = area.height.saturating_sub(self.top);
        Rect::new(
            area.x,
            y,
            area.width.saturating_sub(self.right).max(1),
            height,
        )
    }
}

impl Renderable for ChatWidget {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.as_renderable().render(area, buf);
        self.last_rendered_width.set(Some(area.width as usize));
    }

    fn desired_height(&self, width: u16) -> u16 {
        self.as_renderable().desired_height(width)
    }

    fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        self.as_renderable().cursor_pos(area)
    }

    fn cursor_style(&self, area: Rect) -> crossterm::cursor::SetCursorStyle {
        self.as_renderable().cursor_style(area)
    }
}
