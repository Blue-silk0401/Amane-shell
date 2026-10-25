mod colors;

use std::fs;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use amane::{
    Animation, Center, Color, Easing, Full, Image, Key, Keyboard, Layer, LayerWindow, Monitor,
    Parent, Pointer, Rectangle, Row, Scroll, Service, Stack, Text, Vertical, Widget, Zone,
    children,
};

use colors::{BUCKETS, Colors};

use crate::theme;

// where your wallpapers are, relative to your home folder
const FOLDER: &str = "Pictures/wallpapers";

// the wallpaper you used first, until you pick another
const FALLBACK: &str = "Cyberpunk-Toyko-Night.png";

// the chosen wallpaper is written here, so the lock screen and the next start use it
const STATE: &str = ".local/state/amane/wallpaper";

// the band across the screen, and the room at its top for the color bar
const HEIGHT: f32 = 470.0;
const TOP: f32 = 56.0;

// the picture in the middle, and the thin strips on both sides of it
const CENTER_SHARE: f32 = 0.35;
const CENTER_MAX: f32 = 680.0;
const STRIP: f32 = 76.0;
const GAP: f32 = 8.0;
const RADIUS: f32 = 14.0;

// pictures are kept small in memory, as big as the middle one
const THUMB_WIDTH: u32 = 560;
const THUMB_HEIGHT: u32 = 340;

const MOVE_MS: u64 = 380;
const FADE_MS: u64 = 520;

const BAR_HEIGHT: f32 = 36.0;
const SWATCH_SLOT: f32 = 24.0;
const COUNTER_WIDTH: f32 = 190.0;

pub struct Picker {
    shown: bool,

    // every wallpaper, in color order
    files: Vec<String>,

    // the one in the middle
    selected: usize,

    // which file is in the middle right now, a fraction while sliding
    position: Animation,

    // 0 hidden, 1 fully shown
    fade: Animation,

    // the part of a card the wheel has moved so far, for touchpads
    wheel: f32,
}

impl Service for Picker {
    fn new() -> Self {
        Self {
            shown: false,
            files: Vec::new(),
            selected: 0,
            position: moving(0.0),
            fade: Animation::new(0.0)
                .duration(Duration::from_millis(FADE_MS))
                .easing(Easing::Out),
            wheel: 0.0,
        }
    }

    // it only changes through input, so there is nothing to poll
    fn listen() {}
}

fn moving(start: f32) -> Animation {
    Animation::new(start)
        .duration(Duration::from_millis(MOVE_MS))
        .easing(Easing::Out)
}

// called once from main, so the colors start being found at startup, not at the first opening
pub fn warm_up() {
    drop(Colors::read());
    drop(Wallpaper::read());
}

fn home() -> String {
    std::env::var("HOME").unwrap_or_default()
}

fn folder() -> String {
    format!("{}/{FOLDER}", home())
}

fn state_file() -> String {
    format!("{}/{STATE}", home())
}

// the wallpaper in use: the one chosen last, or the first one if none was chosen
pub fn current() -> String {
    let saved = fs::read_to_string(state_file()).unwrap_or_default();

    let saved = saved.trim();

    if saved.is_empty() {
        format!("{}/{FALLBACK}", folder())
    } else {
        String::from(saved)
    }
}

// wraps text in single quotes for the shell, so odd file names can't break the command
fn quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

// runs a command, so only call it from handlers and service threads, never from view()
fn list_files() -> Vec<String> {
    let command = format!(
        "find {} -type f \\( -iname '*.png' -o -iname '*.jpg' -o -iname '*.jpeg' \\) | sort",
        quote(&folder())
    );

    amane::output(&command).lines().map(String::from).collect()
}


// `amane ipc call wallpaper`, bound to a key in niri
pub fn ipc(arguments: &[String]) -> String {
    let shown = Picker::read().shown;

    let command = arguments.first().map(String::as_str).unwrap_or("toggle");

    match command {
        "show" => show(),
        "hide" => hide(),
        _ if shown => hide(),
        _ => show(),
    }

    String::from("ok")
}

