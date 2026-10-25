use std::path::PathBuf;
use std::time::SystemTime;

use amane::{
    Center, Color, Column, Full, Image, Notifications, Pointer, Rectangle, Row, ScrollArea, Start,
    Service, Text, Urgency, Widget, children,
};

use super::{BELL, NERD_FONT};
use crate::notifications::{self, Quiet};
use crate::theme;

const LIST: &str = "alerts-list";

const GAP: f32 = 10.0;
const HEADER_HEIGHT: f32 = 40.0;
const BUTTON_SIZE: f32 = 36.0;
const CLEAR_WIDTH: f32 = 84.0;
const CLEAR_HEIGHT: f32 = 32.0;

const CARD_HEIGHT: f32 = 76.0;
const CARD_PADDING: f32 = 12.0;
const CARD_GAP: f32 = 6.0;
const PICTURE: f32 = 40.0;
const CLOSE_SIZE: f32 = 28.0;

const BACK: &str = "\u{f004d}";
const CLOSE: &str = "\u{f0156}";

// a copy of one notification, so the click handlers don't borrow from the Service
struct Item {
    id: u32,
    app: String,
    summary: String,
    body: String,
    ago: String,
    critical: bool,
    avatar: Option<PathBuf>,
    icon: Option<PathBuf>,
}

pub fn view(width: f32, height: f32) -> Column {
    // the list keeps the oldest first, so reversing puts the newest on top
    let items = {
        let stored = Notifications::read();

        let items: Vec<Item> = stored
            .list()
            .iter()
            .rev()
            .map(|notification| {
                let (avatar, icon) = notifications::pictures(notification);

                Item {
                    id: notification.id(),
                    app: String::from(notification.app_name()),
                    summary: notifications::plain(notification.summary()),
                    body: notifications::plain(notification.body()),
                    ago: ago(notification.received()),
                    critical: notification.urgency() == Urgency::Critical,
                    avatar,
                    icon,
                }
            })
            .collect();

        items
    };

    let body_height = height - HEADER_HEIGHT - GAP;

    let body: Box<dyn Widget> = if items.is_empty() {
        Box::new(
            Rectangle::new()
                .width(width)
                .height(body_height)
                .align_child(Center, Center)
                .child(Text::new("No notifications").size(14.0).color(theme::muted())),
        )
    } else {
        let mut cards: Vec<Box<dyn Widget>> = Vec::new();

        for item in &items {
            cards.push(Box::new(card(item, width)));
        }

        Box::new(
            Rectangle::new()
                .width(width)
                .height(body_height)
                .child(ScrollArea::new(LIST, Column::new(cards).gap(CARD_GAP))),
        )
    };

    let rows: Vec<Box<dyn Widget>> = vec![Box::new(header(items.len(), width)), body];

    Column::new(rows).gap(GAP)
}

// back arrow, title, the bell that switches popups on and off, and Clear all
fn header(count: usize, width: f32) -> Row {
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

    let title_width = width - BUTTON_SIZE * 2.0 - CLEAR_WIDTH - GAP * 3.0;

    let title = Rectangle::new()
        .width(title_width)
        .height(HEADER_HEIGHT)
        .align_child(Start, Center)
        .child(
            Text::new(if count == 0 {
                String::from("Notifications")
            } else {
                format!("Notifications ({count})")
            })
            .size(16.0)
            .color(theme::text())
            .elide(),
        );

    // lit while popups are on
    let (fill, color) = if Quiet::on() {
        (theme::surface(), theme::muted())
    } else {
        (theme::accent(), theme::on_accent())
    };

    let bell = Rectangle::new()
        .width(BUTTON_SIZE)
        .height(BUTTON_SIZE)
        .radius(12.0)
        .fill(fill)
        .cursor(Pointer)
        .on_click(|_| Quiet::toggle())
        .align_child(Center, Center)
        .child(Text::new(BELL).size(16.0).font(NERD_FONT).tight().color(color));

    let clear = Rectangle::new()
        .width(CLEAR_WIDTH)
        .height(CLEAR_HEIGHT)
        .radius(CLEAR_HEIGHT / 2.0)
        .fill(theme::surface())
        .cursor(Pointer)
        .on_click(|_| Notifications::clear())
        .align_child(Center, Center)
        .child(Text::new("Clear all").size(13.0).color(theme::text()));

    Row::new(children![back, title, bell, clear])
        .gap(GAP)
        .align(Center)
}

// one notification: click it to run its action, or the cross to dismiss it
fn card(item: &Item, width: f32) -> Rectangle {
    let inner = width - CARD_PADDING * 2.0;

    let id = item.id;

    let text_width = inner - PICTURE - CLOSE_SIZE - GAP * 2.0;

    let text = Column::new(children![
        line(&format!("{}  \u{b7}  {}", item.app, item.ago), 11.0, theme::muted(), 14.0, text_width),
        line(&item.summary, 14.0, theme::text(), 18.0, text_width),
        line(&item.body, 12.0, theme::muted(), 16.0, text_width),
    ])
    .gap(1.0);

    let close = Rectangle::new()
        .width(CLOSE_SIZE)
        .height(CLOSE_SIZE)
        .radius(Full)
        .cursor(Pointer)
        .on_click(move |_| Notifications::dismiss(id))
        .align_child(Center, Center)
        .child(
            Text::new(CLOSE)
                .size(13.0)
                .font(NERD_FONT)
                .tight()
                .color(theme::muted()),
        );

    let card = Rectangle::new()
        .width(width)
        .height(CARD_HEIGHT)
        .radius(14.0)
        .fill(theme::surface())
        .padding(CARD_PADDING)
        .cursor(Pointer)
        .on_click(move |_| Notifications::click(id))
        .align_child(Start, Center)
        .child(Row::new(children![picture(item), text, close]).gap(GAP).align(Center));

    if item.critical {
        card.border(2.0, theme::danger())
    } else {
        card
    }
}

// the sender's picture as a circle, or the app's icon, or a bell
fn picture(item: &Item) -> Rectangle {
    let slot = Rectangle::new().width(PICTURE).height(PICTURE);

    if let Some(avatar) = &item.avatar {
        let pixels = (PICTURE * 2.0) as u32;

        return slot
            .radius(Full)
            .fill(Image::cover(avatar.clone()).thumbnail(pixels, pixels));
    }

    if let Some(icon) = &item.icon {
        let inner = PICTURE - 12.0;
        let pixels = (inner * 2.0) as u32;

        return slot
            .radius(12.0)
            .fill(theme::bg())
            .align_child(Center, Center)
            .child(
                Rectangle::new()
                    .width(inner)
                    .height(inner)
                    .fill(Image::contain(icon.clone()).thumbnail(pixels, pixels)),
            );
    }

    slot.radius(Full)
        .fill(theme::bg())
        .align_child(Center, Center)
        .child(Text::new(BELL).size(18.0).font(NERD_FONT).tight().color(theme::accent()))
}

// one line of text in a box, cut off with "..." instead of spilling out
fn line(text: &str, size: f32, color: Color, height: f32, width: f32) -> Rectangle {
    Rectangle::new()
        .width(width)
        .height(height)
        .align_child(Start, Center)
        .child(Text::new(text).size(size).color(color).elide())
}

// "now", "5m ago", "3h ago", "2d ago"
fn ago(received: SystemTime) -> String {
    let seconds = SystemTime::now()
        .duration_since(received)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);

    match seconds {
        0..=59 => String::from("now"),
        60..=3599 => format!("{}m ago", seconds / 60),
        3600..=86399 => format!("{}h ago", seconds / 3600),
        _ => format!("{}d ago", seconds / 86400),
    }
}
