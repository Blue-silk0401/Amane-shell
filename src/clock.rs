use std::time::Duration;

use amane::Service;

// A Service is data that lives outside the view and updates in the background.
// The view can't run commands (it would freeze the bar), so the clock is a Service.
pub struct Clock {
    time: String,
}

impl Service for Clock {
    // runs the first time anything calls Clock::read()
    fn new() -> Self {
        Self { time: read_time() }
    }

    // how often update() runs
    fn interval() -> Duration {
        Duration::from_secs(5)
    }

    // return true only when something visible changed, so the bar isn't redrawn for nothing
    fn update(&mut self) -> bool {
        let time = read_time();

        if time == self.time {
            return false;
        }

        self.time = time;

        true
    }
}

impl Clock {
    pub fn time(&self) -> &str {
        &self.time
    }
}

// `date +%H:%M` prints something like "17:52"
fn read_time() -> String {
    amane::output("date '+%-I:%M %p'").trim().to_string()
}
