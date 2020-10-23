use tui::style::{Color, Style};
use tui::layout::{Rect, Direction, Layout, Constraint, Alignment};
use std::iter;
use tui::widgets::{Paragraph, Wrap, Borders, Block};
use chrono::{Duration, NaiveDate, Datelike};
use crate::http_client::timetrack::Timetrack;

pub const PURPLE: Color = Color::Rgb(102, 0, 255);
pub const WHITE: Color = Color::Rgb(255, 255, 255);
pub const BLACK: Color = Color::Rgb(0, 0, 0);
pub const GREY: Color = Color::Rgb(73, 73, 73);
pub const PINK: Color = Color::Rgb(233, 76, 112);
pub const BG_GREY: Color = Color::Rgb(245, 245, 245);

pub const WEEKDAYS: [&str; 7] = [
    "mandag", "tirsdag", "onsdag", "torsdag", "fredag", "lørdag", "søndag",
];

pub trait RectSplit {
    fn split_vertical(self) -> Vec<Rect>;
    fn split_horizontal(self) -> Vec<Rect>;
    fn split_into_equal_parts_horizontal(self, number_of_parts: usize) -> Vec<Rect>;
    fn split_into_equal_parts(self, numer_of_parts: usize, direction: Direction) -> Vec<Rect>;
    fn split_into_equal_parts_vertical(self, number_of_parts: usize) -> Vec<Rect>;
}

impl RectSplit for Rect {
    fn split_vertical(self) -> Vec<Rect> {
        Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(self)
    }

    fn split_horizontal(self) -> Vec<Rect> {
        Layout::default()
            .direction(Direction::Horizontal)
            .margin(1)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(self)
    }

    fn split_into_equal_parts(self, number_of_parts: usize, direction: Direction) -> Vec<Rect> {
        let mut constraints: Vec<Constraint> = vec![];
        constraints.extend(
            iter::repeat(Constraint::Percentage((100 / number_of_parts) as u16))
                .take(number_of_parts),
        );
        Layout::default()
            .direction(direction)
            .margin(1)
            .constraints(constraints.as_ref())
            .margin(1)
            .split(self)
    }

    fn split_into_equal_parts_horizontal(self, number_of_parts: usize) -> Vec<Rect> {
        self.split_into_equal_parts(number_of_parts, Direction::Horizontal)
    }

    fn split_into_equal_parts_vertical(self, number_of_parts: usize) -> Vec<Rect> {
        self.split_into_equal_parts(number_of_parts, Direction::Vertical)
    }
}

pub fn paragraph(text: &str, alignment: Alignment, color: Color) -> Paragraph {
    Paragraph::new(text)
        .style(Style::default().fg(color))
        .alignment(alignment)
        .wrap(Wrap { trim: true })
}

pub fn bg_paragraph(text: &str, alignment: Alignment, color: Color, bg_color: Color) -> Paragraph {
    Paragraph::new(text)
        .style(Style::default().fg(color).bg(bg_color))
        .alignment(alignment)
        .wrap(Wrap { trim: true })
}

pub fn white_text(text: &str, alignment: Alignment) -> Paragraph {
    paragraph(text, alignment, WHITE)
}

pub fn black_text(text: &str, alignment: Alignment) -> Paragraph {
    paragraph(text, alignment, BLACK)
}

pub fn grey_text(text: &str, alignment: Alignment) -> Paragraph {
    paragraph(text, alignment, GREY)
}

pub fn today_text(text: &str, alignment: Alignment) -> Paragraph {
    Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(PURPLE)),
        )
        .style(Style::default().fg(PINK))
        .alignment(alignment)
        .wrap(Wrap { trim: true })
}

pub fn week_for_current_day(current_date: NaiveDate) -> Vec<NaiveDate> {
    let days_from_monday = current_date.weekday().num_days_from_monday();
    let monday = current_date - Duration::days(1) * days_from_monday as i32;
    vec![
        monday,
        monday + Duration::days(1),
        monday + Duration::days(2),
        monday + Duration::days(3),
        monday + Duration::days(4),
        monday + Duration::days(5),
        monday + Duration::days(6),
    ]
}

pub fn find_timetracking_for_day_of_week<'a>(
    day_of_week: usize,
    time_trackings: &'a Vec<&Timetrack>,
) -> Option<&'a Timetrack> {
    time_trackings
        .iter()
        .find(|&&tracking| tracking.date.weekday() as usize == day_of_week)
        .map(|&timetrack| timetrack)
}