fn show() {
    // listing takes a moment, so it is done before anything is locked
    let files = list_files();
    let files = Colors::read().sorted(files);

    let current = current();

    let index = files.iter().position(|file| *file == current).unwrap_or(0);

    crate::panel::dismiss();
    crate::calendar::dismiss();
    crate::power::dismiss();

    let mut picker = Picker::write();

    picker.files = files;
    picker.selected = index;
    picker.position = moving(index as f32);
    picker.wheel = 0.0;
    picker.shown = true;
    picker.fade.to(1.0);
}

fn hide() {
    let mut picker = Picker::write();

    picker.shown = false;
    picker.fade.to(0.0);
}

// slides to a wallpaper
fn go_to(index: usize) {
    let mut picker = Picker::write();

    if picker.files.is_empty() {
        return;
    }

    let last = picker.files.len() - 1;

    picker.selected = index.min(last);

    let target = picker.selected as f32;

    picker.position.to(target);
}

fn step(by: i64) {
    let (selected, count) = {
        let picker = Picker::read();

        (picker.selected, picker.files.len())
    };

    if count == 0 {
        return;
    }

    let next = (selected as i64 + by).clamp(0, count as i64 - 1);

    go_to(next as usize);
}

// uses a wallpaper and closes the picker
fn pick(index: usize) {
    let path = Picker::read().files.get(index).cloned();

    hide();

    if let Some(path) = path {
        choose(&path);
    }
}

// the first wallpaper of a color, from a click on its swatch
fn jump_to_bucket(bucket: usize) {
    let index = {
        let picker = Picker::read();
        let colors = Colors::read();

        picker
            .files
            .iter()
            .position(|file| colors.bucket(file) == Some(bucket))
    };

    if let Some(index) = index {
        go_to(index);
    }
}

// the wheel or touchpad: one card per notch, a touchpad's small moves add up first
fn scrolled(scroll: Scroll) {
    let wheel = Picker::read().wheel + scroll.y;

    let whole = wheel.trunc();

    Picker::write().wheel = wheel - whole;

    step(whole as i64);
}

fn key_pressed(key: Key) {
    let (shown, selected) = {
        let picker = Picker::read();

        (picker.shown, picker.selected)
    };

    if !shown {
        return;
    }

    match key {
        Key::Left | Key::Character('h') => step(-1),
        Key::Right | Key::Character('l') => step(1),
        Key::Escape => hide(),
        Key::Enter => pick(selected),
        _ => {}
    }
}

