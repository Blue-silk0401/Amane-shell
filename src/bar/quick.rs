use amane::{Audio, Center, Color, Full, Network, Pointer, Rectangle, Row, Service, Text, children};

use crate::notifications::Quiet;
use crate::panel::{self, NERD_FONT, Panel};
use crate::theme;

const HEIGHT: f32 = 28.0;
const ICON_SLOT: f32 = 20.0;
const SIDE: f32 = 10.0;
const GAP: f32 = 8.0;

// a crossed-out bell, shown while do not disturb is on
const BELL_OFF: &str = "\u{f009b}";

pub const WIDTH: f32 = SIDE * 2.0 + ICON_SLOT * 3.0 + GAP * 2.0;

// whether the pointer is on the button
pub struct Hover {
    on: bool,
}

impl Service for Hover {
    fn new() -> Self {
        Self { on: false }
    }

    fn listen() {}
}

// only writes when something changes, so it doesn't redraw for nothing
fn set_hover(inside: bool) {
    let current = Hover::read().on;

    if current != inside {
        Hover::write().on = inside;
    }
}

// wifi, volume and notifications together: click them to open the panel
pub fn view() -> Rectangle {
    let (wifi_icon, wifi_color) = {
        let network = Network::read();

        if !network.wifi_enabled() {
            (panel::WIFI_OFF, theme::muted())
        } else if network.connected() {
            (panel::WIFI, theme::text())
        } else {
            (panel::WIFI, theme::muted())
        }
    };

    let volume_color = if Audio::read().muted() { theme::muted() } else { theme::text() };
    let (bell_icon, bell_color) = if Quiet::on() {
        (BELL_OFF, theme::accent())
    } else {
        (panel::BELL, theme::text())
    };

    // clear, and lit while the pointer is on it or the panel is open
    let hovered = Hover::read().on;

    let fill = if hovered || Panel::is_open() {
        theme::hover()
    } else {
        Color::TRANSPARENT
    };

    Rectangle::new()
        .width(WIDTH)
        .height(HEIGHT)
        .radius(Full)
        .fill(fill)
        .cursor(Pointer)
        .on_hover(|inside| set_hover(inside))
        .on_click(|_| panel::toggle())
        .align_child(Center, Center)
        .child(
            Row::new(children![
                glyph(wifi_icon, wifi_color),
                glyph(panel::VOLUME, volume_color),
                glyph(bell_icon, bell_color),
            ])
            .gap(GAP)
            .align(Center),
        )
}

fn glyph(icon: &str, color: Color) -> Rectangle {
    Rectangle::new()
        .width(ICON_SLOT)
        .height(ICON_SLOT)
        .align_child(Center, Center)
        .child(Text::new(icon).size(16.0).font(NERD_FONT).tight().color(color))
}
