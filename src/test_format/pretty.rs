use crate::test_format::{Event, EventType, Format};

#[derive(Clone)]
pub struct Pretty;

impl Format for Pretty {
    fn format(&self, event: &Event) -> String {
        match event.r#type {
            EventType::Start => {
                format!("Test started #{}", event.data.id)
            }
            EventType::Finish => {
                format!("Test finished #{}", event.data.id)
            }
            EventType::Report => {
                let total_transfer = self.format_bytes(event.data.total_transfer);
                if let Some(previous_data) = event.previous_data {
                    let throughput = self.format_bytes(
                        (event.data.total_transfer - previous_data.total_transfer) * 8,
                    );
                    format!(
                        "[{:.2}s] {}B ({}bit/s)",
                        event.data.elapsed().as_secs_f64(),
                        total_transfer,
                        throughput
                    )
                } else {
                    format!(
                        "[{:.2}s] {}B",
                        event.data.elapsed().as_secs_f64(),
                        total_transfer
                    )
                }
            }
        }
    }
}

impl Pretty {
    fn format_bytes(&self, n: usize) -> String {
        let formats = ["", "K", "M", "G", "T", "P"];
        let base = 1000_f64;
        let n = n as f64;
        let index = (n.log10() / base.log10()).floor() as usize;
        let n = n / base.powi(index as i32);
        format!("{:.2}{}", n, formats[index])
    }
}
