use crate::test_format::{Event, Format};

#[derive(Clone)]
pub struct Json;

impl Format for Json {
    fn format(&self, event: &Event) -> String {
        serde_json::to_string(event).unwrap_or("".to_string())
    }
}
