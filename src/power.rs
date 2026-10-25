use std::time::Duration;

use amane::{
    Animation, Center, Color, Column, Easing, Full, Horizontal, Key, Keyboard, Layer, LayerWindow,
    Parent, Pointer, Rectangle, Row, Service, Start, Text, Vertical, Widget, children,
};

use crate::panel::NERD_FONT;
use crate::hang;
use crate::theme;

const WIDTH: f32 = 250.0;
const PADDING: f32 = 12.0;
const INNER: f32 = WIDTH - PADDING * 2.0;
const RADIUS: f32 = 24.0;

const ROW_HEIGHT: f32 = 44.0;
const GAP: f32 = 4.0;
const ICON_SLOT: f32 = 28.0;
const SLIDE_MS: u64 = 320;

// icon, label, and whether it asks "are you sure" with a second click
const ENTRIES: [(&str, &str, bool); 5] = [
    ("\u{f033e}", "Lock", false),
    ("\u{f0594}", "Sleep", false),
    ("\u{f0343}", "Log out", true),
    ("\u{f0709}", "Restart", true),
    ("\u{f0425}", "Shut down", true),
];

pub struct Power {
    open: bool,

    // how far down the menu is
    slide: Animation,

    // the row under the pointer
    hover: Option<usize>,

    // the row that is waiting for its second click
    confirm: Option<usize>,

    // the pointer is on the logo in the bar
    logo: bool,
}

impl Service for Power {
    fn new() -> Self {
        Self {
            open: false,
            slide: Animation::new(0.0)
                .duration(Duration::from_millis(SLIDE_MS))
                .easing(Easing::Out),
            hover: None,
            confirm: None,
            logo: false,
        }
    }

    // it only changes through clicks
    fn listen() {}
}

impl Power {
    pub fn is_open() -> bool {
        Self::read().open
    }
}

// the logo calls this
pub fn toggle() {
    if Power::is_open() {
        close();
    } else {
        open_menu();
    }
}

// `amane ipc call power`, so a niri key can open it too
pub fn ipc(_: &[String]) -> String {
    toggle();

    String::from("toggled")
}

fn open_menu() {
    crate::panel::dismiss();
    crate::calendar::dismiss();

    let mut power = Power::write();

    power.open = true;
    power.confirm = None;
    power.hover = Some(0);
    power.slide.to(1.0);
}

fn close() {
    let mut power = Power::write();

    power.open = false;
    power.confirm = None;
    power.hover = None;
    power.slide.to(0.0);
}


// a see-through window under the menu, so a click anywhere else closes it
pub fn scrim() -> LayerWindow {
    LayerWindow::new()
        .width(Full)
        .height(Full)
        .layer(Layer::Top)
        .namespace("power-scrim")
        .visible(Power::is_open())
        .child(Rectangle::new().width(Parent).height(Parent).on_click(|_| close()))
}

fn choose(index: usize) {
    let (_, _, sure) = ENTRIES[index];

    let confirmed = Power::read().confirm == Some(index);

    // the risky ones need a second click on the same row
    if sure && !confirmed {
        Power::write().confirm = Some(index);

        return;
    }

    close();

    match index {
        0 => {
            crate::lock::ipc(&[]);
        }
        1 => {
            // locked first, so waking up asks for the password
            crate::lock::ipc(&[]);
            amane::spawn("sleep 1 && systemctl suspend");
        }
        2 => amane::spawn("niri msg action quit --skip-confirmation"),
        3 => amane::spawn("systemctl reboot"),
        _ => amane::spawn("systemctl poweroff"),
    }
}

fn menu_height() -> f32 {
    let count = ENTRIES.len() as f32;

    PADDING * 2.0 + ROW_HEIGHT * count + GAP * (count - 1.0)
}

