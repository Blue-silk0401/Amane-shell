use std::time::Duration;

use amane::{
    Center, Column, Layer, LayerWindow, Monitor, Parent, Rectangle, Service, Text, Weight, Zone,
    children,
};

use crate::theme;

const FONT: &str = "JetBrainsMono Nerd Font Propo";

// how far down the screen the weekday starts, as a share of the screen's height
const TOP_SHARE: f32 = 0.16;

const WEEKDAY_SIZE: f32 = 64.0;
const DATE_SIZE: f32 = 22.0;
const TIME_SIZE: f32 = 20.0;
const GAP: f32 = 14.0;

// the weekday, date and time the face shows, kept up to date in the background
pub struct Face {
    weekday: String,
    date: String,
    time: String,
}

impl Service for Face {
    fn new() -> Self {
        read()
    }

    fn interval() -> Duration {
        Duration::from_secs(5)
    }

    fn update(&mut self) -> bool {
        let now = read();

        if now.weekday == self.weekday && now.date == self.date && now.time == self.time {
            return false;
        }

        *self = now;

        true
    }
}

// one call to `date` gives all three, split on a bar that no date contains
fn read() -> Face {
    let text = amane::output("date '+%A|%d %b %Y|%-I:%M %p'");
    let mut parts = text.trim().split('|');

    let mut next = || parts.next().unwrap_or("").to_uppercase();

    Face { weekday: next(), date: next(), time: next() }
}

// a space between every letter, since Text has no letter spacing of its own
fn spaced(text: &str) -> String {
    text.chars().map(String::from).collect::<Vec<_>>().join(" ")
}

pub fn view(monitor: &Monitor) -> LayerWindow {
    let face = Face::read();

    let weekday = Text::new(spaced(&face.weekday))
        .size(WEEKDAY_SIZE)
        .font(FONT)
        .weight(Weight::Bold)
        .color(theme::accent());

    let date = Text::new(face.date.clone()).size(DATE_SIZE).font(FONT).color(theme::text());

    let time = Text::new(format!("-  {}  -", face.time))
        .size(TIME_SIZE)
        .font(FONT)
        .color(theme::text());

    // pushes everything down to a third of the screen
    let spacer = Rectangle::new().width(Parent).height(monitor.height as f32 * TOP_SHARE);

    LayerWindow::new()
        .width(amane::Full)
        .height(amane::Full)
        .layer(Layer::Bottom)
        .space(Zone::Ignore)
        .namespace("amane-face")
        .click_through()
        .child(Column::new(children![spacer, weekday, date, time]).width(Parent).align(Center).gap(GAP))
}
