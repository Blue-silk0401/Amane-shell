use std::time::Duration;

use amane::{
    Animation, Center, Column, Easing, End, Horizontal, Key, Keyboard, Layer, LayerWindow,
    Parent, Pointer, Rectangle, Row, Service, Stack, Start, Text, Vertical, Weight, Widget,
    children,
};

use crate::hang;
use crate::theme;

const WIDTH: f32 = 340.0;
const PADDING: f32 = 16.0;
const INNER: f32 = WIDTH - PADDING * 2.0;
const RADIUS: f32 = 24.0;

const SLIDE_MS: u64 = 380;
const MONTH_MS: u64 = 300;

const TITLE_HEIGHT: f32 = 32.0;
const WEEKDAYS_HEIGHT: f32 = 22.0;
const GAP: f32 = 8.0;
const CELL_GAP: f32 = 2.0;
const CELL_HEIGHT: f32 = 34.0;
const ROWS: usize = 6;

const GRID_HEIGHT: f32 = ROWS as f32 * CELL_HEIGHT + (ROWS - 1) as f32 * CELL_GAP;
const PAGE_HEIGHT: f32 = TITLE_HEIGHT + WEEKDAYS_HEIGHT + GRID_HEIGHT + GAP * 2.0;
const HEIGHT: f32 = PAGE_HEIGHT + PADDING * 2.0;

const NAV_SIZE: f32 = 30.0;
const NERD_FONT: &str = "JetBrainsMono Nerd Font";
const PREVIOUS: &str = "\u{f004d}";
const NEXT: &str = "\u{f0054}";

const WEEKDAYS: [&str; 7] = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

pub struct Calendar {
    open: bool,

    // the pointer is over the time in the bar
    hover: bool,

    // how far down the window is
    slide: Animation,

    // which month is chosen (0 is this one), and the picture of it sliding there
    offset: i64,
    month: Animation,

    // year, month and day
    today: (i64, i64, i64),
}

impl Service for Calendar {
    fn new() -> Self {
        Self {
            open: false,
            hover: false,
            slide: Animation::new(0.0)
                .duration(Duration::from_millis(SLIDE_MS))
                .easing(Easing::Out),
            offset: 0,
            month: month_animation(),
            today: today(),
        }
    }

    // only to notice that midnight passed
    fn interval() -> Duration {
        Duration::from_secs(60)
    }

    fn update(&mut self) -> bool {
        let now = today();

        if now == self.today {
            return false;
        }

        self.today = now;

        true
    }
}

impl Calendar {
    pub fn is_open() -> bool {
        Self::read().open
    }

    pub fn hovered() -> bool {
        Self::read().hover
    }
}

fn month_animation() -> Animation {
    Animation::new(0.0)
        .duration(Duration::from_millis(MONTH_MS))
        .easing(Easing::Out)
}

// prints something like "2026 10 20"
fn today() -> (i64, i64, i64) {
    let text = amane::output("date '+%Y %m %d'");

    let mut numbers = text.split_whitespace().filter_map(|part| part.parse::<i64>().ok());

    match (numbers.next(), numbers.next(), numbers.next()) {
        (Some(year), Some(month), Some(day)) => (year, month, day),
        _ => (1970, 1, 1),
    }
}

// the time in the bar tells us when the pointer is on it
pub fn set_hover(inside: bool) {
    Calendar::write().hover = inside;
}

pub fn toggle() {
    if Calendar::is_open() {
        close();
    } else {
        open_calendar();
    }
}

// `amane ipc call calendar`
pub fn ipc(_: &[String]) -> String {
    toggle();

    String::from("toggled")
}

// other popups call this when they open, so only one is down at a time
pub fn dismiss() {
    if Calendar::is_open() {
        close();
    }
}

fn open_calendar() {
    crate::panel::dismiss();
    crate::power::dismiss();

    let mut calendar = Calendar::write();

    calendar.open = true;
    calendar.offset = 0;
    calendar.month = month_animation();
    calendar.slide.to(1.0);
}

fn close() {
    let mut calendar = Calendar::write();

    calendar.open = false;
    calendar.slide.to(0.0);
}

fn key_pressed(key: Key) {
    if key == Key::Escape {
        close();
    }
}

// previous or next month
fn step_month(by: i64) {
    let mut calendar = Calendar::write();

    calendar.offset += by;

    let target = calendar.offset as f32;

    calendar.month.to(target);
}

// back to this month
fn go_home() {
    let mut calendar = Calendar::write();

    calendar.offset = 0;
    calendar.month.to(0.0);
}

// a see-through window under the calendar, so a click anywhere else closes it
pub fn scrim() -> LayerWindow {
    LayerWindow::new()
        .width(amane::Full)
        .height(amane::Full)
        .layer(Layer::Top)
        .namespace("calendar-scrim")
        .visible(Calendar::is_open())
        .child(Rectangle::new().width(Parent).height(Parent).on_click(|_| close()))
}