pub fn view(monitor: &Monitor) -> LayerWindow {
    let width = monitor.width as f32;

    let picker = Picker::read();

    let shown = picker.shown;
    let fade = picker.fade.value();

    let visible = shown || fade > 0.001;

    let window = LayerWindow::new()
        .width(Full)
        .height(HEIGHT)
        .anchor_vertical(Vertical::Middle)
        .layer(Layer::Overlay)
        .space(Zone::Ignore)
        .keyboard(if shown { Keyboard::Exclusive } else { Keyboard::None })
        .namespace("wallpaper-picker")
        .visible(visible)
        .on_key(key_pressed);

    // while it fades away it must not catch clicks meant for other windows
    let window = if shown { window } else { window.click_through() };

    if !visible {
        return window.child(Rectangle::new().width(Parent).height(Parent));
    }

    if picker.files.is_empty() {
        return window.child(empty());
    }

    let count = picker.files.len();
    let selected = picker.selected.min(count - 1);
    let position = picker.position.value();

    let center_width = (width * CENTER_SHARE).min(CENTER_MAX);
    let center_height = (center_width * 0.6).min(HEIGHT - TOP - 24.0);
    let strip_height = center_height * 0.86;

    // the cards' middle line, under the color bar
    let middle = TOP + (HEIGHT - TOP) / 2.0;

    // how far a card's middle is from the screen's middle when it is the next one over
    let near = center_width / 2.0 + GAP + STRIP / 2.0;

    // only the cards that can be on screen are built
    let reach = (width / 2.0 / (STRIP + GAP)).ceil() as i64 + 3;
    let nearest = position.round() as i64;

    let first = (nearest - reach).max(0);
    let last = (nearest + reach).min(count as i64 - 1);

    let mut cards: Vec<(f32, Rectangle)> = Vec::new();

    for index in first..=last {
        let distance = index as f32 - position;
        let far = distance.abs();

        // a card narrows into a strip as it leaves the middle
        let blend = far.min(1.0);

        let (card_width, card_height) = if far <= 1.0 {
            (
                center_width * (1.0 - blend) + STRIP * blend,
                center_height * (1.0 - blend) + strip_height * blend,
            )
        } else {
            (
                (STRIP * (1.0 - 0.05 * (far - 1.0))).max(STRIP * 0.6),
                (strip_height * (1.0 - 0.02 * (far - 1.0))).max(strip_height * 0.8),
            )
        };

        let offset = if far <= 1.0 {
            far * near
        } else {
            near + (far - 1.0) * (STRIP + GAP)
        };

        let x = width / 2.0 + distance.signum() * offset;

        // off the screen, so not worth drawing
        if x + card_width / 2.0 < 0.0 || x - card_width / 2.0 > width {
            continue;
        }

        let darkness = (far.min(3.0) / 3.0 * 0.7 * 255.0) as u8;

        let card = card(
            &picker.files[index as usize],
            index as usize,
            index as usize == selected,
            card_width,
            card_height,
            x - card_width / 2.0,
            middle - card_height / 2.0,
            darkness,
        );

        cards.push((far, card));
    }

    // the middle card is drawn last, so it lies on top
    cards.sort_by(|first, second| second.0.total_cmp(&first.0));

    let mut layers: Vec<Box<dyn Widget>> = Vec::new();

    layers.push(Box::new(
        Rectangle::new()
            .width(Parent)
            .height(Parent)
            .fill(Color::rgba(0, 0, 0, 140))
            .on_click(|_| hide()),
    ));

    for (_, card) in cards {
        layers.push(Box::new(card));
    }

    // the name shows once the middle card has stopped
    if (position - selected as f32).abs() < 0.05 {
        layers.push(Box::new(label(
            &picker.files[selected],
            width / 2.0,
            middle + center_height / 2.0,
        )));
    }

    let (pending, known, active) = {
        let colors = Colors::read();

        (
            colors.pending(),
            colors.known(),
            colors.bucket(&picker.files[selected]),
        )
    };

    layers.push(Box::new(bar(width, selected, count, pending, known, active)));

    window.child(reveal(layers, width, fade))
}

fn empty() -> Rectangle {
    Rectangle::new()
        .width(Parent)
        .height(Parent)
        .fill(Color::rgba(0, 0, 0, 140))
        .on_click(|_| hide())
        .align_child(Center, Center)
        .child(
            Text::new(format!("No wallpapers found in {}", folder()))
                .size(16.0)
                .color(theme::text()),
        )
}

// a wallpaper: the middle one has an outline, the further ones are darker
fn card(
    path: &str,
    index: usize,
    centered: bool,
    width: f32,
    height: f32,
    x: f32,
    y: f32,
    darkness: u8,
) -> Rectangle {
    let shading = Rectangle::new()
        .width(Parent)
        .height(Parent)
        .radius(RADIUS)
        .fill(Color::rgba(0, 0, 0, darkness));

    let card = Rectangle::new()
        .width(width)
        .height(height)
        .radius(RADIUS)
        .fill(Image::cover(path).thumbnail(THUMB_WIDTH, THUMB_HEIGHT))
        .cursor(Pointer)
        .on_click(move |_| {
            if centered {
                pick(index);
            } else {
                go_to(index);
            }
        })
        .translate(x, y)
        .child(shading);

    if centered {
        card.border(3.0, theme::outline())
    } else {
        card
    }
}

