use std::time::Duration;

use amane::{
    Animation, Audio, Bluetooth, Brightness, Column, Easing, Full, Horizontal, Key, Keyboard,
    Layer, LayerWindow, Link, Network, Notifications, Parent, Rectangle, Row, Service,
    Stack, Vertical, Widget, children,
};

use crate::notifications::Quiet;
use crate::hang;

mod alerts;
mod media;
pub mod dim;
mod slider;
mod toggles;
mod wifi;

pub const NERD_FONT: &str = "JetBrainsMono Nerd Font";

// icons from the Nerd Font, written as code points
pub const WIFI: &str = "\u{f05a9}";
pub const WIFI_OFF: &str = "\u{f05aa}";
pub const VOLUME: &str = "\u{f057e}";
pub const BELL: &str = "\u{f009a}";
const MICROPHONE: &str = "\u{f036c}";
const BLUETOOTH: &str = "\u{f00af}";
const BRIGHTNESS: &str = "\u{f00e0}";

const WIDTH: f32 = 380.0;
const PADDING: f32 = 16.0;
const INNER: f32 = WIDTH - PADDING * 2.0;
const GAP: f32 = 12.0;

// only the bottom corners show: the top ones are hidden behind the bar
const RADIUS: f32 = 24.0;

const SLIDE_MS: u64 = 380;
const PAGE_MS: u64 = 320;

// how tall the panel grows for the wifi and notifications tabs
const TAB_HEIGHT: f32 = 560.0;

// true: a new tab comes in from the right edge. false: from the left
const FROM_RIGHT: bool = true;

// the tabs you can go into from the main page
#[derive(Clone, Copy, PartialEq)]
enum Page {
    Wifi,
    Alerts,
}

pub struct Panel {
    open: bool,

    // the tab that is open, or was just open while it slides away
    page: Page,

    // in a tab, or on the main page
    inside: bool,

    // how far down the panel is
    slide: Animation,

    // 0 is the main page, 1 is the tab
    pages: Animation,
}

impl Service for Panel {
    fn new() -> Self {
        Self {
            open: false,
            page: Page::Wifi,
            inside: false,
            slide: Animation::new(0.0)
                .duration(Duration::from_millis(SLIDE_MS))
                .easing(Easing::Out),
            pages: page_animation(),
        }
    }

    // it only changes through clicks, so there is nothing to poll
    fn listen() {}
}

fn page_animation() -> Animation {
    Animation::new(0.0)
        .duration(Duration::from_millis(PAGE_MS))
        .easing(Easing::Out)
}

impl Panel {
    pub fn is_open() -> bool {
        Self::read().open
    }

    fn is_inside() -> bool {
        Self::read().inside
    }
}

// the bar's pill calls this
pub fn toggle() {
    let open = Panel::is_open();

    if open {
        close();
    } else {
        open_panel();
    }
}

// `amane ipc call panel`, so a niri key can open it too
pub fn ipc(_: &[String]) -> String {
    let open = Panel::is_open();

    if open {
        close();

        String::from("closed")
    } else {
        open_panel();

        String::from("opened")
    }
}

fn open_panel() {
    crate::power::dismiss();
    crate::calendar::dismiss();

    // each opening starts on the main page, on the current month
    wifi::reset();

    let mut panel = Panel::write();

    panel.open = true;
    panel.inside = false;
    panel.pages = page_animation();
    panel.slide.to(1.0);
}

fn close() {
    wifi::reset();

    let mut panel = Panel::write();

    panel.open = false;
    panel.slide.to(0.0);
}

// into the wifi tab
fn open_wifi() {
    wifi::reset();

    // new networks show up a few seconds after asking
    Network::scan();

    open_page(Page::Wifi);
}

// into the notifications tab
fn open_alerts() {
    open_page(Page::Alerts);
}

fn open_page(page: Page) {
    let mut panel = Panel::write();

    panel.page = page;
    panel.inside = true;
    panel.pages.to(1.0);
}

