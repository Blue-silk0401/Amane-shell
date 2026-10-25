use std::time::Duration;

use amane::Service;

// leave empty to use the place your IP address points to, or write a city like "Reykjavik"
const CITY: &str = "";

const REFRESH: Duration = Duration::from_secs(30 * 60);
const RETRY: Duration = Duration::from_secs(60);

pub struct Weather {
    text: String,
}

impl Service for Weather {
    fn new() -> Self {
        Self { text: String::new() }
    }

    fn listen() {
        loop {
            let text = fetch();

            let failed = text.is_empty();

            {
                let mut weather = Self::write();

                if weather.text != text && !failed {
                    weather.text = text;
                }
            }

            std::thread::sleep(if failed { RETRY } else { REFRESH });
        }
    }
}

impl Weather {
    pub fn text(&self) -> &str {
        &self.text
    }
}

// "18°C  Partly cloudy", or nothing when the request fails
fn fetch() -> String {
    let place = CITY.replace(' ', "+");

    let reply = amane::output(&format!("curl -s --max-time 8 'wttr.in/{place}?format=%t|%C'"));
    let reply = reply.trim();

    if reply.contains('<') {
        return String::new();
    }

    let Some((temperature, condition)) = reply.split_once('|') else {
        return String::new();
    };

    format!("{}  {}", temperature.trim().trim_start_matches('+'), condition.trim())
}
