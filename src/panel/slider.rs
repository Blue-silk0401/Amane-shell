use amane::{Center, End, Full, Point, Pointer, Rectangle, Stack, Start, Text, children};

use super::NERD_FONT;
use crate::theme;

pub const HEIGHT: f32 = 44.0;

// the track starts past the icon and ends before the percentage
const TRACK_LEFT: f32 = 48.0;
const TRACK_HEIGHT: f32 = 6.0;
const TRACK_GAP: f32 = 10.0;
const THUMB: f32 = 14.0;
const SIDE: f32 = 14.0;
const PERCENT_WIDTH: f32 = 40.0;

/*
 * pressing anywhere on it sets the value under the pointer, and dragging follows.
 * `on_change` is whatever should happen with the new number, so one slider
 * serves both volume and brightness
 */
pub fn view(icon: &str, percent: u8, width: f32, on_change: fn(u8)) -> Stack {
    let value = f32::from(percent) / 100.0;

    let track_width = width - TRACK_LEFT - TRACK_GAP - PERCENT_WIDTH - SIDE;

    let filled = Rectangle::new()
        .width(track_width * value)
        .height(TRACK_HEIGHT)
        .radius(Full)
        .fill(theme::accent());

    let track_bar = Rectangle::new()
        .width(track_width)
        .height(TRACK_HEIGHT)
        .radius(Full)
        .fill(theme::hover())
        .child(filled);

    // kept inside the track at both ends
    let thumb_x = (value * track_width - THUMB / 2.0).clamp(0.0, track_width - THUMB);

    let thumb = Rectangle::new()
        .width(THUMB)
        .height(THUMB)
        .radius(Full)
        .fill(theme::text())
        .translate(thumb_x, (TRACK_HEIGHT - THUMB) / 2.0);

    let track = Rectangle::new()
        .width(track_width)
        .height(HEIGHT)
        .translate(TRACK_LEFT, 0.0)
        .align_child(Start, Center)
        .child(Stack::new(children![track_bar, thumb]).width(track_width).height(TRACK_HEIGHT));

    let icon = Rectangle::new()
        .width(TRACK_LEFT)
        .height(HEIGHT)
        .translate(SIDE, 0.0)
        .align_child(Start, Center)
        .child(
            Text::new(icon)
                .size(18.0)
                .font(NERD_FONT)
                .tight()
                .color(theme::text()),
        );

    let label = Rectangle::new()
        .width(PERCENT_WIDTH)
        .height(HEIGHT)
        .translate(width - SIDE - PERCENT_WIDTH, 0.0)
        .align_child(End, Center)
        .child(Text::new(format!("{percent}%")).size(12.0).color(theme::text()));

    // the pointer is measured from this rectangle's corner, the track starts further in
    let background = Rectangle::new()
        .width(width)
        .height(HEIGHT)
        .radius(16.0)
        .fill(theme::surface())
        .cursor(Pointer)
        .on_drag(move |point: Point| {
            let along = (point.x - TRACK_LEFT) / track_width;

            on_change((along.clamp(0.0, 1.0) * 100.0).round() as u8);
        });

    Stack::new(children![background, icon, track, label])
        .width(width)
        .height(HEIGHT)
}
