mod pretty;
mod json;

pub use pretty::*;
pub use json::*;

use chrono::{DateTime, Utc};
use serde::{Serialize};
use crate::test::{TestData, TestListener};

#[derive(Serialize)]
pub enum EventType {
    Start,
    Finish,
    Report,
}

#[derive(Serialize)]
pub struct Event<'a> {
    r#type: EventType,
    data: &'a TestData,
    previous_data: &'a Option<TestData>,
    timestamp: DateTime<Utc>,
}

impl<'a> Event<'a> {
    pub fn new(r#type: EventType, data: &'a TestData, previous_data: &'a Option<TestData>) -> Self {
        Self { r#type, timestamp: Utc::now(), data, previous_data }
    }
}

pub trait Format {
    fn format(&self, event: &Event) -> String;
}

#[derive(Clone)]
pub struct FormattedTestPrinter<F: Format> {
    format: F,
    last_data: Option<TestData>,
}

impl<F: Format> FormattedTestPrinter<F> {
    pub fn new(format: F) -> Self {
        Self { format, last_data: None }
    }
}

impl<F: Format> TestListener for FormattedTestPrinter<F> {
    fn on_start(&mut self, data: &TestData) {
        self.format_and_print(EventType::Start, &data);
    }

    fn on_finish(&mut self, data: &TestData) {
        self.format_and_print(EventType::Finish, &data);
    }

    fn on_report(&mut self, data: &TestData) {
        self.format_and_print(EventType::Report, &data);
    }
}

impl<F: Format> FormattedTestPrinter<F> {
    fn format_and_print(&mut self, r#type: EventType, data: &TestData) {
        let formatted = self.format.format(&Event::new(r#type, &data, &self.last_data));
        println!("{}", formatted);
        self.last_data = Some(data.clone());
    }
}

