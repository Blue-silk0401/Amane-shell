use std::time::Duration;

use amane::{
    Animation, Center, Column, Easing, Full, Horizontal, Key, Keyboard, Layer, LayerWindow, Parent,
    Pointer, Rectangle, Service, Start, Text, Vertical, Widget, Zone, children,
};

use crate::theme;

const WIDTH: f32 = 520.0;
const PADDING: f32 = 12.0;
const HEADER: f32 = 34.0;
const ROW: f32 = 38.0;
const GAP: f32 = 2.0;
const MAX_ROWS: usize = 10;
const FADE_MS: u64 = 160;

pub struct Clipboard {
    open: bool,

    // id and one line of text for each saved copy, newest first
    entries: Vec<(String, String)>,

    selected: usize,
    fade: Animation,
}

impl Service for Clipboard {
    fn new() -> Self {
        Self {
            open: false,
            entries: Vec::new(),
            selected: 0,
            fade: Animation::new(0.0)
                .duration(Duration::from_millis(FADE_MS))
                .easing(Easing::Out),
        }
    }

    fn interval() -> Duration {
        Duration::from_secs(3600)
    }
}

impl Clipboard {
    pub fn is_open() -> bool {
        Self::read().open
    }
}

// `cliphist list` prints "id<tab>text", newest first
fn load() -> Vec<(String, String)> {
    let text = amane::output("cliphist list");

    let mut entries = Vec::new();

    for line in text.lines() {
        let Some((id, preview)) = line.split_once('\t') else {
            continue;
        };

        // the id goes into a shell command later, so only plain numbers are accepted
        if id.is_empty() || !id.chars().all(|character| character.is_ascii_digit()) {
            continue;
        }

        entries.push((id.to_string(), preview.trim().to_string()));

        if entries.len() == MAX_ROWS {
            break;
        }
    }

    entries
}

pub fn toggle() {
    if Clipboard::is_open() {
        close();
    } else {
        open_menu();
    }
}

// `amane ipc call clipboard`, bound to a key in niri
pub fn ipc(_: &[String]) -> String {
    toggle();

    String::from("toggled")
}

fn open_menu() {
    crate::panel::dismiss();
    crate::calendar::dismiss();
    crate::power::dismiss();

    let entries = load();

    let mut clipboard = Clipboard::write();

    clipboard.open = true;
    clipboard.entries = entries;
    clipboard.selected = 0;
    clipboard.fade.to(1.0);
}

fn close() {
    let mut clipboard = Clipboard::write();

    clipboard.open = false;
    clipboard.fade.to(0.0);
}

// puts a saved copy back on the clipboard and closes the picker
fn choose(index: usize) {
    let id = Clipboard::read().entries.get(index).map(|entry| entry.0.clone());

    let Some(id) = id else {
        return;
    };

    close();

    amane::spawn(&format!("cliphist decode {id} | wl-copy"));
}

fn select(index: usize) {
    Clipboard::write().selected = index;
}

fn key_pressed(key: Key) {
    let (count, selected) = {
        let clipboard = Clipboard::read();

        (clipboard.entries.len(), clipboard.selected)
    };

    match key {
        Key::Escape => close(),
        Key::Down if count > 0 => select((selected + 1).min(count - 1)),
        Key::Up if count > 0 => select(selected.saturating_sub(1)),
        Key::Enter => choose(selected),
        _ => {}
    }
}

// a see-through window under the picker, so a click anywhere else closes it
pub fn scrim() -> LayerWindow {
    LayerWindow::new()
        .width(Full)
        .height(Full)
        .layer(Layer::Top)
        .namespace("clipboard-scrim")
        .visible(Clipboard::is_open())
        .child(Rectangle::new().width(Parent).height(Parent).on_click(|_| close()))
}

pub fn view() -> LayerWindow {
    let (open, progress, entries, selected) = {
        let clipboard = Clipboard::read();

        (clipboard.open, clipboard.fade.value(), clipboard.entries.clone(), clipboard.selected)
    };

    let shown = open || progress > 0.001;

    let rows = entries.len().max(1);
    let height = PADDING * 2.0 + HEADER + rows as f32 * ROW + (rows - 1) as f32 * GAP;

    let window = LayerWindow::new()
        .width(WIDTH)
        .height(height)
        .anchor_vertical(Vertical::Middle)
        .anchor_horizontal(Horizontal::Middle)
        .layer(Layer::Overlay)
        .space(Zone::Ignore)
        .namespace("amane-clipboard")
        .keyboard(if open { Keyboard::Exclusive } else { Keyboard::None })
        .on_key(key_pressed)
        .visible(shown);

    if !shown {
        return window.click_through().child(Rectangle::new().width(Parent).height(Parent));
    }

    let inner = WIDTH - PADDING * 2.0;

    let clear = Rectangle::new()
        .width(64.0)
        .height(26.0)
        .radius(10.0)
        .fill(theme::surface())
        .cursor(Pointer)
        .on_click(|_| clear_all())
        .align_child(Center, Center)
        .child(Text::new("Clear").size(12.0).color(theme::text()));

    let title = Rectangle::new()
        .width(inner - 64.0)
        .height(HEADER)
        .align_child(Start, Center)
        .child(Text::new("Clipboard").size(13.0).color(theme::muted()));

    let header = amane::Row::new(children![
        title,
        Rectangle::new().width(64.0).height(HEADER).align_child(Center, Center).child(clear),
    ]);

    let mut list: Vec<Box<dyn Widget>> = Vec::new();

    if entries.is_empty() {
        list.push(Box::new(
            Rectangle::new()
                .width(inner)
                .height(ROW)
                .align_child(Start, Center)
                .child(Text::new("Nothing copied yet").size(14.0).color(theme::muted())),
        ));
    }

    for (index, (_, text)) in entries.iter().enumerate() {
        let fill = if index == selected { theme::hover() } else { amane::Color::TRANSPARENT };

        list.push(Box::new(
            Rectangle::new()
                .width(inner)
                .height(ROW)
                .radius(10.0)
                .fill(fill)
                .padding(10.0)
                .cursor(Pointer)
                .on_hover(move |inside| {
                    if inside {
                        select(index);
                    }
                })
                .on_click(move |_| choose(index))
                .align_child(Start, Center)
                .child(Text::new(text.clone()).size(14.0).color(theme::text()).elide()),
        ));
    }

    let card = Rectangle::new()
        .width(WIDTH)
        .height(height)
        .radius(20.0)
        .fill(theme::bg())
        .border(1.0, theme::surface())
        .opacity(progress.clamp(0.0, 1.0))
        .padding(PADDING)
        .child(Column::new(children![header, Column::new(list).gap(GAP)]));

    window.child(card)
}

// forgets every saved copy
fn clear_all() {
    amane::spawn("cliphist wipe");

    let mut clipboard = Clipboard::write();

    clipboard.entries.clear();
    clipboard.selected = 0;
}