// the file's name on the middle card
fn label(path: &str, x: f32, bottom: f32) -> Rectangle {
    let name: String = Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("")
        .replace(['_', '-'], " ")
        .to_uppercase()
        .chars()
        .take(38)
        .collect();

    let width = name.chars().count() as f32 * 8.0 + 28.0;

    Rectangle::new()
        .width(width)
        .height(26.0)
        .radius(Full)
        .fill(Color::rgba(0, 0, 0, 180))
        .align_child(Center, Center)
        .translate(x - width / 2.0, bottom - 26.0 - 14.0)
        .child(Text::new(name).size(12.0).color(theme::text()))
}

// the row of color swatches and the counter, at the top
fn bar(
    width: f32,
    selected: usize,
    count: usize,
    pending: usize,
    known: usize,
    active: Option<usize>,
) -> Rectangle {
    let mut swatches: Vec<Box<dyn Widget>> = Vec::new();

    for bucket in 0..BUCKETS {
        swatches.push(Box::new(swatch(bucket, active == Some(bucket))));
    }

    let text = if known == 0 && pending == 0 {
        String::from("needs imagemagick")
    } else if pending > 0 {
        format!("{}/{count}  sorting {pending}", selected + 1)
    } else {
        format!("{}/{count}", selected + 1)
    };

    let counter = Rectangle::new()
        .width(COUNTER_WIDTH)
        .height(BAR_HEIGHT)
        .align_child(Center, Center)
        .child(Text::new(text).size(12.0).color(theme::muted()));

    let swatches_width = BUCKETS as f32 * SWATCH_SLOT + (BUCKETS - 1) as f32 * 6.0;

    let bar_width = swatches_width + 16.0 + COUNTER_WIDTH + 32.0;

    Rectangle::new()
        .width(bar_width)
        .height(BAR_HEIGHT)
        .radius(Full)
        .fill(Color::rgba(0, 0, 0, 180))
        .align_child(Center, Center)
        .translate((width - bar_width) / 2.0, 12.0)
        .child(
            Row::new(children![Row::new(swatches).gap(6.0).align(Center), counter])
                .gap(16.0)
                .align(Center),
        )
}

// a color to jump to, bigger and outlined while the middle wallpaper is of that color
fn swatch(bucket: usize, active: bool) -> Rectangle {
    let size = if active { 20.0 } else { 14.0 };

    let dot = Rectangle::new()
        .width(size)
        .height(size)
        .radius(Full)
        .fill(colors::swatch(bucket));

    let dot = if active { dot.border(2.0, theme::outline()) } else { dot };

    Rectangle::new()
        .width(SWATCH_SLOT)
        .height(SWATCH_SLOT)
        .align_child(Center, Center)
        .cursor(Pointer)
        .on_click(move |_| jump_to_bucket(bucket))
        .child(dot)
}

/*
 * the picker grows out of the middle like a circle: a round clip, bigger each frame,
 * with the picker inside it held still. the window's own edges cut the circle once
 * it is bigger than the band, and when it is fully open the clip is dropped, since
 * a clip is a gpu layer that costs something every frame
 */
fn reveal(layers: Vec<Box<dyn Widget>>, width: f32, progress: f32) -> Rectangle {
    let content = Rectangle::new()
        .width(width)
        .height(HEIGHT)
        .on_scroll(scrolled)
        .child(Stack::new(layers).width(width).height(HEIGHT));

    if progress >= 0.999 {
        return content;
    }

    // a circle this wide, centered on the band, covers every corner of it
    let diagonal = (width * width + HEIGHT * HEIGHT).sqrt();

    let diameter = (diagonal * progress).max(1.0);

    // the circle's top left corner, so its middle is the band's middle
    let left = (width - diameter) / 2.0;
    let top = (HEIGHT - diameter) / 2.0;

    Rectangle::new()
        .width(diameter)
        .height(diameter)
        .radius(Full)
        .clip()
        .translate(left, top)
        // moved back by the same amount, so the picker doesn't travel with the circle
        .child(content.translate(-left, -top))
}

// a see-through window under the picker, so a click above or below the band closes it
pub fn scrim() -> LayerWindow {
    let shown = Picker::read().shown;

    LayerWindow::new()
        .width(Full)
        .height(Full)
        .layer(Layer::Top)
        .namespace("wallpaper-scrim")
        .visible(shown)
        .child(Rectangle::new().width(Parent).height(Parent).on_click(|_| hide()))
}

