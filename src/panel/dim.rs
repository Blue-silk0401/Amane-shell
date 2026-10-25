use amane::{Color, Full, Layer, LayerWindow, Parent, Rectangle, Service, Zone};

// never darker than this, so the screen can always be turned back up
const LOWEST: u8 = 15;

/*
 * software brightness for a monitor that can't be controlled: a black window over
 * the whole screen, as see-through as the slider says. 100 is no dimming at all
 */
pub struct Dim {
    percent: u8,
}

impl Service for Dim {
    fn new() -> Self {
        Self { percent: 100 }
    }

    // it only changes through the slider, so there is nothing to poll
    fn listen() {}
}

// the slider's value, always there because this works on any screen
pub fn level() -> Option<u8> {
    Some(Dim::read().percent)
}

pub fn set(percent: u8) {
    let percent = percent.clamp(LOWEST, 100);

    // dragging reports on every move, but only a new number needs a redraw
    let current = Dim::read().percent;

    if current != percent {
        Dim::write().percent = percent;
    }
}

pub fn view() -> LayerWindow {
    let percent = Dim::read().percent;

    // 0 is clear, and the lower the brightness the more black shows
    let amount = f32::from(100 - percent) / 100.0;

    LayerWindow::new()
        .width(Full)
        .height(Full)
        .layer(Layer::Overlay)
        // covers the bar too, not just what's below it
        .space(Zone::Ignore)
        .namespace("dim")
        // it only darkens, every click goes through to what's underneath
        .click_through()
        // no window at all while the screen is at full brightness
        .visible(percent < 100)
        .child(
            Rectangle::new()
                .width(Parent)
                .height(Parent)
                .fill(Color::BLACK)
                .opacity(amount),
        )
}
