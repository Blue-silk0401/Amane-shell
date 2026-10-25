use amane::{
    Arc, Canvas, Cap, Center, Color, Cpu, Memory, Rectangle, Row, Service, Shape, Stack, Text,
    children, shapes,
};

use crate::panel::NERD_FONT;
use crate::theme;

const CPU_ICON: &str = "\u{f061a}";
const MEMORY_ICON: &str = "\u{f035b}";

const HEIGHT: f32 = 28.0;

// the ring and how thick its line is, with the icon inside
const RING: f32 = 28.0;
const RING_STROKE: f32 = 2.5;
const ICON_SIZE: f32 = 12.0;

const CPU_TEXT: f32 = 34.0;
const MEMORY_TEXT: f32 = 44.0;
const INNER_GAP: f32 = 6.0;
const GAP: f32 = 14.0;

pub const WIDTH: f32 = (RING + INNER_GAP) * 2.0 + CPU_TEXT + MEMORY_TEXT + GAP;

// at or above this the ring and the number turn red
const HOT: u8 = 85;

// where the ring starts: 0 is straight up, and the angles grow clockwise,
// so -90 is the left side. use 0 to start at the top instead
const START: f32 = 0.0;

// processor use as a percentage, memory as gigabytes in use, each in a ring that fills with use
pub fn view() -> Row {
    let cpu = Cpu::read().percent();

    let (percent, used_kib) = {
        let memory = Memory::read();

        (memory.percent(), memory.used_kib())
    };

    // the kernel counts in kibibytes, and this is gibibytes, the unit fastfetch shows
    let gib = used_kib as f32 / 1024.0 / 1024.0;

    Row::new(children![
        stat(CPU_ICON, format!("{cpu}%"), CPU_TEXT, cpu),
        stat(MEMORY_ICON, format!("{gib:.1}G"), MEMORY_TEXT, percent),
    ])
    .gap(GAP)
    .align(Center)
}

fn stat(icon: &str, text: String, text_width: f32, percent: u8) -> Row {
    let hot = percent >= HOT;

    let color = if hot { theme::danger() } else { theme::text() };
    let ring_color = if hot { theme::danger() } else { theme::accent() };

    let value = f32::from(percent.min(100)) / 100.0;

    let icon = Rectangle::new()
        .width(RING)
        .height(RING)
        .align_child(Center, Center)
        .child(Text::new(icon).size(ICON_SIZE).font(NERD_FONT).tight().color(color));

    let ring = Stack::new(children![ring(value, ring_color), icon])
        .width(RING)
        .height(RING);

    Row::new(children![ring, slot(text_width, Text::new(text).size(13.0).color(color))])
        .gap(INNER_GAP)
        .align(Center)
}

// a dim full circle, and on top of it the part that is used
fn ring(value: f32, color: Color) -> Canvas {
    let mut shapes = shapes![Arc::new().stroke(RING_STROKE, theme::surface())];

    // an empty ring draws only the track: a zero-length line with round ends would show as a dot
    if value > 0.01 {
        // a sweep of exactly a full turn can vanish, so it stops just short
        let sweep = (360.0 * value).min(359.9);

        shapes.push(Box::new(
            Arc::new()
                .start(START)
                .sweep(sweep)
                .stroke(RING_STROKE, color)
                .cap(Cap::Round),
        ));
    }

    Canvas::new().width(RING).height(RING).shapes(shapes)
}

// a fixed-width box, so the group doesn't change size when the number gains a digit
fn slot(width: f32, text: Text) -> Rectangle {
    Rectangle::new()
        .width(width)
        .height(HEIGHT)
        .align_child(Center, Center)
        .child(text)
}