// back to the main page
fn back() {
    wifi::reset();

    let mut panel = Panel::write();

    panel.inside = false;
    panel.pages.to(0.0);
}

// escape goes back one step at a time: a password, then the tab, then the panel
fn key_pressed(key: Key) {
    if key != Key::Escape {
        return;
    }

    if wifi::asking() {
        wifi::reset();
    } else if Panel::is_inside() {
        back();
    } else {
        close();
    }
}

/*
 * a see-through window over the whole screen, just under the panel, only there while
 * the panel is open. a click anywhere on it lands here instead of on whatever is
 * underneath, and closes the panel. (the bar sits above it, so the pill still works.)
 */
pub fn scrim() -> LayerWindow {
    LayerWindow::new()
        .width(Full)
        .height(Full)
        .layer(Layer::Top)
        .namespace("panel-scrim")
        .visible(Panel::is_open())
        .child(Rectangle::new().width(Parent).height(Parent).on_click(|_| close()))
}

// every row added up, so the window is exactly as tall as the main page
fn window_height(has_brightness: bool) -> f32 {
    let sliders = if has_brightness { 2.0 } else { 1.0 };

    PADDING * 2.0
        + toggles::HEIGHT
        + slider::HEIGHT * sliders
        + GAP * sliders
        + if media::visible() { media::HEIGHT + GAP } else { 0.0 }
}

// a laptop's backlight if there is one, otherwise the monitor itself through ddcutil
fn brightness_level() -> Option<u8> {
    let brightness = Brightness::read();

    if brightness.present() {
        return Some(brightness.percent());
    }

    drop(brightness);

    dim::level()
}

pub fn view() -> LayerWindow {
    let panel = Panel::read();

    let open = panel.open;
    let page = panel.page;

    // 0 is hidden behind the bar, 1 is fully down
    let progress = panel.slide.value();

    // 0 is the main page, 1 is the tab
    let pages = panel.pages.value();

    drop(panel);

    let brightness = brightness_level();

    let height = window_height(brightness.is_some());

    // the tabs have more to show than the main page, so the panel grows for them
    let tab_height = TAB_HEIGHT.max(height);
    let panel_height = height + (tab_height - height) * pages;
    let window_tall = if pages > 0.001 { tab_height } else { height };

    // still shown while it slides back up
    let shown = open || progress > 0.001;

    // the window sits right under the bar, because windows keep out of the bar's reserved space
    let window = LayerWindow::new()
        .width(WIDTH + hang::FILLET)
        .height(window_tall)
        .anchor_vertical(Vertical::Top)
        .anchor_horizontal(Horizontal::Right)
        .layer(Layer::Overlay)
        .namespace("panel")
        // clicking inside lets Escape close it
        .keyboard(if open { Keyboard::OnDemand } else { Keyboard::None })
        .on_key(key_pressed)
        .visible(shown);

    // while it slides away it must not catch clicks meant for other windows
    let window = if open { window } else { window.click_through() };

    if !shown {
        return window.child(Rectangle::new().width(Parent).height(Parent));
    }

    let page_height = panel_height - PADDING * 2.0;

    let sign = if FROM_RIGHT { 1.0 } else { -1.0 };

    /*
     * the main page and the tab sit a whole panel width apart. changing tab
     * slides both across together, and the window's edges cut off whatever
     * leaves the panel. a page that is fully out of sight isn't even built.
     */
    let mut layers: Vec<Box<dyn Widget>> = Vec::new();

    if pages < 0.999 {
        layers.push(Box::new(
            Rectangle::new()
                .width(INNER)
                .height(page_height)
                .translate(-sign * pages * WIDTH, 0.0)
                .child(main_page(brightness)),
        ));
    }

    if pages > 0.001 {
        let tab = match page {
            Page::Wifi => wifi::view(INNER, page_height),
            Page::Alerts => alerts::view(INNER, page_height),
        };

        layers.push(Box::new(
            Rectangle::new()
                .width(INNER)
                .height(page_height)
                .translate(sign * (1.0 - pages) * WIDTH, 0.0)
                .child(tab),
        ));
    }

    let content = Stack::new(layers).width(INNER).height(page_height);

    /*
     * the panel is RADIUS taller than the window and sits RADIUS above it, so its
     * rounded top corners are cut off by the window's edge, right where the bar ends.
     * sliding it down from above makes it look like it comes out from under the bar.
     */
    window.child(hang::hang(content, WIDTH, panel_height, RADIUS, PADDING, progress, true, false))
}

