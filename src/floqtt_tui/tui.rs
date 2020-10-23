use crate::floqtt_tui::{ApplicationState, Event, Events, Key, black_text, week_for_current_day, WEEKDAYS, today_text, RectSplit, white_text, BG_GREY, PURPLE, bg_paragraph, BLACK, WHITE, find_timetracking_for_day_of_week, grey_text};
use crate::http_client::timetrack::Timetrack;
use chrono::Utc;
use chrono::{Datelike, NaiveDate};
use crossterm::terminal::{enable_raw_mode, disable_raw_mode};
use itertools::Itertools;
use std::io;
use std::io::Stdout;
use std::sync::mpsc::RecvError;
use tui::backend::CrosstermBackend;
use tui::layout::{Alignment, Constraint, Direction, Layout, Margin};
use tui::style::{ Style};
use tui::widgets::{Block, Paragraph};
use tui::Terminal;

pub struct FloqTTTUI {
    state: ApplicationState,
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl FloqTTTUI {
    pub fn new(time_trackings: Vec<Timetrack>) -> Self {
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let terminal = match Terminal::new(backend) {
            Ok(terminal) => terminal,
            Err(_) => panic!("error creating tui"),
        };
        let state = ApplicationState::new(time_trackings);
        enable_raw_mode().expect("could not enter raw mode");
        Self {
            state,
            terminal,
        }
    }

    pub fn start(&mut self) -> Result<(), RecvError> {
        let events = Events::new(250);
        loop {
            match events.next()? {
                Event::Input(key) => match key {
                    Key::Ctrl('c') => {
                        self.terminal.clear();
                        disable_raw_mode().expect("disabling raw mode failed");
                        panic!("shutdown")
                    }
                    Key::Ctrl('d') => {
                        self.terminal.clear();
                        disable_raw_mode().expect("disabling raw mode failed");
                        panic!("shutdown")
                    }

                    Key::Tab => {

                    }

                    Key::Char(input) => self.state.input_write(input),
                    Key::Backspace => {
                        self.state.input_remove_previous();
                    }
                    Key::Left => self.state.move_cursor_back(),
                    Key::Right => self.state.move_cursor_forward(),
                    Key::Enter => {
                        if self.state.input == "quit" || self.state.input == "q" {
                           self.terminal.clear();
                           disable_raw_mode().expect("disabling raw mode failed");
                           break;
                        }
                        self.state.enter()
                    },
                    Key::Up => self.state.previous_cmd(),
                    _ => {}
                },

                Event::Tick => {
                    self.draw_tui().expect("Error drawing");
                }
            }
        }
        Ok(())
    }

    pub fn draw_tui(&mut self) -> Result<(), io::Error> {
        let time_trackings = &self.state.time_trackings();
        let antall_timer: f32 = time_trackings
            .iter()
            .map(|day| day.time.num_minutes() as f32 / 60.0)
            .sum();
        let avspasering_today = &self.state.current_date.format("%d.%m");

        let today = self.state.current_date;
        let input = &self.state.input;
        let input_cursor = self.state.input_cursor;
        self.terminal.draw(|f| {
            let window_margin = Margin {
                vertical: 0,
                horizontal: 0,
            };
            let window_size = f.size().inner(&window_margin);
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Percentage(15),
                        Constraint::Percentage(79),
                        Constraint::Percentage(6),
                    ]
                    .as_ref(),
                )
                .split(window_size);

            let main_layout = chunks[1];

            let background = Block::default()
                .title("Floq Timetracker")
                .style(Style::default().bg(BG_GREY));

            let header = Block::default().style(Style::default().bg(PURPLE));

            let header_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    [
                        Constraint::Percentage(45),
                        Constraint::Percentage(10),
                        Constraint::Percentage(45),
                    ]
                    .as_ref(),
                )
                .vertical_margin(1)
                .split(chunks[0]);

            let main_area = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(0), Constraint::Percentage(100)].as_ref())
                .split(main_layout);

            let main_body_layout = main_area[1];

            let main_body = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(15), Constraint::Percentage(85)].as_ref())
                .split(main_body_layout);

            let main_header = main_body[0];

            let main_days_header = main_header.split_into_equal_parts_horizontal(8);

            let week_description_str = format!("\n{} / 37,5\nFørte timer i uke 41", antall_timer);
            let week_description = white_text(week_description_str.as_str(), Alignment::Right);
            let current_date_str = format!("\n−28,5t\nStatus avspasering {}", avspasering_today);
            let time_off_description = white_text(current_date_str.as_str(), Alignment::Left);

            f.render_widget(background, window_size);
            f.render_widget(header, chunks[0]);
            f.render_widget(week_description, header_layout[0]);
            f.render_widget(time_off_description, header_layout[2]);

            let weekdays = week_for_current_day(today);
            main_days_header
                .into_iter()
                .enumerate()
                .for_each(|(index, layout)| {
                    if index == 0 {
                        let week_number_string = format!("Uke {} ", today.iso_week().week());
                        f.render_widget(
                            black_text(week_number_string.as_str(), Alignment::Center),
                            layout,
                        )
                    } else {
                        let weekday = format!(
                            "{}\n{}",
                            weekdays[index - 1].format("%d"),
                            WEEKDAYS[index - 1]
                        );
                        if today.weekday() as usize == index - 1 {
                            f.render_widget(today_text(weekday.as_str(), Alignment::Center), layout)
                        } else {
                            f.render_widget(black_text(weekday.as_str(), Alignment::Center), layout)
                        }
                    }
                });

            let mut project_week: Vec<(&String, Vec<&Timetrack>)> = Vec::new();

            for (project, timetrackings) in time_trackings
                .iter()
                .sorted_by_key(|&timetrack| &timetrack.id)
                .group_by(|timetrack| &timetrack.id)
                .into_iter()
            {
                project_week.push((project, timetrackings.collect()));
            }

            main_body[1]
                .split_into_equal_parts_vertical(project_week.len())
                .into_iter()
                .enumerate()
                .for_each(|(project_index, rowlayout)| {
                    let days_for_project = &project_week[project_index];
                    rowlayout
                        .split_into_equal_parts_horizontal(7 + 1)
                        .into_iter()
                        .enumerate()
                        .for_each(|(day_index, daylayout)| {
                            let (project, timetrackings) = days_for_project;
                            if day_index == 0 {
                                let project_label = format!(
                                    "{} - {}\n{}",
                                    timetrackings[0].customer, project, timetrackings[0].project
                                );
                                f.render_widget(
                                    grey_text(project_label.as_str(), Alignment::Left),
                                    daylayout,
                                );
                            } else {
                                match find_timetracking_for_day_of_week(
                                    day_index - 1,
                                    timetrackings,
                                ) {
                                    Some(project) => f.render_widget(
                                        bg_paragraph(
                                            format!("{}", project.time.num_minutes() as f32 / 60.0)
                                                .as_str(),
                                            Alignment::Center,
                                            BLACK,
                                            WHITE,
                                        ),
                                        daylayout,
                                    ),
                                    None => f.render_widget(
                                        bg_paragraph("0.0", Alignment::Center, BLACK, WHITE),
                                        daylayout,
                                    ),
                                }
                            }
                        })
                });
            let input_string = format!(" > {}", input);
            let input_panel = Paragraph::new(input_string)
                .block(Block::default())
                .style(Style::default().fg(PURPLE))
                .alignment(Alignment::Left);

            f.set_cursor(chunks[2].x + 3 + input_cursor as u16, chunks[2].y);
            f.render_widget(input_panel, chunks[2]);
        })
    }
}
