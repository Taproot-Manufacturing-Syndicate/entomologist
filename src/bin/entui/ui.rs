use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Widget, WidgetRef},
};

use crate::app::{App, ViewState};

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // LAYOUT
        match &self.view_state {
            ViewState::Overview => {
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
            ViewState::Issue { issue, comments } => {
                let layout = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(area);

                issue.render(layout[0], buf);
                comments.render_ref(layout[1], buf);
            }
        }
    }
}
