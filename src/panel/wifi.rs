use std::cell::RefCell;
use std::time::Duration;

use amane::{
    Animation, Center, Color, Column, Easing, Full, Network, Padding, Pointer, Rectangle, Row,
    ScrollArea, Service, Stack, Start, Text, TextInput, Widget, children,
};

use super::NERD_FONT;
use crate::theme;

// the password box's name: amane keeps what was typed under it between redraws
const PASSWORD: &str = "wifi-password";

// the list's name: amane keeps how far it is scrolled under it
const LIST: &str = "wifi-list";

const GAP: f32 = 10.0;
const HEADER_HEIGHT: f32 = 40.0;
const BUTTON_SIZE: f32 = 36.0;

// the on/off switch: a pill with a ball that slides along it
const SWITCH_WIDTH: f32 = 52.0;
const SWITCH_HEIGHT: f32 = 30.0;
const KNOB: f32 = 22.0;
const KNOB_MARGIN: f32 = (SWITCH_HEIGHT - KNOB) / 2.0;
const KNOB_TRAVEL: f32 = SWITCH_WIDTH - KNOB - KNOB_MARGIN * 2.0;
const SWITCH_MS: u64 = 180;

const CARD_HEIGHT: f32 = 60.0;
const CARD_PADDING: f32 = 12.0;
const CARD_GAP: f32 = 6.0;
const PASSWORD_HEIGHT: f32 = 40.0;
const JOIN_WIDTH: f32 = 64.0;

// strong to weak
const SIGNAL: [&str; 4] = ["\u{f0928}", "\u{f0925}", "\u{f0922}", "\u{f091f}"];
const LOCK: &str = "\u{f033e}";
const CHECK: &str = "\u{f012c}";
const BACK: &str = "\u{f004d}";

thread_local! {
    /*
     * how far the knob has slid, 0 is off and 1 is on. it changes on every frame while
     * it moves, so it lives here and not in a Service. it starts empty and is created on
     * first use at the right spot, so opening the tab never plays a slide by itself.
     */
    static KNOB_POSITION: RefCell<Option<Animation>> = RefCell::new(None);
}

// which network is asking for a password, and what was typed so far
pub struct Wifi {
    asking: Option<String>,
    typed: String,
}

impl Service for Wifi {
    fn new() -> Self {
        Self {
            asking: None,
            typed: String::new(),
        }
    }

    fn listen() {}
}

pub fn asking() -> bool {
    Wifi::read().asking.is_some()
}

// forgets any half-typed password
pub fn reset() {
    {
        let mut wifi = Wifi::write();

        wifi.asking = None;
        wifi.typed.clear();
    }

    TextInput::set_text(PASSWORD, "");
}

// a copy of one network, so the click handlers don't borrow from the Service
struct Entry {
    ssid: String,
    strength: u8,
    secured: bool,
    active: bool,
    saved: bool,
}

pub fn view(width: f32, height: f32) -> Column {
    let (enabled, connecting, mut entries) = {
        let network = Network::read();

        let entries: Vec<Entry> = network
            .access_points()
            .iter()
            // a hidden network has no name to show
            .filter(|access_point| !access_point.ssid().is_empty())
            .map(|access_point| Entry {
                ssid: String::from(access_point.ssid()),
                strength: access_point.strength(),
                secured: access_point.secured(),
                active: access_point.active(),
                saved: access_point.saved(),
            })
            .collect();

        (network.wifi_enabled(), network.connecting(), entries)
    };

    // the connected one first, then the strongest
    entries.sort_by(|a, b| b.active.cmp(&a.active).then(b.strength.cmp(&a.strength)));

    let asking = Wifi::read().asking.clone();

    let body_height = height - HEADER_HEIGHT - GAP;

    let body: Box<dyn Widget> = if !enabled {
        Box::new(message("Wi-Fi is off", width, body_height))
    } else if entries.is_empty() {
        Box::new(message("Looking for networks...", width, body_height))
    } else {
        let mut cards: Vec<Box<dyn Widget>> = Vec::new();

        for entry in &entries {
            let wants_password = asking.as_deref() == Some(entry.ssid.as_str());

            cards.push(Box::new(card(entry, wants_password, width)));
        }

        Box::new(
            Rectangle::new()
                .width(width)
                .height(body_height)
                .child(ScrollArea::new(LIST, Column::new(cards).gap(CARD_GAP))),
        )
    };

    let items: Vec<Box<dyn Widget>> = vec![Box::new(header(enabled, connecting, width)), body];

    Column::new(items).gap(GAP)
}

