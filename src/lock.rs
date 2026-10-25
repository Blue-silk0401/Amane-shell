use amane::{
    Center, Color, Column, Full, Image, LayerWindow, Lock, Monitor, Rectangle, Row, Service, Stack,
    Start, Text, TextInput, Weight, Widget, children,
};

use crate::clock::Clock;
use crate::theme;


// the picture is shrunk to this size and then blurred, so a small radius is already heavy
const BLUR: u32 = 1;

// how dark the picture is made, so the clock and pill always read
const DARKEN: f32 = 0.10;

// the password box's name: amane keeps what was typed under it
const INPUT: &str = "lock-password";

const PILL_WIDTH: f32 = 320.0;
const PILL_HEIGHT: f32 = 52.0;

// each typed letter is one dot, and only this many fit in the pill
const DOT: f32 = 8.0;
const DOT_GAP: f32 = 8.0;
const MAX_DOTS: usize = 17;

const CLOCK_HEIGHT: f32 = 140.0;

// how far down the screen the clock and the pill start
const CLOCK_AT: f32 = 0.14;
const PILL_AT: f32 = 0.60;

// how many letters are typed so far. only the count is kept, never the password itself
pub struct Typing {
    count: usize,
}

impl Service for Typing {
    fn new() -> Self {
        Self { count: 0 }
    }

    fn listen() {}
}

// `amane ipc call lock`, bound to a key in niri
pub fn ipc(_: &[String]) -> String {
    // every lock starts with an empty box
    TextInput::set_text(INPUT, "");
    Typing::write().count = 0;

    Lock::start();

    String::from("locked")
}

// shown on every monitor while the session is locked
pub fn view(monitor: &Monitor) -> LayerWindow {
    let width = monitor.width as f32;
    let height = monitor.height as f32;

    let time = Clock::read().time().to_string();
    let count = Typing::read().count;

    let (checking, failed) = {
        let lock = Lock::read();

        (lock.checking(), lock.failed())
    };

    // a plain color underneath, so a missing picture still looks fine
    let backdrop = Rectangle::new().width(width).height(height).fill(theme::bg());


    let picture = Rectangle::new().width(width).height(height).fill(
        Image::cover(crate::wallpaper::current())
            .thumbnail(64, 36)
            .blurred(BLUR),
    );

    let shade = Rectangle::new()
        .width(width)
        .height(height)
        .fill(Color::BLACK)
        .opacity(DARKEN);

    // the clock, in the upper middle
    let clock = Rectangle::new()
        .width(width)
        .height(CLOCK_HEIGHT)
        .align_child(Center, Center)
        .translate(0.0, height * CLOCK_AT)
        .child(
            Text::new(time)
                .size(120.0)
                .weight(Weight::Light)
                .color(theme::text()),
        );

    // one line under the pill, always the same height so nothing jumps
    let status = Rectangle::new()
        .width(PILL_WIDTH)
        .height(22.0)
        .align_child(Center, Center)
        .child(if checking {
            Text::new("Checking...").size(14.0).color(theme::muted())
        } else if failed {
            Text::new("Wrong password").size(14.0).color(theme::danger())
        } else {
            Text::new(" ").size(14.0).color(theme::muted())
        });

    let input = Rectangle::new()
        .width(width)
        .height(PILL_HEIGHT + 12.0 + 22.0)
        .align_child(Center, Start)
        .translate(0.0, height * PILL_AT)
        .child(
            Column::new(children![pill(count, failed), status])
                .gap(12.0)
                .align(Center),
        );

    LayerWindow::new().width(1.0).height(1.0).child(
        Stack::new(children![backdrop, picture, shade, clock, input])
            .width(width)
            .height(height),
    )
}

/*
 * three layers: the pill shape, the real text box (invisible, but it is what takes
 * the typing), and the dots drawn on top by us. the box tells us only how many
 * letters there are, and we draw that many dots, so typing adds one and deleting removes one
 */
fn pill(count: usize, failed: bool) -> Stack {
    let border = if failed { theme::danger() } else { theme::outline() };

    let shape = Rectangle::new()
        .width(PILL_WIDTH)
        .height(PILL_HEIGHT)
        .radius(Full)
        .fill(theme::surface())
        .border(2.0, border)
        .opacity(0.85);

    let field = Rectangle::new()
        .width(PILL_WIDTH)
        .height(PILL_HEIGHT)
        .align_child(Start, Center)
        .child(
            TextInput::new(INPUT)
                .size(16.0)
                .color(Color::TRANSPARENT)
                .password()
                .focused()
                .on_change(|text| Typing::write().count = text.chars().count())
                .on_submit(|password| {
                    Lock::unlock(&password);

                    // the box is emptied right away, so a wrong guess starts over
                    TextInput::set_text(INPUT, "");
                    Typing::write().count = 0;
                }),
        );

    Stack::new(children![shape, field, dots(count)])
        .width(PILL_WIDTH)
        .height(PILL_HEIGHT)
}

// a hint while empty, and one dot per letter after that
fn dots(count: usize) -> Rectangle {
    let slot = Rectangle::new()
        .width(PILL_WIDTH)
        .height(PILL_HEIGHT)
        .align_child(Center, Center);

    if count == 0 {
        return slot.child(Text::new("Password").size(15.0).color(theme::muted()));
    }

    let mut row: Vec<Box<dyn Widget>> = Vec::new();

    for _ in 0..count.min(MAX_DOTS) {
        row.push(Box::new(
            Rectangle::new()
                .width(DOT)
                .height(DOT)
                .radius(Full)
                .fill(theme::text()),
        ));
    }

    slot.child(Row::new(row).gap(DOT_GAP).align(Center))
}
