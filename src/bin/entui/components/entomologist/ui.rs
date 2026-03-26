use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{
        Block, List, ListDirection, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
        StatefulWidget, Widget, WidgetRef, Wrap,
    },
};

use strum::{EnumIter, IntoEnumIterator};

use entomologist::issue::{Issue, State};

use tui_widget_list::{ListBuilder, ListView};

use crate::components::entomologist::{
    CommentEntry, CommentsList, Entry, IssuesList, StateSelectorWidget,
};

// have to do this since neither Widget nor Issue were defined in this crate
impl Widget for &Entry {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let block = Block::bordered().title("ENTRY");
        let text = format!(
            "TITLE: {}\nID: {}\nSTATE: {}",
            self.title, self.id, self.state
        );
        let text = if !self.tags.is_empty() {
            format!("{text}\nTAGS: {:?}", self.tags)
        } else {
            text
        };
        let text = match &self.assignee {
            Some(assignee) => format!("{text}\nASSIGNEE: {}", assignee),
            None => format!("{text}\nASSIGNEE: NONE"),
        };
        let text = format!("{text}\n\nDESCRIPTION:\n{}", self.description);
        let pg = Paragraph::new(text).wrap(Wrap { trim: true }).block(block);

        pg.render(area, buf);
    }
}

impl Widget for &StateSelectorWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let list = State::iter()
            .map(|state| {
                let string = Into::<&'static str>::into(state);
                ListItem::new(string)
            })
            .collect::<List>()
            .style(Style::new().white())
            .highlight_style(Style::new().bg(Color::White).fg(Color::Black))
            .direction(ListDirection::TopToBottom);
        let state = &mut *self.list_state.borrow_mut();
        StatefulWidget::render(list, area, buf, state);

        let state_list: Vec<_> = State::iter().collect();
        match state.selected() {
            Some(index) => self.selected_state.replace(state_list[index].clone()),
            None => self.selected_state.replace(State::New {}),
        };
    }
}

impl Widget for &IssuesList {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Fill(1), Constraint::Length(20)])
            .split(area);

        // ISSUE LIST
        let issue_list_area = layout[0];

        let issues_list_widget = self
            .list_items
            .iter()
            .collect::<List>()
            .block(Block::bordered().title("ISSUES"))
            .style(Style::new().white())
            .highlight_style(Style::new().bg(Color::White).fg(Color::Black))
            .direction(ListDirection::TopToBottom);

        // wooooooooof :(
        let state = &mut *self.list_state.borrow_mut();
        StatefulWidget::render(issues_list_widget, issue_list_area, buf, state);

        match state.selected() {
            Some(index) => match &self.list_items[index] {
                super::IssueListItem::Heading(_) => self.selected_issue.replace(None),
                super::IssueListItem::Issue(issue) => self
                    .selected_issue
                    .replace(Some(Entry::new_from_id_issue(&issue.id, issue))),
            },
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
                let pg = Paragraph::new(text)
                    .block(preview_block)
                    .alignment(Alignment::Center);
                pg.render(preview_area, buf);
            }
        }
    }
}

impl Widget for CommentEntry {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let title_text = format!("{} - {}", self.author, self.creation_time);
        let block = Block::bordered().title(title_text);
        let text = format!("{}", self.description);
        let pg = Paragraph::new(text).wrap(Wrap { trim: true }).block(block);
        pg.render(area, buf);
    }
}

impl WidgetRef for CommentsList {
    fn render_ref(&self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let builder = ListBuilder::new(|context| {
            let item = CommentEntry::new_from_comment(&self.comments[context.index]);

            // annoyingly we have to do this because we need the size pre-calculated
            // we could probably set the area in the item as part of the builder
            // or maybe some other way of moving this information so we don't
            // calculate it twice, but for now, lazy
            let title_text = format!("{} - {}", item.author, item.creation_time);
            let block = Block::bordered().title(title_text);
            let text = format!("{}", item.description);
            let pg = Paragraph::new(text).wrap(Wrap { trim: true }).block(block);

            let main_axis_size = pg.line_count(area.width) as u16;

            (item, main_axis_size)
        });

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("┐"))
            .end_symbol(Some("┘"));
        // let list = ListView::new(builder, 50)
        //     .block(Block::default().borders(Borders::ALL))

        let list_v = ListView::new(builder, self.comments.len()).scrollbar(scrollbar);
        // let state = self.list_state.borrow_mut();

        let state = &mut *self.list_state.borrow_mut();
        StatefulWidget::render(list_v, area, buf, state);
    }
}

// TODO:
// sort comments list in reverse order, or set scroll to bottom
// filter out issues that are done / break other issues up better
// allow scrolling through comments / issue text
