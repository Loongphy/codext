//! Render composition for the main chat widget surface.

use super::*;

impl ChatWidget {
    pub(super) fn as_renderable(&self) -> RenderableItem<'_> {
        let active_cell_right_reserve = self.ambient_pet_wrap_reserved_cols();
        let active_cell_renderable = match &self.transcript.active_cell {
            Some(cell) => RenderableItem::Owned(Box::new(TranscriptAreaRenderable {
                child: cell.as_ref(),
                top: 1,
                right: active_cell_right_reserve,
            })),
            None => RenderableItem::Owned(Box::new(())),
        };
        let active_hook_cell_renderable = match &self.active_hook_cell {
            Some(cell) if cell.should_render() => {
                RenderableItem::Owned(Box::new(TranscriptAreaRenderable {
                    child: cell,
                    top: 1,
                    right: active_cell_right_reserve,
                }))
            }
            _ => RenderableItem::Owned(Box::new(())),
        };
        let mut flex = FlexRenderable::new();
        flex.push(/*flex*/ 1, active_cell_renderable);
        flex.push(/*flex*/ 0, active_hook_cell_renderable);
        let bottom_pane_renderable =
            RenderableItem::Owned(Box::new(BottomPaneComposerReserveRenderable {
                chat_widget: self,
                right_reserve: active_cell_right_reserve,
            }));
        let bottom_section_renderable = bottom_pane_renderable.inset(Insets::tlbr(
            /*top*/ 1, /*left*/ 0, /*bottom*/ 0, /*right*/ 0,
        ));
        flex.push(/*flex*/ 0, bottom_section_renderable);
        RenderableItem::Owned(Box::new(flex))
    }
}

struct BottomPaneComposerReserveRenderable<'a> {
    chat_widget: &'a ChatWidget,
    right_reserve: u16,
}

impl Renderable for BottomPaneComposerReserveRenderable<'_> {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.chat_widget
            .bottom_pane
            .render_with_composer_right_reserve_and_header(
                area,
                buf,
                self.right_reserve,
                super::status_header::renderable(self.chat_widget),
            );
    }

    fn desired_height(&self, width: u16) -> u16 {
        self.chat_widget
            .bottom_pane
            .desired_height_with_composer_right_reserve_and_header(
                width,
                self.right_reserve,
                super::status_header::renderable(self.chat_widget),
            )
    }

    fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        self.chat_widget
            .bottom_pane
            .cursor_pos_with_composer_right_reserve_and_header(
                area,
                self.right_reserve,
                super::status_header::renderable(self.chat_widget),
            )
    }

    fn cursor_style(&self, area: Rect) -> crossterm::cursor::SetCursorStyle {
        self.chat_widget
            .bottom_pane
            .cursor_style_with_composer_right_reserve_and_header(
                area,
                self.right_reserve,
                super::status_header::renderable(self.chat_widget),
            )
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
