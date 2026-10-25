use amane::{Battery, Center, Row, Service, Text, Widget};

use super::{quick, stats};
use crate::theme;

// from the group's edges, and between its parts
const SIDE: f32 = 14.0;
const GAP: f32 = 14.0;

// a guess at how wide one character is
const CHAR_WIDTH: f32 = 8.0;

// the battery (if there is one), processor and memory use, and the panel button
pub fn view() -> (Row, f32) {
    let mut items: Vec<Box<dyn Widget>> = Vec::new();

    let mut width = SIDE * 2.0;

    if let Some((text, text_width)) = battery() {
        items.push(Box::new(text));

        width += text_width + GAP;
    }

    items.push(Box::new(stats::view()));
    items.push(Box::new(quick::view()));

    width += stats::WIDTH + GAP + quick::WIDTH;

    (Row::new(items).gap(GAP).align(Center), width)
}

// `None` on a desktop with no battery, so nothing is drawn
fn battery() -> Option<(Text, f32)> {
    let battery = Battery::read();

    if !battery.present() {
        return None;
    }

    let percent = battery.percent();

    let color = if battery.charging() {
        theme::success()
    } else if percent > 20 {
        theme::text()
    } else {
        theme::danger()
    };

    let text = format!("{percent}%");

    let width = text.chars().count() as f32 * CHAR_WIDTH;

    Some((Text::new(text).size(13.0).color(color), width))
}
