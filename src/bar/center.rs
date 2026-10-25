use amane::{Center, Color, Full, Pointer, Rectangle, Row, Service, Text, children};

use crate::calendar::{self, Calendar};
use crate::clock::Clock;
use crate::date::Date;
use crate::theme;

const TEXT_SIZE: f32 = 14.0;
const DOT_SIZE: f32 = 4.0;
const INNER_GAP: f32 = 8.0;

const BUTTON_HEIGHT: f32 = 28.0;
const BUTTON_PADDING: f32 = 12.0;

// a guess at how wide one character is, to size the highlight around the text
const CHAR_WIDTH: f32 = 8.4;

// from the group's edges to the highlight
const SIDE: f32 = 10.0;

// the date and time, as one button that opens the calendar, and how wide the group is
pub fn view() -> (Rectangle, f32) {
    let time = Clock::read().time().to_string();
    let date = Date::read().text().to_string();

    // clear until the pointer is on it, and kept lit while the calendar is down
    let lit = Calendar::is_open() || Calendar::hovered();

    let fill = if lit { theme::hover() } else { Color::TRANSPARENT };

    let characters = (time.chars().count() + date.chars().count()) as f32;

    let button_width = characters * CHAR_WIDTH + DOT_SIZE + INNER_GAP * 2.0 + BUTTON_PADDING * 2.0;

    let dot = Rectangle::new()
        .width(DOT_SIZE)
        .height(DOT_SIZE)
        .radius(Full)
        .fill(theme::muted());

    let contents = Row::new(children![
        Text::new(time).size(TEXT_SIZE).color(theme::text()),
        dot,
        Text::new(date).size(TEXT_SIZE).color(theme::text()),
    ])
    .gap(INNER_GAP)
    .align(Center);

    let button = Rectangle::new()
        .width(button_width)
        .height(BUTTON_HEIGHT)
        .radius(Full)
        .fill(fill)
        .cursor(Pointer)
        .on_hover(|inside| calendar::set_hover(inside))
        .on_click(|_| calendar::toggle())
        .align_child(Center, Center)
        .child(contents);

    (button, button_width + SIDE * 2.0)
}
