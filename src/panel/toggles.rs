use amane::{Center, Column, Pointer, Rectangle, Text, children};

use super::NERD_FONT;
use crate::theme;

pub const HEIGHT: f32 = 64.0;

// filled with the accent color while it is on
pub fn button(icon: &str, label: &str, active: bool, width: f32, action: fn()) -> Rectangle {
    let (fill, color) = if active {
        (theme::accent(), theme::on_accent())
    } else {
        (theme::surface(), theme::text())
    };

    let glyph = Text::new(icon)
        .size(20.0)
        .font(NERD_FONT)
        .tight()
        .color(color);

    // a fixed box, so a long name is cut off with "..." instead of spilling out
    let name = Rectangle::new()
        .width(width - 12.0)
        .height(14.0)
        .align_child(Center, Center)
        .child(Text::new(label).size(11.0).color(color).elide());

    Rectangle::new()
        .width(width)
        .height(HEIGHT)
        .radius(16.0)
        .fill(fill)
        .cursor(Pointer)
        .on_click(move |_| action())
        .align_child(Center, Center)
        .child(Column::new(children![glyph, name]).gap(4.0).align(Center))
}
