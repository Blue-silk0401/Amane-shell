use std::cell::Cell;
use std::time::{Duration, Instant};

use amane::{
    Center, Column, Image, Media, Pointer, Rectangle, Row, Service, Stack, Start, Text, Weight,
    children,
};

use crate::theme;

const ICON_FONT: &str = "Symbols Nerd Font";
const PLAY: &str = "\u{f040a}";
const PAUSE: &str = "\u{f03e4}";
const NEXT: &str = "\u{f04ad}";
const PREVIOUS: &str = "\u{f04ae}";
const NOTE: &str = "\u{f075a}";

const PAD: f32 = 12.0;
const ART: f32 = 48.0;
const BAR_HIT: f32 = 14.0;
const BAR: f32 = 5.0;
const BUTTON: f32 = 36.0;
const GAP: f32 = 8.0;

pub const HEIGHT: f32 = PAD * 2.0 + ART + BAR_HIT + BUTTON + GAP * 2.0;

thread_local! {
    static LAST_SEEK: Cell<Option<Instant>> = const { Cell::new(None) };
}

// the card only exists while some player is open
pub fn visible() -> bool {
    Media::read().active().is_some()
}

pub fn view(width: f32) -> Rectangle {
    let media = Media::read();
    let inner = width - PAD * 2.0;

    let (title, artist) = if media.title().is_empty() {
        ("Nothing playing".to_string(), String::new())
    } else {
        (media.title().to_string(), media.artist().to_string())
    };

    let text = Column::new(children![
        Text::new(title).size(14.0).weight(Weight::Bold).color(theme::text()).elide(),
        Text::new(artist).size(12.0).color(theme::muted()).elide(),
    ])
    .gap(2.0);

    let top = Row::new(children![
        cover(media.art_url()),
        Rectangle::new().width(inner - ART - GAP).height(ART).align_child(Start, Center).child(text),
    ])
    .gap(GAP);

    let controls = Row::new(children![
        button(PREVIOUS, || Media::previous()),
        button(if media.playing() { PAUSE } else { PLAY }, || Media::play_pause()),
        button(NEXT, || Media::next()),
    ])
    .gap(GAP);

    let controls = Rectangle::new()
        .width(inner)
        .height(BUTTON)
        .align_child(Center, Center)
        .child(controls);

    Rectangle::new()
        .width(width)
        .height(HEIGHT)
        .radius(16.0)
        .fill(theme::surface())
        .padding(PAD)
        .child(
            Column::new(children![top, seek_bar(inner, media.position(), media.length()), controls])
                .gap(GAP),
        )
}

fn cover(url: &str) -> Rectangle {
    let slot = Rectangle::new().width(ART).height(ART).radius(10.0).fill(theme::bg()).clip();

    if let Some(path) = url.strip_prefix("file://") {
        if std::path::Path::new(path).exists() && Image::loaded(path) {
            let pixels = ART as u32;
            return slot.fill(Image::cover(path).thumbnail(pixels, pixels));
        }
    }

    slot.align_child(Center, Center)
        .child(Text::new(NOTE).size(24.0).font(ICON_FONT).tight().color(theme::muted()))
}

fn button(icon: &str, clicked: fn()) -> Rectangle {
    Rectangle::new()
        .width(BUTTON)
        .height(BUTTON)
        .radius(BUTTON / 2.0)
        .cursor(Pointer)
        .on_click(move |_| clicked())
        .align_child(Center, Center)
        .child(Text::new(icon).size(20.0).font(ICON_FONT).tight().color(theme::text()))
}

fn seek_bar(width: f32, position: Duration, length: Duration) -> Rectangle {
    let total = length.as_secs_f32();
    let fraction = if total > 0.0 { (position.as_secs_f32() / total).clamp(0.0, 1.0) } else { 0.0 };

    let track = Rectangle::new().width(width).height(BAR).radius(BAR / 2.0).fill(theme::bg());
    let filled = Rectangle::new()
        .width((width * fraction).max(BAR))
        .height(BAR)
        .radius(BAR / 2.0)
        .fill(theme::accent());

    Rectangle::new()
        .width(width)
        .height(BAR_HIT)
        .cursor(Pointer)
        .on_drag(move |point| seek((point.x / width).clamp(0.0, 1.0) * total))
        .align_child(Start, Center)
        .child(Stack::new(children![track, filled]))
}

// Amane can't seek, so this asks playerctl; capped at about 7 calls a second while dragging
fn seek(seconds: f32) {
    let recent = LAST_SEEK
        .with(|last| last.get().is_some_and(|t| t.elapsed() < Duration::from_millis(140)));

    if recent {
        return;
    }

    LAST_SEEK.with(|last| last.set(Some(Instant::now())));
    amane::spawn(&format!("playerctl position {seconds:.1}"));
}
