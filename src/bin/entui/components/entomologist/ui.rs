use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    widgets::{Block, BorderType, List, ListDirection, ListItem, Paragraph, StatefulWidget, Widget},
};

use entomologist::issue::Issue;

use crate::components::entomologist::{Entry, IssuesList};

fn generate_list_item<'a>(id: &String, issue: &Issue) -> ListItem<'a> {
    let title = issue.title();
    ListItem::new(format!("{title}"))
}

// have to do this since neither Widget nor Issue were defined in this crate
impl Widget for &Entry {
    fn render(self, area: Rect, buf: &mut Buffer)
        where
            Self: Sized {
        let block = Block::bordered().title("PREVIEW");
        let text = format!("TITLE: {}\nID: {}\nSTATE: {}", self.title, self.id, self.state);
        let text = match &self.assignee {
            Some(assignee) => format!("{text}\nASSIGNEE: {}", assignee),
            None => format!("{text}\nASSIGNEE: NONE")
        };
        let text = format!("{text}\n\nDESCRIPTION:\n{}", self.description);
        let pg = Paragraph::new(text).block(block);

        pg.render(area, buf);
    }
}

impl Widget for &IssuesList {
    fn render(self, area: Rect, buf: &mut Buffer)
        where
            Self: Sized {

        let issues_list: Vec<Entry> = self.issues.issues.iter().map(|(id, issue)| Entry::new_from_id_issue(id, issue)).collect();
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Fill(1), Constraint::Length(20)])
            .split(area);

    
        // ISSUE LIST
        let issue_list_area = layout[0];

        let issues_list_widget = self.issues.issues.iter().map(|(id, issue)| generate_list_item(id, issue)).collect::<List>()
            .block(Block::bordered().title("ISSUES"))
            .style(Style::new().white())
            .highlight_style(Style::new().bg(Color::White).fg(Color::Black))
            .direction(ListDirection::TopToBottom);
        
        // wooooooooof :(
        let state = &mut *self.list_state.borrow_mut();
        StatefulWidget::render(issues_list_widget, issue_list_area, buf, state);
        
        match state.selected() {
            Some(index) => self.selected_issue.replace(Some(issues_list[index].clone())),
            None => self.selected_issue.replace(None),
        };

        // ISSUE PREVIEW
        let preview_area = layout[1];
        match &(*self.selected_issue.borrow()) {
            Some(entry) => {
                // .unwrap() as this should never fail and i can't handle an error
                // inside this trait rn (lazy)
                entry.render(preview_area, buf);
            }
            None => {
                let text = "NO ISSUE SELECTED";
                let preview_block = Block::bordered().title("PREVIEW");
                let pg = Paragraph::new(text).block(preview_block).alignment(Alignment::Center);
                pg.render(preview_area, buf);
            }
        }
    }
}