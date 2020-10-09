use std::io;
use tui::backend::CrosstermBackend;
use tui::Terminal;
use tui::widgets::{Widget, Block, Borders, Paragraph, Wrap};
use tui::layout::{Layout, Constraint, Direction, Rect, Alignment, Margin};
use tui::style::{Style, Color};
use chrono::{NaiveDate, Duration, Weekday, Datelike};
use chrono::Utc;
use std::iter;
use itertools::Itertools;
use crate::http_client::timetrack::Timetrack;
use std::hash::Hash;
use tui::style::Color::White;

const PURPLE:Color = Color::Rgb(102, 0, 255);
const WHITE:Color = Color::Rgb(255, 255, 255);
const BLACK:Color = Color::Rgb(0,0,0);
const GREY:Color = Color::Rgb(73, 73, 73);
const PINK:Color = Color::Rgb(233, 76, 112);
const BG_GREY:Color = Color::Rgb(245, 245, 245);

const WEEKDAYS: [&str; 7] = ["mandag", "tirsdag", "onsdag", "torsdag", "fredag", "lørdag", "søndag"];

trait RectSplit {
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
            .constraints(
                [
                    Constraint::Percentage(50),
                    Constraint::Percentage(50)
                ].as_ref()
            )
            .split(self)
    }

    fn split_horizontal(self) -> Vec<Rect> {
        Layout::default()
            .direction(Direction::Horizontal)
            .margin(1)
            .constraints(
                [
                    Constraint::Percentage(50),
                    Constraint::Percentage(50)
                ].as_ref()
            )
            .split(self)
    }

    fn split_into_equal_parts(self, number_of_parts: usize, direction: Direction) -> Vec<Rect> {
        let mut constraints: Vec<Constraint> = vec![];
        constraints.extend(iter::repeat(Constraint::Percentage((100/number_of_parts) as u16)).take(number_of_parts));
        Layout::default()
            .direction(direction)
            .margin(1)
            .constraints(
                constraints.as_ref()
            )
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

fn paragraph(text: &str, alignment: Alignment, color: Color) -> Paragraph {
    Paragraph::new(text)
        .style(Style::default().fg(color))
        .alignment(alignment)
        .wrap(Wrap { trim: true })
}

fn bgParagraph(text: &str, alignment: Alignment, color: Color, bgColor: Color) -> Paragraph {
    Paragraph::new(text)
        .style(Style::default().fg(color).bg(bgColor))
        .alignment(alignment)
        .wrap(Wrap { trim: true })
}


fn whiteText(text: &str, alignment: Alignment ) -> Paragraph {
    paragraph(text, alignment, WHITE)
}

fn blackText(text: &str, alignment: Alignment ) -> Paragraph {
    paragraph(text, alignment, BLACK)
}

fn greyText(text: &str, alignment: Alignment) -> Paragraph {
    paragraph(text, alignment, GREY)
}

fn pinkText(text: &str, alignment:Alignment) -> Paragraph {
    paragraph(text, alignment, PINK)
}




fn todayText(text: &str, alignment: Alignment) -> Paragraph {
    Paragraph::new(text)
        .block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(PURPLE)))
        .style(Style::default().fg(PINK))
        .alignment(alignment)
        .wrap(Wrap { trim: true })
}

pub struct FloqTTTUI {
    current_date: NaiveDate,
    time_trackings: Vec<Timetrack>
}

fn week_for_current_day(current_date: NaiveDate) -> Vec<NaiveDate> {

    let days_from_monday = current_date.weekday().num_days_from_monday();
    let monday = current_date - Duration::days(1) * days_from_monday as i32;
    vec![monday,
         monday + Duration::days(1),
         monday + Duration::days(2),
         monday + Duration::days(3),
         monday + Duration::days(4),
         monday + Duration::days(5),
         monday + Duration::days(6)
    ]
}


fn find_timetracking_for_day_of_week<'a>(day_of_week: usize, time_trackings: &'a Vec<&Timetrack>) -> Option<&'a Timetrack> {
    time_trackings.iter().find( |&&tracking| {
        tracking.date.weekday() as usize == day_of_week
    }).map(|&timetrack| timetrack)
}


impl FloqTTTUI {
    pub fn new(time_trackings: Vec<Timetrack>) -> Self {
        Self {
            current_date: Utc::now().naive_local().date(),
            time_trackings: time_trackings
        }
    }

