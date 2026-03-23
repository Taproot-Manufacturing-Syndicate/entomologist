use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Clear, Paragraph, Widget, WidgetRef},
};

use crate::app::{App, PopupState, ViewManager, ViewState};

impl Widget for &ViewState {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match &self {
            ViewState::Overview { issue_list } => {
                let layout = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(vec![
                        Constraint::Percentage(100),
                        // Constraint::Percentage(50),
                    ])
                    .split(area);

                // BLOCK 0 - ISSUE LIST
                issue_list.render(layout[0], buf);

                // if *popup {
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

impl Widget for &PopupState {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match &self {
            PopupState::StateSelection => {
                let popup_block = Block::bordered().title("SET STATE");
                let centered_area =
                    area.centered(Constraint::Percentage(60), Constraint::Percentage(20));
                // clears out any background in the area before rendering the popup
                let clear = Clear {};
                clear.render(centered_area, buf);
                // paragraph.render(centered_area, buf);
                // another solution is to use the inner area of the block
                let inner_area = popup_block.inner(centered_area);
                let paragraph = Paragraph::new("Lorem ipsum").block(popup_block);
                paragraph.render(inner_area, buf);
                // frame.render_widget(your_widget, inner_area);
            }
        }
    }
}

impl Widget for &ViewManager {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.view_state.render(area, buf);
        if let Some(popup) = &self.popup_state {
            popup.render(area, buf);
        }
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.view_manager.render(area, buf);
    }
}
