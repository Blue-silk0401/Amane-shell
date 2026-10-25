use std::time::Duration;

use amane::{
    Animation, Apps, Center, Color, Column, DesktopApp, Easing, Full, Image, Key, Keyboard, Layer,
    LayerWindow, Padding, Parent, Pointer, Rectangle, Row, Scroll, Service, Stack, Start, Text,
    TextInput, Vertical, Widget, children,
};

use amane::{Canvas, Path, Shape, shapes};
use crate::hang;
use crate::theme;

// the text input's name: amane keeps what was typed under it between redraws
const INPUT: &str = "launcher";

const WIDTH: f32 = 560.0;
const PADDING: f32 = 14.0;
const INNER_WIDTH: f32 = WIDTH - PADDING * 2.0;

const FIELD_HEIGHT: f32 = 46.0;
const ROW_HEIGHT: f32 = 48.0;
const ICON_SIZE: f32 = 32.0;
const GAP: f32 = 8.0;

const RADIUS: f32 = 20.0;
const INNER_RADIUS: f32 = 12.0;

// how many apps are shown at once, the rest are reached by scrolling
const MAX_ROWS: usize = 7;

// how many rows one turn of the mouse wheel moves (a touchpad gives fractions of it)
const ROWS_PER_NOTCH: f32 = 2.0;

const LIST_HEIGHT: f32 = ROW_HEIGHT * MAX_ROWS as f32;

const HEIGHT: f32 = PADDING * 2.0 + FIELD_HEIGHT + GAP + LIST_HEIGHT;



const SLIDE_MS: u64 = 320;

// what the launcher remembers: open or not, what was typed, which row is selected,
// which row is at the top of the list, and how far up the panel is
pub struct Launcher {
    open: bool,
    query: String,
    selected: usize,

    // the first app shown, so scrolling is a number that goes up and down
    top: usize,

    // the part of a row the wheel has moved so far, for touchpads
    wheel: f32,

    slide: Animation,
}

impl Service for Launcher {
    fn new() -> Self {
        Self {
            open: false,
            query: String::new(),
            selected: 0,
            top: 0,
            wheel: 0.0,
            slide: Animation::new(0.0)
                .duration(Duration::from_millis(SLIDE_MS))
                .easing(Easing::Out),
        }
    }

    // it only changes through input, so there is nothing to poll
    fn listen() {}
}

// called by `amane ipc call launcher`, bound to a key in niri
pub fn ipc(_: &[String]) -> String {
    let open = Launcher::read().open;

    if open {
        close();

        String::from("closed")
    } else {
        open_launcher();

        String::from("opened")
    }
}

fn open_launcher() {
    {
        let mut launcher = Launcher::write();

        launcher.open = true;
        launcher.query.clear();
        launcher.selected = 0;
        launcher.top = 0;
        launcher.wheel = 0.0;
        launcher.slide.to(1.0);
    }

    TextInput::set_text(INPUT, "");
}

fn close() {
    let mut launcher = Launcher::write();

    launcher.open = false;
    launcher.slide.to(0.0);
}

pub fn view() -> LayerWindow {
    let launcher = Launcher::read();

    let open = launcher.open;
    let query = launcher.query.clone();
    let selected = launcher.selected;
    let top = launcher.top;

    // 0 is hidden below the screen, 1 is fully up; reading it keeps frames coming while it moves
    let progress = launcher.slide.value();

    drop(launcher);

    // still shown while it slides back down
    let shown = open || progress > 0.001;

    // keys only reach the window while it is open
    let keyboard = if open { Keyboard::Exclusive } else { Keyboard::None };

    let window = LayerWindow::new()
        .width(WIDTH + hang::FILLET * 2.0)
        .height(HEIGHT)
        .anchor_vertical(Vertical::Bottom)
        .layer(Layer::Overlay)
        .keyboard(keyboard)
        .on_key(key_pressed)
        .visible(shown);

    // while it slides away it must not catch clicks meant for other windows
    let window = if open { window } else { window.click_through() };

    // fully hidden: nothing is built, so the search box is made fresh (and focused) on each open
    if !shown {
        return window.child(Rectangle::new().width(Parent).height(Parent));
    }

    let apps = matching(&query);

    // a shorter list after typing can't leave us scrolled past its end
    let top = top.min(apps.len().saturating_sub(MAX_ROWS));

    let field = Rectangle::new()
        .width(INNER_WIDTH)
        .height(FIELD_HEIGHT)
        .radius(INNER_RADIUS)
        .fill(theme::surface())
        .padding(Padding {
            top: 0.0,
            right: 14.0,
            bottom: 0.0,
            left: 14.0,
        })
        .align_child(Start, Center)
        .child(
            TextInput::new(INPUT)
                .size(16.0)
                .color(theme::text())
                .placeholder("Search apps...")
                .focused()
                .on_change(query_changed)
                .on_submit(|_| submit()),
        );

    // only the rows that fit are built, starting from `top`
    let mut rows: Vec<Box<dyn Widget>> = Vec::new();

    for (index, app) in apps.iter().enumerate().skip(top).take(MAX_ROWS) {
        rows.push(Box::new(row(app, index, index == selected)));
    }

    if rows.is_empty() {
        rows.push(Box::new(
            Text::new("No matching apps").size(14.0).color(theme::muted()),
        ));
    }

    // the wheel works anywhere over the list, the rows keep their own clicks
    let list = Rectangle::new()
        .width(INNER_WIDTH)
        .height(LIST_HEIGHT)
        .on_scroll(scrolled)
        .child(Stack::new(children![
            Column::new(rows),
            scrollbar(apps.len(), top)
        ]));

    // pushed down by the whole window height when hidden, so it starts and ends below the
    // screen's edge; the window's own edge clips it, which reads as sliding up from under
    window.child(floor_panel(Column::new(children![field, list]).gap(GAP), progress))
}