// back arrow, title and the on/off switch
fn header(enabled: bool, connecting: bool, width: f32) -> Row {
    let back = Rectangle::new()
        .width(BUTTON_SIZE)
        .height(BUTTON_SIZE)
        .radius(12.0)
        .fill(theme::surface())
        .cursor(Pointer)
        .on_click(|_| super::back())
        .align_child(Center, Center)
        .child(
            Text::new(BACK)
                .size(16.0)
                .font(NERD_FONT)
                .tight()
                .color(theme::text()),
        );

    let title_width = width - BUTTON_SIZE - SWITCH_WIDTH - GAP * 2.0;

    let title = Rectangle::new()
        .width(title_width)
        .height(HEADER_HEIGHT)
        .align_child(Start, Center)
        .child(
            Text::new(if connecting { "Connecting..." } else { "Wi-Fi" })
                .size(16.0)
                .color(theme::text()),
        );

    Row::new(children![back, title, switch(enabled)])
        .gap(GAP)
        .align(Center)
}

/*
 * a dim pill with a lit one laid over it, and a ball on top. turning it on fades the
 * lit pill in while the ball slides right, and off does the same backwards
 */
fn switch(enabled: bool) -> Stack {
    let target: f32 = if enabled { 1.0 } else { 0.0 };

    let amount = KNOB_POSITION.with_borrow_mut(|slot| {
        let knob = slot.get_or_insert_with(|| {
            Animation::new(target)
                .duration(Duration::from_millis(SWITCH_MS))
                .easing(Easing::Out)
        });

        knob.to(target);

        knob.value()
    });

    // this one takes the click, the two on top of it only draw
    let dim = Rectangle::new()
        .width(SWITCH_WIDTH)
        .height(SWITCH_HEIGHT)
        .radius(Full)
        .fill(theme::hover())
        .cursor(Pointer)
        .on_click(|_| super::toggle_wifi());

    let lit = Rectangle::new()
        .width(SWITCH_WIDTH)
        .height(SWITCH_HEIGHT)
        .radius(Full)
        .fill(theme::accent())
        .opacity(amount);

    let ball = Rectangle::new()
        .width(KNOB)
        .height(KNOB)
        .radius(Full)
        .fill(theme::text())
        .translate(KNOB_MARGIN + amount * KNOB_TRAVEL, KNOB_MARGIN);

    Stack::new(children![dim, lit, ball])
        .width(SWITCH_WIDTH)
        .height(SWITCH_HEIGHT)
}

fn message(text: &str, width: f32, height: f32) -> Rectangle {
    Rectangle::new()
        .width(width)
        .height(height)
        .align_child(Center, Center)
        .child(Text::new(text).size(14.0).color(theme::muted()))
}