pub fn view() -> LayerWindow {
    let power = Power::read();

    let open = power.open;
    let hover = power.hover;
    let confirm = power.confirm;

    // 0 is hidden behind the bar, 1 is fully down
    let progress = power.slide.value();

    drop(power);

    let height = menu_height();

    let shown = open || progress > 0.001;

    let window = LayerWindow::new()
        .width(WIDTH + hang::FILLET)
        .height(height)
        .anchor_vertical(Vertical::Top)
        .anchor_horizontal(Horizontal::Left)
        .layer(Layer::Overlay)
        .namespace("power")
        .keyboard(if open { Keyboard::Exclusive } else { Keyboard::None })
        .on_key(key_pressed)
        .visible(shown);

    // while it slides away it must not catch clicks meant for other windows
    let window = if open { window } else { window.click_through() };

    if !shown {
        return window.child(Rectangle::new().width(Parent).height(Parent));
    }

    let mut rows: Vec<Box<dyn Widget>> = Vec::new();

    for index in 0..ENTRIES.len() {
        rows.push(Box::new(row(index, hover == Some(index), confirm == Some(index))));
    }

    // the same trick as the panel: taller than the window and raised, so the bar's
    // edge hides the rounded top corners and it looks like it comes from under the bar
    window.child(hang::hang(Column::new(rows).gap(GAP), WIDTH, height, RADIUS, PADDING, progress, false, true))
}

fn row(index: usize, hovered: bool, asking: bool) -> Rectangle {
    let (icon, label, _) = ENTRIES[index];

    let fill = if asking {
        theme::danger()
    } else if hovered {
        theme::hover()
    } else {
        Color::TRANSPARENT
    };

    let color = if asking { theme::on_accent() } else { theme::text() };

    let words = if asking { "Click to confirm" } else { label };

    let glyph = Rectangle::new()
        .width(ICON_SLOT)
        .height(ROW_HEIGHT)
        .align_child(Center, Center)
        .child(Text::new(icon).size(18.0).font(NERD_FONT).tight().color(color));

    let side = Rectangle::new().width(8.0).height(1.0);

    Rectangle::new()
        .width(INNER)
        .height(ROW_HEIGHT)
        .radius(16.0)
        .fill(fill)
        .cursor(Pointer)
        .align_child(Start, Center)
        .on_hover(move |inside| {
            let mut power = Power::write();

            if inside {
                power.hover = Some(index);
            } else if power.hover == Some(index) {
                power.hover = None;
            }
        })
        .on_click(move |_| choose(index))
        .child(
            Row::new(children![side, glyph, Text::new(words).size(15.0).color(color)])
                .gap(10.0)
                .align(Center),
        )
}

// other popups call this when they open, so only one is down at a time
pub fn dismiss() {
    if Power::is_open() {
        close();
    }
}

impl Power {
    pub fn logo_hovered() -> bool {
        Self::read().logo
    }
}

// the logo in the bar tells us when the pointer is on it
pub fn set_logo_hover(inside: bool) {
    Power::write().logo = inside;
}

// how wide the menu's window is and how far down it is, so the bar's group above it can grow to meet it
pub fn reach() -> (f32, f32) {
    let progress = Power::read().slide.value();

    (WIDTH + hang::FILLET, progress)
}

// the arrow keys (or j and k) move the highlight, Enter runs it, Escape closes
fn key_pressed(key: Key) {
    let (open, highlighted) = {
        let power = Power::read();

        (power.open, power.hover)
    };

    if !open {
        return;
    }

    let count = ENTRIES.len();

    match key {
        // wraps round at both ends, and starts from the first or last row if none is highlighted
        Key::Down | Key::Character('j') => {
            move_to(highlighted.map_or(0, |index| (index + 1) % count));
        }
        Key::Up | Key::Character('k') => {
            move_to(highlighted.map_or(count - 1, |index| (index + count - 1) % count));
        }
        Key::Enter => {
            if let Some(index) = highlighted {
                choose(index);
            }
        }
        Key::Escape => close(),
        _ => {}
    }
}

// highlights a row, and drops any "click again to confirm" waiting on another one
fn move_to(index: usize) {
    let mut power = Power::write();

    power.hover = Some(index);
    power.confirm = None;
}
