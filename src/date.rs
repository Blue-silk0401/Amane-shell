use std::time::Duration;

use amane::Service;

// like the Clock, but for the date
pub struct Date {
    text: String,
}

impl Service for Date {
    fn new() -> Self {
        Self { text: read_date() }
    }

    // the date changes once a day, so checking every minute is plenty
    fn interval() -> Duration {
        Duration::from_secs(60)
    }

    fn update(&mut self) -> bool {
        let text = read_date();

        if text == self.text {
            return false;
        }

        self.text = text;

        true
    }
}

impl Date {
    pub fn text(&self) -> &str {
        &self.text
    }
}

// prints something like "Fri 16 Oct"
fn read_date() -> String {
    amane::output("date '+%a , %d %b'").trim().to_string()
}