// how long the circle takes to grow over the old wallpaper
const REVEAL_MS: u64 = 900;

// where the circle starts, as a share of the screen's width and height
const ORIGIN: (f32, f32) = (0.5, 0.5);

// how big the circle is when it first appears, in pixels (0 starts it as a dot)
const START_RADIUS: f32 = 40.0;

// a wallpaper that never loads is shown anyway after this long
const LOAD_TIMEOUT: Duration = Duration::from_secs(3);

/*
 * the wallpaper on screen, and the new one growing over it. the picker only writes
 * the chosen path to a file; this service watches that file and does the rest
 */
pub struct Wallpaper {
    // the picture on screen
    shown: String,

    // the picture growing over it, while there is one
    incoming: Option<String>,

    // how far the circle has grown, 0 to 1
    reveal: Animation,
}

impl Service for Wallpaper {
    fn new() -> Self {
        // the file has to exist before it can be watched
        let file = state_file();

        if !Path::new(&file).exists() {
            if let Some(parent) = Path::new(&file).parent() {
                let _ = fs::create_dir_all(parent);
            }

            let _ = fs::write(&file, current());
        }

        Self {
            shown: current(),
            incoming: None,
            reveal: growing(),
        }
    }

    // runs on a thread of its own, so waiting and sleeping here is fine
    fn listen() {
        let changes = amane::watch_file(&state_file());

        for _ in changes {
            let path = current();

            let known = {
                let wallpaper = Self::read();

                path == wallpaper.shown
            };

            if known {
                continue;
            }

            swap_to(path);
        }
    }
}

fn growing() -> Animation {
    Animation::new(0.0)
        .duration(Duration::from_millis(REVEAL_MS))
        .easing(Easing::InOut)
}

// the new wallpaper grows over the old one, which is replaced once it is covered
fn swap_to(path: String) {
    // the circle would grow over nothing if the picture wasn't decoded yet
    let started = Instant::now();

    while !Image::loaded(&path) && started.elapsed() < LOAD_TIMEOUT {
        thread::sleep(Duration::from_millis(16));
    }

    {
        let mut wallpaper = Wallpaper::write();

        wallpaper.incoming = Some(path.clone());
        wallpaper.reveal = growing();
        wallpaper.reveal.to(1.0);
    }

    thread::sleep(Duration::from_millis(REVEAL_MS));

    let mut wallpaper = Wallpaper::write();

    wallpaper.shown = path;
    wallpaper.incoming = None;
}

// the picker's choice: written to a file, which the service above notices

// under everything, taking no clicks: the wallpaper itself
pub fn background(monitor: &Monitor) -> LayerWindow {
    let width = monitor.width as f32;
    let height = monitor.height as f32;

    let wallpaper = Wallpaper::read();

    let mut layers: Vec<Box<dyn Widget>> = vec![
        // seen only until the first picture is decoded
        Box::new(Rectangle::new().width(Parent).height(Parent).fill(theme::bg())),
        Box::new(
            Rectangle::new()
                .width(Parent)
                .height(Parent)
                .fill(Image::cover(&wallpaper.shown)),
        ),
    ];

    if let Some(incoming) = &wallpaper.incoming {
        layers.push(Box::new(circle(
            incoming,
            wallpaper.reveal.value(),
            width,
            height,
        )));
    }

    LayerWindow::new()
        .width(Full)
        .height(Full)
        .layer(Layer::Background)
        .space(Zone::Ignore)
        .namespace("wallpaper")
        .click_through()
        .child(Stack::new(layers).width(Parent).height(Parent))
}

/*
 * the new wallpaper, seen through a circle growing from ORIGIN. the picture inside
 * is moved back by as much as the circle is moved, so it stays where the old one is
 * while only the circle grows
 */