pub fn view() -> LayerWindow {
    let (open, progress, shown_month, offset, today) = {
        let calendar = Calendar::read();

        (
            calendar.open,
            calendar.slide.value(),
            calendar.month.value(),
            calendar.offset,
            calendar.today,
        )
    };

    let shown = open || progress > 0.001;

    // both sides get an inward curve, so the window is wider than the calendar
    let window = LayerWindow::new()
        .width(WIDTH + hang::FILLET * 2.0)
        .height(HEIGHT)
        .anchor_vertical(Vertical::Top)
        .anchor_horizontal(Horizontal::Middle)
        .layer(Layer::Overlay)
        .namespace("calendar")
        .keyboard(if open { Keyboard::OnDemand } else { Keyboard::None })
        .on_key(key_pressed)
        .visible(shown);

    let window = if open { window } else { window.click_through() };

    if !shown {
        return window.child(Rectangle::new().width(Parent).height(Parent));
    }

    /*
     * months sit a full width apart and slide to the chosen one, so while
     * sliding both the old and the new month are drawn
     */
    let mut pages: Vec<Box<dyn Widget>> = Vec::new();

    let nearest = shown_month.floor() as i64;

    for step in [nearest, nearest + 1] {
        let distance = step as f32 - shown_month;

        if distance.abs() >= 1.0 {
            continue;
        }

        pages.push(Box::new(
            month_page(today, step).translate(distance * INNER, 0.0),
        ));
    }

    let mut viewport = Rectangle::new()
        .width(INNER)
        .height(PAGE_HEIGHT)
        .child(Stack::new(pages));

    // a clip is a gpu layer, so only while a month slides past the edge
    if (shown_month - offset as f32).abs() > 0.001 {
        viewport = viewport.clip();
    }

    let navigation = Rectangle::new()
        .width(INNER)
        .height(NAV_SIZE)
        .align_child(End, Start)
        .child(
            Row::new(children![
                nav_button(PREVIOUS, -1),
                nav_button(NEXT, 1),
            ])
            .gap(6.0),
        );

    let body = Stack::new(children![viewport, navigation])
        .width(INNER)
        .height(PAGE_HEIGHT);

    window.child(hang::hang(
        body, WIDTH, HEIGHT, RADIUS, PADDING, progress, true, true,
    ))
}

fn nav_button(icon: &str, by: i64) -> Rectangle {
    Rectangle::new()
        .width(NAV_SIZE)
        .height(NAV_SIZE)
        .radius(10.0)
        .fill(theme::surface())
        .cursor(Pointer)
        .on_click(move |_| step_month(by))
        .align_child(Center, Center)
        .child(Text::new(icon).size(14.0).font(NERD_FONT).tight().color(theme::text()))
}

// the title, the weekday names and six weeks of days
fn month_page(today: (i64, i64, i64), step: i64) -> Rectangle {
    let (today_year, today_month, today_day) = today;

    // counted in months from year 0, so adding the step crosses years by itself
    let index = today_year * 12 + (today_month - 1) + step;

    let year = index.div_euclid(12);
    let month = index.rem_euclid(12) + 1;

    let name = Text::new(format!("{} {year}", MONTHS[(month - 1) as usize]))
        .size(16.0)
        .weight(Weight::SemiBold)
        .color(theme::text());

    let mut title = Rectangle::new()
        .width(INNER - NAV_SIZE * 2.0 - 12.0)
        .height(TITLE_HEIGHT)
        .align_child(Start, Center)
        .child(name);

    // clicking the title of another month leads back to this one
    if step != 0 {
        title = title.cursor(Pointer).on_click(|_| go_home());
    }

    let highlighted = if step == 0 { Some(today_day) } else { None };

    let column = Column::new(children![title, weekdays(), grid(year, month, highlighted)])
        .gap(GAP);

    Rectangle::new().width(INNER).height(PAGE_HEIGHT).child(column)
}

fn cell_width() -> f32 {
    (INNER - CELL_GAP * 6.0) / 7.0
}

fn weekdays() -> Row {
    let mut names: Vec<Box<dyn Widget>> = Vec::new();

    for name in WEEKDAYS {
        names.push(Box::new(
            Rectangle::new()
                .width(cell_width())
                .height(WEEKDAYS_HEIGHT)
                .align_child(Center, Center)
                .child(Text::new(name).size(11.0).color(theme::muted())),
        ));
    }

    Row::new(names).gap(CELL_GAP)
}

