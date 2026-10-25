use amane::{Canvas, Center, Path, Rectangle, Shape, Stack, Widget, children, shapes};

use super::HEIGHT;
use crate::theme;

// how far a group's top edge reaches past its bottom edge at a slanted end
const SLANT: f32 = 16.0;

// how round a free end becomes while a drop-down is open under it
const END_ROUND: f32 = 16.0;

// the number that makes a cubic curve look like a quarter circle
const ARC: f32 = 0.5523;

#[derive(Clone, Copy, PartialEq)]
pub enum Side {
    Left,
    Middle,
    Right,
}

/*
 * one group of the bar on its dark shape. its free ends (the ones not on the screen's
 * edge) are slanted, wider at the top than at the bottom. `reach` is how wide the
 * drop-down under this group is, and how far down it is: the shape grows to that width,
 * the slants straighten out and the bottom corners of the free ends turn round, so
 * the bar flows into the drop-down's own curved corner instead of ending in a straight edge
 */
pub fn group(
    side: Side,
    content: impl Widget + 'static,
    content_width: f32,
    bar_width: f32,
    reach: (f32, f32),
) -> Rectangle {
    let (reach, progress) = reach;

    // the middle group has two free ends, the others one
    let free_left = side != Side::Left;
    let free_right = side != Side::Right;

    let mut ends = 0.0;

    if free_left {
        ends += 1.0;
    }

    if free_right {
        ends += 1.0;
    }

    let round = END_ROUND * progress;
    let slant = SLANT * (1.0 - progress);

    // the rounded corners reach one END_ROUND past the drop-down's edge, so the curve
    // of the bar ends exactly where the drop-down's own curve begins
    let target = reach + END_ROUND * ends;

    let bottom = content_width + (target - content_width).max(0.0) * progress;

    let left_slant = if free_left { slant } else { 0.0 };
    let right_slant = if free_right { slant } else { 0.0 };

    let total = bottom + left_slant + right_slant;

    // where the group sits on the bar, and where its content sits inside it
    let (x, offset) = match side {
        Side::Left => (0.0, 0.0),
        Side::Middle => ((bar_width - total) / 2.0, (total - content_width) / 2.0),
        Side::Right => (bar_width - total, total - content_width),
    };

    let mut path = Path::new().move_to(0.0, 0.0).line_to(total, 0.0);

    // down the right end, rounding its bottom corner, then along the bottom
    let right_x = total - right_slant;

    if free_right && round > 0.5 {
        let (dx, dy) = unit(-right_slant, HEIGHT);

        let before = (right_x - dx * round, HEIGHT - dy * round);
        let after = (right_x - round, HEIGHT);

        path = path.line_to(before.0, before.1).cubic_to(
            before.0 + dx * round * ARC,
            before.1 + dy * round * ARC,
            after.0 + round * ARC,
            HEIGHT,
            after.0,
            after.1,
        );
    } else {
        path = path.line_to(right_x, HEIGHT);
    }

    // along the bottom to the left end, rounding its corner, then up it
    let left_x = left_slant;

    if free_left && round > 0.5 {
        let (ex, ey) = unit(-left_slant, -HEIGHT);

        let start = (left_x + round, HEIGHT);
        let end = (left_x + ex * round, HEIGHT + ey * round);

        path = path.line_to(start.0, start.1).cubic_to(
            start.0 - round * ARC,
            HEIGHT,
            end.0 - ex * round * ARC,
            end.1 - ey * round * ARC,
            end.0,
            end.1,
        );
    } else {
        path = path.line_to(left_x, HEIGHT);
    }

    let path = path.close().fill(theme::bg());

    let canvas = Canvas::new()
        .width(total)
        .height(HEIGHT)
        .shapes(shapes![path]);

    let inner = Rectangle::new()
        .width(content_width)
        .height(HEIGHT)
        .align_child(Center, Center)
        .translate(offset, 0.0)
        .child(content);

    Rectangle::new()
        .width(total)
        .height(HEIGHT)
        .translate(x, 0.0)
        .child(Stack::new(children![canvas, inner]).width(total).height(HEIGHT))
}

// the direction of a step, as a length of 1
fn unit(x: f32, y: f32) -> (f32, f32) {
    let length = f32::hypot(x, y);

    (x / length, y / length)
}
