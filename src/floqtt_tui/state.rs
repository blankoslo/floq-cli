use crate::http_client::timetrack::Timetrack;
use chrono::{NaiveDate, Utc};

pub struct ApplicationState {
    pub input: String,
    pub input_cursor: usize,
    pub stack: Vec<String>,
    stack_cursor: usize,
    time_trackings: Vec<Timetrack>,
    pub current_date: NaiveDate
}

impl ApplicationState {
    pub fn new(time_trackings: Vec<Timetrack>) -> Self {
        ApplicationState {
            input: String::new(),
            input_cursor: 0,
            stack: vec![],
            stack_cursor: 0,
            time_trackings,
            current_date: Utc::now().naive_local().date(),
        }
    }

    pub fn input_write(&mut self, character: char) {
        self.input.insert(self.input_cursor, character);
        self.input_cursor += 1;
    }

    pub fn input_remove_previous(&mut self) {
        if self.input_cursor > 0 {
            self.input_cursor -= 1;
            self.input.remove(self.input_cursor);
        }
    }

    pub fn move_cursor_back(&mut self) {
        if self.input_cursor > 0 {
            self.input_cursor -= 1;
        }
    }

    pub fn move_cursor_forward(&mut self) {
        if self.input_cursor < self.input.len() {
            self.input_cursor += 1;
        }
    }

    pub fn enter(&mut self) {
        if !self.input.is_empty() {
            self.stack.push(self.input.clone());
            self.stack_cursor += 1;
            self.input_cursor = 0;
            self.input.clear();
        }
    }

    pub fn previous_cmd(&mut self) {
        if self.stack_cursor < self.stack.len() {
            self.input = self
                .stack
                .get(self.stack_cursor)
                .clone()
                .unwrap()
                .to_string();
            self.stack_cursor += 1;
        } else {
            self.stack_cursor = 0;
            self.input.clear();
        }
    }

    pub fn time_trackings(&self) -> &Vec<Timetrack> {
        &self.time_trackings
    }
}
