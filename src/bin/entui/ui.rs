use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect, Layout, Direction, Constraint},
    style::{Color, Stylize},
    widgets::{Block, BorderType, Paragraph, Widget},
};

use crate::app::App;

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // LAYOUT
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Percentage(100),
                // Constraint::Percentage(50),
            ])
            .split(area);

        // BLOCK 0 - ISSUE LIST
        self.issues_list.render(layout[0], buf);
    }
}