fn grid(year: i64, month: i64, today: Option<i64>) -> Column {
    let first = weekday_of(year, month, 1);
    let length = days_in_month(year, month);

    let (before_year, before_month) = if month == 1 { (year - 1, 12) } else { (year, month - 1) };
    let before = days_in_month(before_year, before_month);

    let mut rows: Vec<Box<dyn Widget>> = Vec::new();

    for week in 0..ROWS {
        let mut cells: Vec<Box<dyn Widget>> = Vec::new();

        for column in 0..7 {
            // 1 is the first of the month, 0 and less are the end of the month before
            let day = (week * 7 + column) as i64 - first + 1;

            let (number, this_month) = if day < 1 {
                (before + day, false)
            } else if day > length {
                (day - length, false)
            } else {
                (day, true)
            };

            let is_today = this_month && today == Some(number);

            cells.push(Box::new(cell(number, this_month, is_today)));
        }

        rows.push(Box::new(Row::new(cells).gap(CELL_GAP)));
    }

    Column::new(rows).gap(CELL_GAP)
}

fn cell(number: i64, this_month: bool, is_today: bool) -> Rectangle {
    let fill = if is_today { theme::accent() } else { amane::Color::TRANSPARENT };
    let color = if is_today { theme::on_accent() } else { theme::text() };

    let weight = if is_today { Weight::SemiBold } else { Weight::Light };

    // the days of the months before and after are faded
    let opacity = if this_month { 1.0 } else { 0.3 };

    Rectangle::new()
        .width(cell_width())
        .height(CELL_HEIGHT)
        .radius(10.0)
        .fill(fill)
        .opacity(opacity)
        .align_child(Center, Center)
        .child(Text::new(number.to_string()).size(13.0).weight(weight).color(color))
}

// 0 is Sunday
fn weekday_of(year: i64, month: i64, day: i64) -> i64 {
    const OFFSETS: [i64; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];

    let y = if month < 3 { year - 1 } else { year };

    (y + y / 4 - y / 100 + y / 400 + OFFSETS[(month - 1) as usize] + day).rem_euclid(7)
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
                29
            } else {
                28
            }
        }
    }
}

// how wide the calendar's window is and how far down it is, so the bar's group above it can grow to meet it
pub fn reach() -> (f32, f32) {
    let progress = Calendar::read().slide.value();

    (WIDTH + hang::FILLET * 2.0, progress)
}

// the gap between the widget and the left edge of the screen
const EDGE: f32 = 24.0;

// how far above the middle of the screen the card sits
const LIFT: f32 = 120.0;

/*
 * the calendar as a permanent widget on the desktop: the same month pages,
 * on a card at the middle of the left edge, with no dropdown behaviour
 */
pub fn widget(monitor: &amane::Monitor) -> LayerWindow {
    let (shown_month, offset, today) = {
        let calendar = Calendar::read();

        (calendar.month.value(), calendar.offset, calendar.today)
    };

    let mut pages: Vec<Box<dyn Widget>> = Vec::new();

    let nearest = shown_month.floor() as i64;

    for step in [nearest, nearest + 1] {
        let distance = step as f32 - shown_month;

        if distance.abs() >= 1.0 {
            continue;
        }

        pages.push(Box::new(month_page(today, step).translate(distance * INNER, 0.0)));
    }

    let mut viewport = Rectangle::new().width(INNER).height(PAGE_HEIGHT).child(Stack::new(pages));

    // a clip is a gpu layer, so only while a month slides past the edge
    if (shown_month - offset as f32).abs() > 0.001 {
        viewport = viewport.clip();
    }

    let navigation = Rectangle::new()
        .width(INNER)
        .height(NAV_SIZE)
        .align_child(End, Start)
        .child(Row::new(children![nav_button(PREVIOUS, -1), nav_button(NEXT, 1)]).gap(6.0));

    let body = Stack::new(children![viewport, navigation]).width(INNER).height(PAGE_HEIGHT);

    let card = Rectangle::new()
        .width(WIDTH)
        .height(HEIGHT + WEATHER)
        .radius(RADIUS)
        .fill(theme::bg())
        .padding(PADDING)
        .child(Column::new(children![weather_line(), body]));

    // from the top of the screen down to the card: the middle of the screen, less the card's half and the lift
    let top = ((monitor.height as f32 - HEIGHT) / 2.0 - LIFT).max(0.0);

    LayerWindow::new()
        .width(WIDTH + EDGE)
        .height(top + HEIGHT + WEATHER)
        .anchor_vertical(Vertical::Top)
        .anchor_horizontal(Horizontal::Left)
        .layer(Layer::Bottom)
        .space(amane::Zone::Ignore)
        .namespace("calendar-widget")
        .child(Column::new(children![
            Rectangle::new().width(Parent).height(top),
            Row::new(children![Rectangle::new().width(EDGE).height(HEIGHT + WEATHER), card]),
        ]))
}

// the line above the month: temperature and conditions
const WEATHER: f32 = 30.0;

fn weather_line() -> Rectangle {
    let text = crate::weather::Weather::read().text().to_string();

    Rectangle::new()
        .width(INNER)
        .height(WEATHER)
        .align_child(Start, Center)
        .child(Text::new(text).size(13.0).color(theme::muted()))
}
