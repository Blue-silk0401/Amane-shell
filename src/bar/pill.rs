use amane::{Center, Color, Full, Rectangle, Text};

const HEIGHT: f32 = 28.0;
const TEXT_SIZE: f32 = 15.0;
const PADDING: f32 = 24.0;

// amane can't size a widget to its text yet, so the width is a guess:
// about 0.6 of the font size per character. Tune the 0.6 if text looks cramped or loose.
fn text_width(text: &str) -> f32 {
    text.chars().count() as f32 * TEXT_SIZE * 0.6
}

// a rounded capsule with centered text, used by the clock and the battery
pub fn view(text: &str, fill: Color, text_color: Color) -> Rectangle {
    Rectangle::new()
        .width(text_width(text) + PADDING)
        .height(HEIGHT)
        .radius(Full)
        .fill(fill)
        .align_child(Center, Center)
        .child(Text::new(text).size(TEXT_SIZE).color(text_color))
}