fn main_page(brightness: Option<u8>) -> Column {
    let volume = Audio::read().volume();

    let mut rows: Vec<Box<dyn Widget>> = vec![
        Box::new(toggle_row()),
        Box::new(slider::view(VOLUME, volume, INNER, set_volume)),
    ];

    if let Some(percent) = brightness {
        rows.push(Box::new(slider::view(BRIGHTNESS, percent, INNER, set_brightness)));
    }


    if media::visible() {
        rows.push(Box::new(media::view(INNER)));
    }

    Column::new(rows).gap(GAP)
}

// wifi, microphone, notifications, bluetooth
fn toggle_row() -> Row {
    let width = (INNER - GAP * 3.0) / 4.0;

    let (wifi_on, wifi_label) = {
        let network = Network::read();

        let label = if !network.wifi_enabled() {
            String::from("Off")
        } else {
            match network.link() {
                Link::Wifi => String::from(network.ssid()),
                _ => String::from("Wi-Fi"),
            }
        };

        (network.wifi_enabled(), label)
    };

    let wifi_icon = if wifi_on { WIFI } else { WIFI_OFF };

    let microphone_on = !Audio::read().microphone_muted();
    let bluetooth_on = Bluetooth::read().powered();

    let stored = Notifications::read().list().len();

    let notifications_on = !Quiet::on();

    let alerts_label = if !notifications_on {
        String::from("Silent")
    } else if stored == 0 {
        String::from("No alerts")
    } else {
        format!("{stored} alerts")
    };

    Row::new(children![
        // opens the wifi tab, which has the on/off switch inside
        toggles::button(wifi_icon, &wifi_label, wifi_on, width, open_wifi),
        toggles::button(MICROPHONE, "Mic", microphone_on, width, Audio::toggle_microphone_mute),
        // opens the notifications tab, which has the popups switch inside
        toggles::button(BELL, &alerts_label, notifications_on, width, open_alerts),
        toggles::button(BLUETOOTH, "Bluetooth", bluetooth_on, width, toggle_bluetooth),
    ])
    .gap(GAP)
}

// the switch inside the wifi tab uses this
fn toggle_wifi() {
    let enabled = Network::read().wifi_enabled();

    Network::set_wifi(!enabled);
}

fn toggle_bluetooth() {
    let powered = Bluetooth::read().powered();

    Bluetooth::set_powered(!powered);
}

// dragging reports on every move, but the sound server only needs a call when the number changes
fn set_volume(percent: u8) {
    if percent != Audio::read().volume() {
        Audio::set_volume(percent);
    }
}

// never below 1%, a black screen is hard to undo
fn set_brightness(percent: u8) {
    let percent = percent.max(1);

    let (backlight, current) = {
        let brightness = Brightness::read();

        (brightness.present(), brightness.percent())
    };

    if backlight {
        if percent != current {
            Brightness::set(percent);
        }
    } else {
        dim::set(percent);
    }
}

// other popups call this when they open, so only one is down at a time
pub fn dismiss() {
    if Panel::is_open() {
        close();
    }
}

// how wide the panel's window is and how far down it is, so the bar's group above it can grow to meet it
pub fn reach() -> (f32, f32) {
    let progress = Panel::read().slide.value();

    (WIDTH + hang::FILLET, progress)
}