// every app whose name contains the query, names that start with it first
fn matching(query: &str) -> Vec<DesktopApp> {
    let query = query.to_lowercase();

    let apps = Apps::read();

    let mut found: Vec<DesktopApp> = apps
        .list()
        .iter()
        .filter(|app| app.name().to_lowercase().contains(&query))
        .cloned()
        .collect();

    // the sort is stable, so each group stays alphabetical
    found.sort_by_key(|app| !app.name().to_lowercase().starts_with(&query));

    found
}

// a thin bar at the right edge showing where the shown apps sit in the whole list
fn scrollbar(count: usize, top: usize) -> Rectangle {
    // nothing to scroll, so nothing to show
    if count <= MAX_ROWS {
        return Rectangle::new().width(1.0).height(1.0);
    }

    let thumb_height = (LIST_HEIGHT * MAX_ROWS as f32 / count as f32).max(24.0);

    let farthest = (count - MAX_ROWS) as f32;

    let y = (LIST_HEIGHT - thumb_height) * top as f32 / farthest;

    Rectangle::new()
        .width(4.0)
        .height(thumb_height)
        .radius(Full)
        .fill(theme::muted())
        .opacity(0.6)
        .translate(INNER_WIDTH - 6.0, y)
}

fn row(app: &DesktopApp, index: usize, selected: bool) -> Rectangle {
    let fill = if selected { theme::hover() } else { Color::TRANSPARENT };

    // the icon takes ICON_SIZE and a 12px gap, the row's own padding takes 24px more
    let name_width = INNER_WIDTH - ICON_SIZE - 12.0 - 24.0;

    let name = Rectangle::new()
        .width(name_width)
        .height(ROW_HEIGHT)
        .align_child(Start, Center)
        .child(Text::new(app.name()).size(15.0).color(theme::text()).elide());

    Rectangle::new()
        .width(INNER_WIDTH)
        .height(ROW_HEIGHT)
        .radius(INNER_RADIUS)
        .fill(fill)
        .cursor(Pointer)
        // the pointer moving onto a row selects it, so the highlight follows the mouse
        .on_hover(move |inside| {
            if inside {
                select(index);
            }
        })
        .on_click(move |_| launch_at(index))
        .padding(Padding {
            top: 0.0,
            right: 12.0,
            bottom: 0.0,
            left: 12.0,
        })
        .align_child(Start, Center)
        .child(Row::new(children![icon(app), name]).gap(12.0).align(Center))
}

// the app's icon file, or empty space so the names stay lined up
fn icon(app: &DesktopApp) -> Rectangle {
    let slot = Rectangle::new().width(ICON_SIZE).height(ICON_SIZE);

    let Some(path) = app.icon_path() else {
        return slot;
    };

    let readable = matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("png" | "jpg" | "jpeg" | "svg")
    );

    if !readable {
        return slot;
    }

    // twice the size, so it stays sharp on a scaled screen
    let pixels = (ICON_SIZE * 2.0) as u32;

    slot.fill(Image::contain(path).thumbnail(pixels, pixels))
}

fn query_changed(text: String) {
    let mut launcher = Launcher::write();

    launcher.query = text;

    // a new search starts at the top of the list
    launcher.selected = 0;
    launcher.top = 0;
    launcher.wheel = 0.0;
}

// the pointer is on a row: only writes when something changes, so it doesn't redraw for nothing
fn select(index: usize) {
    let current = Launcher::read().selected;

    if current != index {
        Launcher::write().selected = index;
    }
}