fn circle(path: &str, progress: f32, width: f32, height: f32) -> Rectangle {
    let x = ORIGIN.0 * width;
    let y = ORIGIN.1 * height;

    // far enough to reach the farthest corner, and a little past it
    let farthest_x = f32::max(x, width - x);
    let farthest_y = f32::max(y, height - y);

    let covering = f32::hypot(farthest_x, farthest_y) + 2.0;

    let radius = START_RADIUS + (covering - START_RADIUS).max(0.0) * progress;

    let picture = Rectangle::new()
        .width(width)
        .height(height)
        .fill(Image::cover(path))
        .translate(radius - x, radius - y);

    Rectangle::new()
        .width(radius * 2.0)
        .height(radius * 2.0)
        .radius(Full)
        .translate(x - radius, y - radius)
        .clip()
        .child(picture)
}

// the picker's choice: written to a file, which the wallpaper service notices
fn choose(path: &str) {
    let file = state_file();

    if let Some(parent) = Path::new(&file).parent() {
        let _ = fs::create_dir_all(parent);
    }

    let _ = fs::write(file, path);

    apply_theme(path);
}

// opens a picture in the Palette, so every color in the shell follows it
pub fn apply_theme(path: &str) {
    use amane::{Palette, Service};

    Palette::write().open(path, 16);

    write_cava();
    write_ghostty();
    write_niri();
    write_btop();
    write_nvim();
    write_gtk();
}

// the same, for the wallpaper that is already set when the shell starts
pub fn theme_now() {
    let path = std::path::PathBuf::from(current());

    apply_theme(&path.to_string_lossy());
}

// cava reads its colors from a theme file, so write one from the new palette and tell it to reload
fn write_cava() {
    let home = std::env::var("HOME").unwrap_or_default();
    let folder = format!("{home}/.config/cava/themes");

    let _ = fs::create_dir_all(&folder);
    let _ = fs::write(format!("{folder}/amane"), crate::theme::cava_theme());

    amane::spawn("pkill -USR2 -x cava");
}

// ghostty includes this file from its config, then reloads when it gets the signal
fn write_ghostty() {
    let home = std::env::var("HOME").unwrap_or_default();
    let folder = format!("{home}/.config/ghostty");

    let _ = fs::create_dir_all(&folder);
    let _ = fs::write(format!("{folder}/amane-colors"), crate::theme::ghostty_theme());

    amane::spawn("pkill -USR2 -x .ghostty-wrappe; pkill -USR2 -x ghostty");
}

// niri includes this file from its config and reloads by itself when it changes
fn write_niri() {
    let home = std::env::var("HOME").unwrap_or_default();
    let folder = format!("{home}/.config/niri");

    let _ = fs::create_dir_all(&folder);
    let _ = fs::write(format!("{folder}/amane-colors.kdl"), crate::theme::niri_theme());
}

// btop reads its colors from a theme file; the running one is asked to reload
fn write_btop() {
    let home = std::env::var("HOME").unwrap_or_default();
    let folder = format!("{home}/.config/btop/themes");

    let _ = fs::create_dir_all(&folder);
    let _ = fs::write(format!("{folder}/amane.theme"), crate::theme::btop_theme());

    amane::spawn("pkill -USR2 -x btop");
}

// neovim reads these colors from a lua file, and watches it for changes
fn write_nvim() {
    let home = std::env::var("HOME").unwrap_or_default();
    let folder = format!("{home}/.local/state/amane");

    let _ = fs::create_dir_all(&folder);
    let _ = fs::write(format!("{folder}/nvim-colors.lua"), crate::theme::nvim_theme());
}

// gtk apps read their accent from a css file; a file that isn't ours is left alone
fn write_gtk() {
    let home = std::env::var("HOME").unwrap_or_default();

    for version in ["gtk-4.0", "gtk-3.0"] {
        let folder = format!("{home}/.config/{version}");
        let file = format!("{folder}/gtk.css");

        let ours = match fs::read_to_string(&file) {
            Ok(existing) => existing.contains("written by amane"),
            Err(_) => true,
        };

        if !ours {
            continue;
        }

        let _ = fs::create_dir_all(&folder);
        let _ = fs::write(&file, crate::theme::gtk_theme());
    }
}