// one network: a row to click, and a password row under it while it asks
fn card(entry: &Entry, wants_password: bool, width: f32) -> Rectangle {
    let inner = width - CARD_PADDING * 2.0;

    let ssid = entry.ssid.clone();
    let (secured, saved, active) = (entry.secured, entry.saved, entry.active);

    let signal = match entry.strength {
        76.. => SIGNAL[0],
        51..=75 => SIGNAL[1],
        26..=50 => SIGNAL[2],
        _ => SIGNAL[3],
    };

    let status = if active {
        "Connected, click to disconnect"
    } else if saved {
        "Saved"
    } else if secured {
        "Secured"
    } else {
        "Open"
    };

    // the right side: a check for the connected one, a lock for one that needs a password
    let marker = if active {
        Some((CHECK, theme::accent()))
    } else if secured && !saved {
        Some((LOCK, theme::muted()))
    } else {
        None
    };

    let text_width = inner - 22.0 - 22.0 - GAP * 2.0;

    let names = Column::new(children![
        line(&entry.ssid, 14.0, theme::text(), 18.0, text_width),
        line(status, 11.0, theme::muted(), 14.0, text_width),
    ])
    .gap(1.0);

    let top = Rectangle::new()
        .width(inner)
        .height(CARD_HEIGHT - CARD_PADDING * 2.0)
        .cursor(Pointer)
        .on_click(move |_| card_clicked(&ssid, secured, saved, active))
        .align_child(Start, Center)
        .child(
            Row::new(children![
                glyph(signal, theme::text()),
                names,
                match marker {
                    Some((icon, color)) => glyph(icon, color),
                    None => glyph("", theme::text()),
                },
            ])
            .gap(GAP)
            .align(Center),
        );

    let mut rows: Vec<Box<dyn Widget>> = vec![Box::new(top)];

    let mut height = CARD_HEIGHT;

    if wants_password {
        rows.push(Box::new(password_row(&entry.ssid, inner)));

        height += PASSWORD_HEIGHT + 8.0;
    }

    Rectangle::new()
        .width(width)
        .height(height)
        .radius(14.0)
        .fill(if active { theme::hover() } else { theme::surface() })
        .padding(CARD_PADDING)
        .child(Column::new(rows).gap(8.0))
}

fn glyph(icon: &str, color: Color) -> Rectangle {
    Rectangle::new()
        .width(22.0)
        .height(22.0)
        .align_child(Center, Center)
        .child(Text::new(icon).size(18.0).font(NERD_FONT).tight().color(color))
}

// one line of text in a box, cut off with "..." instead of spilling out
fn line(text: &str, size: f32, color: Color, height: f32, width: f32) -> Rectangle {
    Rectangle::new()
        .width(width)
        .height(height)
        .align_child(Start, Center)
        .child(Text::new(text).size(size).color(color).elide())
}

// the password box and the Join button
fn password_row(ssid: &str, width: f32) -> Row {
    let field_width = width - JOIN_WIDTH - 8.0;

    let for_enter = String::from(ssid);

    let input = TextInput::new(PASSWORD)
        .size(13.0)
        .color(theme::text())
        .placeholder("Password")
        .password()
        .focused()
        .on_change(|text| Wifi::write().typed = text)
        .on_submit(move |text| join(&for_enter, &text));

    let field = Rectangle::new()
        .width(field_width)
        .height(PASSWORD_HEIGHT)
        .radius(12.0)
        .fill(theme::bg())
        .padding(Padding {
            top: 0.0,
            right: 12.0,
            bottom: 0.0,
            left: 12.0,
        })
        .align_child(Start, Center)
        .child(input);

    let for_button = String::from(ssid);

    let button = Rectangle::new()
        .width(JOIN_WIDTH)
        .height(PASSWORD_HEIGHT)
        .radius(12.0)
        .fill(theme::accent())
        .cursor(Pointer)
        .align_child(Center, Center)
        .on_click(move |_| {
            // copied out on its own line, so the lock is gone before join writes
            let typed = Wifi::read().typed.clone();

            join(&for_button, &typed);
        })
        .child(Text::new("Join").size(13.0).color(theme::on_accent()));

    Row::new(children![field, button]).gap(8.0)
}

// the connected one disconnects, a saved or open one joins at once, any other asks for a password
fn card_clicked(ssid: &str, secured: bool, saved: bool, active: bool) {
    if active {
        Network::disconnect();

        return;
    }

    if saved || !secured {
        Network::connect(ssid, None);

        return;
    }

    // already asking about this one: leave what was typed alone
    let already = Wifi::read().asking.as_deref() == Some(ssid);

    if already {
        return;
    }

    reset();

    Wifi::write().asking = Some(String::from(ssid));
}

fn join(ssid: &str, password: &str) {
    let password = if password.is_empty() { None } else { Some(password) };

    Network::connect(ssid, password);

    reset();
}
