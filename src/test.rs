use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

/// Shared between client and server
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestPlan {
    /// Duration in seconds
    pub duration: f64,
    /// Per packet byte size
    pub packet_size: usize,
}

#[derive(Clone)]
pub struct TestOptions {
    /// Report interval in seconds
    pub report_interval: f64,
    pub event_handler: Rc<RefCell<Box<dyn TestListener>>>,
}

impl Default for TestOptions {
    fn default() -> Self {
        Self::new(f64::MAX, EmptyTestEventHandler)
    }
}

impl TestOptions {
    pub fn new<EventHandler: TestListener + 'static>(
        report_interval: f64,
        event_handler: EventHandler,
    ) -> Self {
        Self {
            report_interval,
            event_handler: Rc::new(RefCell::new(Box::new(event_handler))),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestData {
    pub id: usize,
    pub total_transfer: usize,
    pub total_packets: usize,
    pub plan: TestPlan,

    #[serde(skip_serializing)]
    start_time: DateTime<Utc>,
    #[serde(skip_serializing)]
    report_count: usize,
}

impl TestData {
    pub fn new(id: usize, plan: TestPlan) -> Self {
        Self {
            id,
            total_transfer: 0,
            total_packets: 0,
            start_time: Utc::now(),
            report_count: 0,
            plan,
        }
    }

    pub fn elapsed(&self) -> Duration {
        let now = Utc::now();
        (now - self.start_time).to_std().unwrap()
    }
}

pub struct Test {
    pub data: TestData,
    pub options: TestOptions,
}

pub trait TestListener {
    fn on_start(&mut self, data: &TestData);
    fn on_finish(&mut self, data: &TestData);
    fn on_report(&mut self, data: &TestData);
}

impl Test {
    pub fn new(data: TestData, options: TestOptions) -> Self {
        Self { data, options }
    }

    pub fn elapsed(&self) -> Duration {
        self.data.elapsed()
    }

    pub fn transferred(&mut self, n: usize) {
        self.data.total_transfer += n;
        self.data.total_packets += 1;

        if self.should_report() {
            self.options
                .event_handler
                .borrow_mut()
                .on_report(&mut self.data);
        }
    }

    pub fn start(&mut self) {
        self.data.start_time = Utc::now();
        self.options.event_handler.borrow_mut().on_start(&self.data);
    }

    pub fn finish(&mut self) {
        self.options
            .event_handler
            .borrow_mut()
            .on_finish(&self.data);
    }

    pub fn should_report(&mut self) -> bool {
        let elapsed = self.elapsed();
        if elapsed.as_secs_f64() >= (self.data.report_count as f64) * self.options.report_interval {
            self.data.report_count += 1;
            true
        } else {
            false
        }
    }
}

struct EmptyTestEventHandler;

impl TestListener for EmptyTestEventHandler {
    fn on_start(&mut self, _: &TestData) {}
    fn on_finish(&mut self, _: &TestData) {}
    fn on_report(&mut self, _: &TestData) {}
}
