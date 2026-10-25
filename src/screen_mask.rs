use amane::{Full, Layer, LayerWindow, Mask, Monitor, Parent, Rectangle, Zone};

use crate::theme;

// one radius for every corner, so the whole screen feels like one rounded shape
pub const SCREEN_RADIUS: f32 = 20.0;

// a bar-colored layer over the area below the bar, with a rounded hole cut out of it.
// only the corners are left, so they look like the bar flowing down the sides and
// around the bottom of the screen. it takes no clicks.
pub fn view(_monitor: &Monitor) -> LayerWindow {
    let hole = Rectangle::new()
        .width(Parent)
        .height(Parent)
        .radius(SCREEN_RADIUS)
        .fill(Mask);

    LayerWindow::new()
        .width(Full)
        .height(Full)
        .layer(Layer::Top)
        .space(Zone::Respect)
        .namespace("screen-mask")
        .click_through()
        .child(
            Rectangle::new()
                .width(Parent)
                .height(Parent)
                .fill(theme::bg())
                .child(hole),
        )
}
