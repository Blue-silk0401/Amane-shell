use std::cell::{Cell, RefCell};
use std::time::Duration;

use amane::{
    Animation, Audio, Brightness, Center, Column, Easing, Full, Horizontal, Layer, LayerWindow,
    Parent, Rectangle, Row, Service, Text, Vertical, Zone, children,
};

use crate::theme;

const FONT: &str = "JetBrainsMono Nerd Font";

const WIDTH: f32 = 300.0;
const HEIGHT: f32 = 56.0;

// the space between the pill and the bottom of the screen
const BOTTOM: f32 = 70.0;

const BAR: f32 = 150.0;
const BAR_HEIGHT: f32 = 6.0;

const HOLD_MS: u64 = 1400;
const FADE_MS: u64 = 350;

const VOLUME_MUTED: &str = "\u{f0581}";
const VOLUME_LOW: &str = "\u{f057f}";
const VOLUME_MEDIUM: &str = "\u{f0580}";
const VOLUME_HIGH: &str = "\u{f057e}";
const SUN: &str = "\u{f00e0}";

#[derive(Clone, Copy)]
enum Kind {
    Volume,
    Brightness,
}

thread_local! {
    // volume, muted and brightness as last drawn, so a change can be noticed
    static LAST: Cell<Option<(u8, bool, Option<u8>)>> = const { Cell::new(None) };

    static KIND: Cell<Kind> = const { Cell::new(Kind::Volume) };

    // runs from 0 to 1 over the time the popup stays: held at first, then fading
    static PULSE: RefCell<Animation> = RefCell::new(Animation::new(1.0));
}

fn restart_pulse() {
    let total = HOLD_MS + FADE_MS;

    let mut pulse = Animation::new(0.0)
        .duration(Duration::from_millis(total))
        .easing(Easing::Linear);

    pulse.to(1.0);

    PULSE.with(|slot| *slot.borrow_mut() = pulse);
}

pub fn view() -> LayerWindow {
    let (volume, muted) = {
        let audio = Audio::read();

        (audio.volume().min(100), audio.muted())
    };

    let brightness = {
        let brightness = Brightness::read();

        brightness.present().then(|| brightness.percent().min(100))
    };

    // the first draw only records the values, so the popup doesn't appear at startup
    let current = (volume, muted, brightness);

    let previous = LAST.with(|last| last.replace(Some(current)));

    if let Some((old_volume, old_muted, old_brightness)) = previous {
        if (old_volume, old_muted) != (volume, muted) {
            KIND.with(|kind| kind.set(Kind::Volume));
            restart_pulse();
        } else if old_brightness != brightness {
            KIND.with(|kind| kind.set(Kind::Brightness));
            restart_pulse();
        }
    }

    let progress = PULSE.with(|pulse| pulse.borrow().value());
    let shown = progress < 0.999;

    let window = LayerWindow::new()
        .width(WIDTH)
        .height(HEIGHT + BOTTOM)
        .anchor_vertical(Vertical::Bottom)
        .anchor_horizontal(Horizontal::Middle)
        .layer(Layer::Overlay)
        .space(Zone::Ignore)
        .namespace("amane-osd")
        .click_through()
        .visible(shown);

    if !shown {
        return window.child(Rectangle::new().width(Parent).height(Parent));
    }

    let (icon, percent) = match KIND.with(|kind| kind.get()) {
        Kind::Volume => {
            let icon = if muted || volume == 0 {
                VOLUME_MUTED
            } else if volume < 34 {
                VOLUME_LOW
            } else if volume < 67 {
                VOLUME_MEDIUM
            } else {
                VOLUME_HIGH
            };

            (icon, if muted { 0 } else { volume })
        }
        Kind::Brightness => (SUN, brightness.unwrap_or(0)),
    };

    // held fully visible, then faded out over the last part
    let hold_share = HOLD_MS as f32 / (HOLD_MS + FADE_MS) as f32;

    let opacity = if progress < hold_share {
        1.0
    } else {
        1.0 - (progress - hold_share) / (1.0 - hold_share)
    };

    let icon = Rectangle::new()
        .width(32.0)
        .height(HEIGHT)
        .align_child(Center, Center)
        .child(Text::new(icon).size(22.0).font(FONT).tight().color(theme::accent()));

    let track = Rectangle::new().width(BAR).height(BAR_HEIGHT).radius(Full).fill(theme::surface());

    let fill_width = BAR * percent as f32 / 100.0;

    let bar: Rectangle = if fill_width > 0.5 {
        track.child(
            Rectangle::new()
                .width(fill_width)
                .height(BAR_HEIGHT)
                .radius(Full)
                .fill(theme::accent()),
        )
    } else {
        track
    };

    let bar = Rectangle::new().width(BAR).height(HEIGHT).align_child(Center, Center).child(bar);

    let number = Rectangle::new()
        .width(44.0)
        .height(HEIGHT)
        .align_child(Center, Center)
        .child(Text::new(format!("{percent}")).size(15.0).color(theme::text()));

    let pill = Rectangle::new()
        .width(WIDTH)
        .height(HEIGHT)
        .radius(Full)
        .fill(theme::bg())
        .border(1.0, theme::surface())
        .opacity(opacity.clamp(0.0, 1.0))
        .align_child(Center, Center)
        .child(Row::new(children![icon, bar, number]).gap(10.0));

    window.child(Column::new(children![pill, Rectangle::new().width(Parent).height(BOTTOM)]))
}