    pub fn start(&self) -> Result<(), io::Error> {
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.clear();
        terminal.draw(|f| {
            let window_margin = Margin {vertical: 1, horizontal: 0 };
            let window_size = f.size().inner(&window_margin);
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Percentage(15),
                        Constraint::Percentage(80)
                    ].as_ref()
                )
                .split(window_size);

            let main_layout = chunks[1];

            let background =  Block::default()
                .title("Floq Timetracker")
                .style(Style::default().bg(BG_GREY));


            let header = Block::default()
                .style(Style::default().bg(PURPLE));

            let header_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    [
                        Constraint::Percentage(45),
                        Constraint::Percentage(10),
                        Constraint::Percentage(45)
                    ].as_ref()
                )
                .vertical_margin(1)
                .split(chunks[0]);


            let main_area = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    [
                        Constraint::Percentage(0),
                        Constraint::Percentage(100)
                    ].as_ref()
                )
                .split(main_layout);

            let main_sidebar_layout = main_area[0];
            let main_body_layout = main_area[1];

            let main_body = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Percentage(15),
                        Constraint::Percentage(85)
                    ].as_ref()
                )
                .split(main_body_layout);


            let main_header = main_body[0];

            let main_days_header = main_header.split_into_equal_parts_horizontal(8);


            let antall_timer: f32 = self.time_trackings.iter()
                .map(|day| day.time.num_minutes() as f32 / 60.0)
                .sum();

            let week_description_str = format!("\n{} / 37,5\nFørte timer i uke 41", antall_timer);
            let week_description = whiteText(week_description_str.as_str(), Alignment::Right);
            let current_date_str = format!("\n−28,5t\nStatus avspasering {}", self.current_date.format("%d.%m"));
            let time_off_description = whiteText(current_date_str.as_str(), Alignment::Left);


            f.render_widget(background, window_size);
            f.render_widget(header, chunks[0]);

            f.render_widget(week_description, header_layout[0]);
            f.render_widget(time_off_description, header_layout[2]);



            let weekdays = week_for_current_day(self.current_date);
            main_days_header.into_iter().enumerate().for_each(
                |(index, layout)| {
                    if index == 0 {
                        let week_number_string = format!("Uke {} ", self.current_date.iso_week().week());
                        f.render_widget(blackText(week_number_string.as_str(), Alignment::Center), layout)
                    } else {
                        let weekday = format!("{}\n{}", weekdays[index-1].format("%d"), WEEKDAYS[index-1]);
                        if self.current_date.weekday() as usize == index - 1{
                            f.render_widget(todayText(weekday.as_str(), Alignment::Center), layout)
                        } else {
                            f.render_widget(blackText(weekday.as_str(), Alignment::Center), layout)
                        }
                    }
                }
            );

            let mut project_week:Vec<(&String, Vec<&Timetrack>)> = Vec::new();

            for (project, timetrackings) in self.time_trackings.iter().sorted_by_key(|&timetrack| &timetrack.id).group_by(|timetrack| &timetrack.id).into_iter() {
                project_week.push((project, timetrackings.collect()));
            }
            dbg!(&project_week);



            main_body[1].split_into_equal_parts_vertical(project_week.len()).into_iter().enumerate().for_each(
                |(project_index, rowlayout)| {
                 let daysForProject = &project_week[project_index];
                 rowlayout.split_into_equal_parts_horizontal(7 + 1).into_iter().enumerate().for_each(
                     |(day_index, daylayout)| {
                         let (project, timetrackings) = daysForProject;
                         if day_index == 0 {
                             let projectLabel = format!("{} - {}\n{}", timetrackings[0].customer, project, timetrackings[0].project);
                             f.render_widget(greyText(projectLabel.as_str(), Alignment::Left), daylayout);
                         }
                         else {
                             match find_timetracking_for_day_of_week(day_index - 1, timetrackings) {
                                 Some(project) => f.render_widget(bgParagraph(format!("{}", project.time.num_minutes() as f32 / 60.0).as_str(), Alignment::Center, BLACK, WHITE), daylayout),
                                 None => f.render_widget(bgParagraph("0.0", Alignment::Center, BLACK, WHITE), daylayout)
                             }
                         }
                     }
                 )
             }
            )
        })
    }
}