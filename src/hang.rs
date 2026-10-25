use amane::{Canvas, Padding, Parent, Path, Rectangle, Shape, Stack, Widget, shapes};

use crate::theme;

// how big the inward curve is, and how much room a side that has one adds to the window
pub const FILLET: f32 = 20.0;

// the curve reaches this far under the panel, so no hairline shows between them
const OVERLAP: f32 = 1.0;

// the number that makes a cubic curve look like a quarter circle
const ARC: f32 = 0.5523;

/*
 * a panel that hangs from the bar. it is `width` wide, plus FILLET on each side that
 * has an inward curve, so the window must be that much wider too. progress goes
 * from 0 (hidden behind the bar) to 1 (all the way down).
 */
pub fn hang(
    body: impl Widget + 'static,
    width: f32,
    height: f32,
    radius: f32,
    padding: f32,
    progress: f32,
    left: bool,
    right: bool,
) -> Rectangle {
    let lead = if left { FILLET } else { 0.0 };
    let rise = (1.0 - progress) * (height + radius);

    // taller than the window and raised, so the bar's edge hides its rounded top corners
    let panel = Rectangle::new()
        .width(width)
        .height(height + radius)
        .radius(radius)
        .fill(theme::bg())
        .padding(Padding {
            top: radius + padding,
            right: padding,
            bottom: padding,
            left: padding,
        })
        .translate(lead, -radius - rise)
        .child(body);

    let mut layers: Vec<Box<dyn Widget>> = Vec::new();

    if left {
        layers.push(Box::new(place(left_corner(), 0.0, rise)));
    }

    if right {
        layers.push(Box::new(place(right_corner(), lead + width - OVERLAP, rise)));
    }

    layers.push(Box::new(panel));

    Rectangle::new()
        .width(Parent)
        .height(Parent)
        .child(Stack::new(layers))
}

// puts a curve at the top edge, sliding along with the panel
fn place(corner: Canvas, x: f32, rise: f32) -> Rectangle {
    Rectangle::new()
        .width(FILLET + OVERLAP)
        .height(FILLET)
        .translate(x, -rise)
        .child(corner)
}

// the space between the bar and a panel's left wall, with a hollow curve cut into it
fn left_corner() -> Canvas {
    let r = FILLET;
    let k = ARC * r;

    Canvas::new().width(r + OVERLAP).height(r).shapes(shapes![
        Path::new()
            .move_to(0.0, 0.0)
            .line_to(r + OVERLAP, 0.0)
            .line_to(r + OVERLAP, r)
            .line_to(r, r)
            .cubic_to(r, r - k, k, 0.0, 0.0, 0.0)
            .close()
            .fill(theme::bg()),
    ])
}

// the same, mirrored, for a panel's right wall
fn right_corner() -> Canvas {
    let r = FILLET;
    let k = ARC * r;
    let e = OVERLAP;

    Canvas::new().width(r + e).height(r).shapes(shapes![
        Path::new()
            .move_to(e + r, 0.0)
            .line_to(0.0, 0.0)
            .line_to(0.0, r)
            .line_to(e, r)
            .cubic_to(e, r - k, e + r - k, 0.0, e + r, 0.0)
            .close()
            .fill(theme::bg()),
    ])
}