// the mouse wheel or touchpad over the list
fn scrolled(scroll: Scroll) {
    let (query, top, wheel) = {
        let launcher = Launcher::read();

        (launcher.query.clone(), launcher.top, launcher.wheel)
    };

    let farthest = matching(&query).len().saturating_sub(MAX_ROWS);

    // turning the wheel down makes y positive, which moves further down the list
    let wheel = wheel + scroll.y * ROWS_PER_NOTCH;

    // whole rows move the list, the leftover part waits for the next event
    let whole = wheel.trunc();

    let moved = top as i64 + whole as i64;

    let mut launcher = Launcher::write();

    launcher.top = moved.clamp(0, farthest as i64) as usize;
    launcher.wheel = wheel - whole;
}

// Enter in the search box
fn submit() {
    // copied out on its own line, so the read lock is gone before launch_at writes
    let selected = Launcher::read().selected;

    launch_at(selected);
}

// moves the selection, and the list along with it when the selection leaves the screen
fn move_selection(index: usize) {
    let mut launcher = Launcher::write();

    launcher.selected = index;

    if index < launcher.top {
        launcher.top = index;
    } else if index >= launcher.top + MAX_ROWS {
        launcher.top = index + 1 - MAX_ROWS;
    }
}

// the window's keys: up and down never reach the text box, so they land here
fn key_pressed(key: Key) {
    let (open, query, selected) = {
        let launcher = Launcher::read();

        (launcher.open, launcher.query.clone(), launcher.selected)
    };

    if !open {
        return;
    }

    let count = matching(&query).len();

    match key {
        Key::Down if count > 0 => move_selection((selected + 1) % count),
        Key::Up if count > 0 => move_selection((selected + count - 1) % count),
        Key::Escape => close(),
        _ => {}
    }
}

fn launch_at(index: usize) {
    let query = Launcher::read().query.clone();

    let apps = matching(&query);

    if let Some(app) = apps.get(index) {
        amane::spawn(&format!("setsid -f {} >/dev/null 2>&1 </dev/null", app.exec()));
    }

    close();
}

// a see-through window under the launcher, so a click anywhere else closes it
pub fn scrim() -> LayerWindow {
    let open = Launcher::read().open;

    LayerWindow::new()
        .width(Full)
        .height(Full)
        .layer(Layer::Top)
        .namespace("launcher-scrim")
        .visible(open)
        .child(Rectangle::new().width(Parent).height(Parent).on_click(|_| close()))
}

// the thickness of the outline, and the number that makes a cubic curve look like a quarter circle
const STROKE: f32 = 2.0;
const ARC: f32 = 0.5523;

/*
 * a panel standing on the screen's floor: its sides flare out into the floor with
 * inward curves, and the whole shape slides up from below the screen's edge. the
 * fill and the outline are the same path, so the outline follows every curve
 */
fn floor_panel(body: impl Widget + 'static, progress: f32) -> Rectangle {
    let fillet = hang::FILLET;

    let width = WIDTH + fillet * 2.0;

    // hidden below the screen's edge at 0, standing on the floor at 1
    let drop = (1.0 - progress) * HEIGHT;

    let canvas = Canvas::new().width(width).height(HEIGHT).shapes(shapes![
        silhouette(width).close().fill(theme::bg()),
        silhouette(width).stroke(STROKE, theme::outline()),
    ]);

    let content = Rectangle::new()
        .width(INNER_WIDTH)
        .height(HEIGHT - PADDING * 2.0)
        .translate(fillet + PADDING, PADDING)
        .child(body);

    Rectangle::new()
        .width(width)
        .height(HEIGHT)
        .translate(0.0, drop)
        .child(Stack::new(children![canvas, content]).width(width).height(HEIGHT))
}

// from the floor on the left, up the curve and the wall, round the top, and down to the floor on the right
fn silhouette(width: f32) -> Path {
    let fillet = hang::FILLET;
    let round = RADIUS;

    // the top edge sits half the outline's thickness down, so the outline isn't cut off by the window
    let top = STROKE / 2.0;

    let floor = HEIGHT;

    let left = fillet;
    let right = width - fillet;

    let curve = ARC * fillet;
    let corner = ARC * round;

    Path::new()
        .move_to(0.0, floor)
        .cubic_to(curve, floor, left, floor - fillet + curve, left, floor - fillet)
        .line_to(left, top + round)
        .cubic_to(left, top + round - corner, left + round - corner, top, left + round, top)
        .line_to(right - round, top)
        .cubic_to(right - round + corner, top, right, top + round - corner, right, top + round)
        .line_to(right, floor - fillet)
        .cubic_to(right, floor - fillet + curve, width - curve, floor, width, floor)
}
