use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use amane::{
    Animation, Apps, Center, Color, Column, Easing, Full, Horizontal, Image, Layer, LayerWindow,
    Notification, Notifications, Padding, Parent, Pointer, Rectangle, Row, Service, Stack, Start,
    Text, Urgency, Vertical, Widget, children,
};

use crate::theme;

const CARD_WIDTH: f32 = 380.0;
const CARD_HEIGHT: f32 = 92.0;
const GAP: f32 = 10.0;
const RADIUS: f32 = 16.0;
const PADDING: f32 = 14.0;

// the picture on the left, and the space between it and the text
const ICON_SLOT: f32 = 52.0;
const ICON_GAP: f32 = 12.0;

// the small app icon on the avatar's corner
const BADGE: f32 = 22.0;

// buttons sit under the text when the sender asked for them
const BUTTON_HEIGHT: f32 = 28.0;
const BUTTON_GAP: f32 = 6.0;
const BUTTONS_SPACE: f32 = 8.0;
const MAX_BUTTONS: usize = 3;

// the text fits inside the padding, the 2px border and the picture
const TEXT_WIDTH: f32 = CARD_WIDTH - PADDING * 2.0 - 4.0 - ICON_SLOT - ICON_GAP;

// space inside the window: above the first card, and to the right of every card
const TOP: f32 = 12.0;
const EDGE: f32 = 16.0;

const WINDOW_WIDTH: f32 = CARD_WIDTH + EDGE;

// when more arrive, the oldest ones slide away early
const MAX_SHOWN: usize = 5;

// how long a popup stays (critical ones stay until you click them)
const TIMEOUT: Duration = Duration::from_secs(6);

const SLIDE_MS: u64 = 350;
const SHIFT_MS: u64 = 250;

const NERD_FONT: &str = "JetBrainsMono Nerd Font";
const BELL: &str = "\u{f0f3}";

thread_local! {
    /*
     * every popup's slide and timer live here, not in a Service: a view
     * can't write to a Service, and these change on every frame
     */
    static POPUPS: RefCell<Popups> = RefCell::new(Popups::new());
}

struct Popup {
    // our own copy, so the card can still be drawn while it slides out
    notification: Notification,

    // the sender's picture, like a profile photo, when it sent one
    avatar: Option<PathBuf>,

    // the app's logo, when we can find it
    app_icon: Option<PathBuf>,

    born: Instant,

    // how far out to the right: 1 is fully off screen, 0 is resting
    slide: Animation,

    // the card's top edge, animated so older cards glide down when a new one arrives
    y: Animation,

    leaving: bool,
}

impl Popup {
    // starts off screen and slides in
    fn new(notification: Notification) -> Self {
        let mut slide: Animation = Animation::new(1.0)
            .duration(Duration::from_millis(SLIDE_MS))
            .easing(Easing::Out);

        slide.to(0.0);

        let (avatar, app_icon) = pictures(&notification);

        Self {
            notification,
            avatar,
            app_icon,
            born: Instant::now(),
            slide,
            y: Animation::new(TOP)
                .duration(Duration::from_millis(SHIFT_MS))
                .easing(Easing::Out),
            leaving: false,
        }
    }

    // the sender replaced its own notification: new text, new pictures, a fresh timer
    fn refresh(&mut self, notification: Notification) {
        let (avatar, app_icon) = pictures(&notification);

        self.notification = notification;
        self.avatar = avatar;
        self.app_icon = app_icon;
        self.born = Instant::now();
    }

    // slides back out the way it came
    fn leave(&mut self) {
        self.slide.to(1.0);

        self.leaving = true;
    }

    // out of sight, so it can be forgotten
    fn gone(&self) -> bool {
        self.leaving && self.slide.value() >= 0.999
    }

    // taller when there are buttons to fit
    fn height(&self) -> f32 {
        if self.notification.actions().is_empty() {
            CARD_HEIGHT
        } else {
            CARD_HEIGHT + BUTTONS_SPACE + BUTTON_HEIGHT
        }
    }

    fn card(&self) -> Rectangle {
        let notification = &self.notification;

        let id = notification.id();

        let border = if notification.urgency() == Urgency::Critical {
            theme::danger()
        } else {
            theme::outline()
        };

        let mut lines: Vec<Box<dyn Widget>> = vec![
            Box::new(line(notification.app_name().to_string(), 12.0, theme::muted(), 16.0)),
            Box::new(line(plain(notification.summary()), 15.0, theme::text(), 20.0)),
        ];

        let body = plain(notification.body());

        if !body.is_empty() {
            lines.push(Box::new(line(body, 13.0, theme::muted(), 18.0)));
        }

        let text = Column::new(lines).gap(2.0);

        let top = Row::new(children![self.picture(), text])
            .gap(ICON_GAP)
            .align(Center);

        let mut content: Vec<Box<dyn Widget>> = vec![Box::new(top)];

        if !notification.actions().is_empty() {
            content.push(Box::new(buttons(notification)));
        }

        Rectangle::new()
            .width(CARD_WIDTH)
            .height(self.height())
            .radius(RADIUS)
            .fill(theme::bg())
            .border(2.0, border)
            .padding(PADDING)
            .align_child(Start, Center)
            .translate(self.slide.value() * WINDOW_WIDTH, self.y.value())
            .cursor(Pointer)
            // runs the notification's default action, or dismisses it when it has none
            .on_click(move |_| Notifications::click(id))
            .child(Column::new(content).gap(BUTTONS_SPACE))
    }

    // avatar with a badge, or the app icon, or a bell
    fn picture(&self) -> Stack {
        let slot = Rectangle::new().width(ICON_SLOT).height(ICON_SLOT);

        let mut layers: Vec<Box<dyn Widget>> = Vec::new();

        if let Some(avatar) = &self.avatar {
            // `cover` fills the whole circle, cropping the picture's edges if needed
            let pixels = (ICON_SLOT * 2.0) as u32;

            layers.push(Box::new(
                slot.radius(Full)
                    .fill(Image::cover(avatar.clone()).thumbnail(pixels, pixels)),
            ));

            if let Some(icon) = &self.app_icon {
                layers.push(Box::new(badge(icon)));
            }
        } else if let Some(icon) = &self.app_icon {
            // `contain` shows the whole icon, never cropped
            let inner = ICON_SLOT - 16.0;
            let pixels = (inner * 2.0) as u32;

            layers.push(Box::new(
                slot.radius(14.0)
                    .fill(theme::surface())
                    .align_child(Center, Center)
                    .child(
                        Rectangle::new()
                            .width(inner)
                            .height(inner)
                            .fill(Image::contain(icon.clone()).thumbnail(pixels, pixels)),
                    ),
            ));
        } else {
            layers.push(Box::new(
                slot.radius(Full)
                    .fill(theme::surface())
                    .align_child(Center, Center)
                    .child(
                        Text::new(BELL)
                            .size(22.0)
                            .font(NERD_FONT)
                            .tight()
                            .color(theme::accent()),
                    ),
            ));
        }

        Stack::new(layers).width(ICON_SLOT).height(ICON_SLOT)
    }
}

// the small circle on the avatar's bottom right corner
fn badge(icon: &Path) -> Rectangle {
    let inner = BADGE - 6.0;
    let pixels = (inner * 2.0) as u32;

    Rectangle::new()
        .width(BADGE)
        .height(BADGE)
        .radius(Full)
        .fill(theme::bg())
        .align_child(Center, Center)
        .translate(ICON_SLOT - BADGE, ICON_SLOT - BADGE)
        .child(
            Rectangle::new()
                .width(inner)
                .height(inner)
                .fill(Image::contain(icon.to_path_buf()).thumbnail(pixels, pixels)),
        )
}

// one button for each action the sender offered, lined up with the text
fn buttons(notification: &Notification) -> Rectangle {
    let id = notification.id();

    let mut row: Vec<Box<dyn Widget>> = Vec::new();

    for action in notification.actions().iter().take(MAX_BUTTONS) {
        let key = String::from(action.key());
        let label = action.label();

        // a guess at the text's width, like the bar's pills
        let width = label.chars().count() as f32 * 7.5 + 24.0;

        row.push(Box::new(
            Rectangle::new()
                .width(width)
                .height(BUTTON_HEIGHT)
                .radius(8.0)
                .fill(theme::surface())
                .align_child(Center, Center)
                .cursor(Pointer)
                .on_click(move |_| Notifications::invoke(id, &key))
                .child(Text::new(label).size(12.0).color(theme::text())),
        ));
    }

    Rectangle::new()
        .width(Parent)
        .height(BUTTON_HEIGHT)
        .padding(Padding {
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            left: ICON_SLOT + ICON_GAP,
        })
        .child(Row::new(row).gap(BUTTON_GAP))
}

struct Popups {
    // oldest first
    list: Vec<Popup>,

    // what we have already seen, so a popup that timed out doesn't pop up again
    known: Vec<(u32, SystemTime)>,
}

impl Popups {
    fn new() -> Self {
        Self {
            list: Vec::new(),
            known: Vec::new(),
        }
    }

    fn update(&mut self, incoming: &[Notification]) {
        let quiet = Quiet::on();

        // new arrivals, or a sender replacing its own earlier notification
        for notification in incoming {
            if self.known.contains(&(notification.id(), notification.received())) {
                continue;
            }

            let existing = self
                .list
                .iter_mut()
                .find(|popup| popup.notification.id() == notification.id());

            match existing {
                Some(popup) => {
                    popup.refresh(notification.clone());

                    if popup.leaving {
                        popup.leaving = false;
                        popup.slide.to(0.0);
                    }
                }
                None => {
                    if !quiet {
                        self.list.push(Popup::new(notification.clone()));
                    }
                }
            }
        }

        self.known = incoming
            .iter()
            .map(|notification| (notification.id(), notification.received()))
            .collect();

        // timed out, or dismissed somewhere else: slide out
        for popup in &mut self.list {
            if popup.leaving {
                continue;
            }

            let listed = incoming
                .iter()
                .any(|notification| notification.id() == popup.notification.id());

            let critical = popup.notification.urgency() == Urgency::Critical;

            let expired = !critical && popup.born.elapsed() >= TIMEOUT;

            if !listed || expired {
                popup.leave();
            }
        }

        // too many at once: the oldest leave early
        let mut active = self.list.iter().filter(|popup| !popup.leaving).count();

        for popup in &mut self.list {
            if active <= MAX_SHOWN {
                break;
            }

            if !popup.leaving {
                popup.leave();

                active -= 1;
            }
        }

        self.list.retain(|popup| !popup.gone());

        // the newest is on top, and each older card starts where the one above it ends
        let mut top = TOP;

        for popup in self.list.iter_mut().rev() {
            popup.y.to(top);

            top += popup.height() + GAP;
        }

        // the timeout is checked here, so the view has to keep running while a card waits
        if self.list.iter().any(|popup| !popup.leaving) {
            amane::request_frame();
        }
    }

    fn window(&self) -> LayerWindow {
        let mut height = TOP;

        let mut cards: Vec<Box<dyn Widget>> = Vec::new();

        for popup in &self.list {
            height += popup.height() + GAP;

            cards.push(Box::new(popup.card()));
        }

        LayerWindow::new()
            .width(WINDOW_WIDTH)
            .height(height)
            .anchor_vertical(Vertical::Top)
            .anchor_horizontal(Horizontal::Right)
            .layer(Layer::Overlay)
            .namespace("notifications")
            // hidden when empty, so it never blocks clicks on windows underneath
            .visible(!self.list.is_empty())
            .child(Rectangle::new().width(Parent).height(Parent).child(Stack::new(cards)))
    }
}

pub fn view() -> LayerWindow {
    // reading Notifications is also what starts amane as the notification daemon
    let notifications = Notifications::read();

    POPUPS.with_borrow_mut(|popups| {
        popups.update(notifications.list());

        popups.window()
    })
}

// one line of a card, cut off with "..." instead of spilling out
fn line(text: String, size: f32, color: Color, height: f32) -> Rectangle {
    Rectangle::new()
        .width(TEXT_WIDTH)
        .height(height)
        .align_child(Start, Center)
        .child(Text::new(text).size(size).color(color).elide())
}

// senders may put simple markup like <b> in the text, and newlines
pub fn plain(text: &str) -> String {
    let mut out = String::new();
    let mut inside_tag = false;

    for character in text.chars() {
        match character {
            '<' => inside_tag = true,
            '>' => inside_tag = false,
            '\n' => out.push(' '),
            _ if !inside_tag => out.push(character),
            _ => {}
        }
    }

    out
}

// (the sender's picture, the app's icon), looked up once when the popup is made
pub fn pictures(notification: &Notification) -> (Option<PathBuf>, Option<PathBuf>) {
    let avatar = file_path(notification.image()).filter(|path| drawable(path));

    (avatar, find_app_icon(notification))
}

/*
 * the icon is a file path, or a name from the icon theme like "discord";
 * a name is looked up through the installed apps, and with no usable icon
 * the app that sent it may still be installed under the same name
 */
fn find_app_icon(notification: &Notification) -> Option<PathBuf> {
    let icon = notification.icon();

    if let Some(path) = file_path(icon) {
        return Some(path).filter(|path| drawable(path));
    }

    let apps = Apps::read();

    for app in apps.list() {
        let same_icon = !icon.is_empty() && app.icon() == Some(icon);
        let same_app = app.name().eq_ignore_ascii_case(notification.app_name());

        if same_icon || same_app {
            return app
                .icon_path()
                .map(PathBuf::from)
                .filter(|path| drawable(path));
        }
    }

    None
}

// a path or a file:// link becomes a path, an icon name or empty text becomes nothing
fn file_path(text: &str) -> Option<PathBuf> {
    if let Some(url) = text.strip_prefix("file://") {
        return Some(PathBuf::from(decode_url(url)));
    }

    if text.starts_with('/') {
        return Some(PathBuf::from(text));
    }

    None
}

// only these formats are drawn, so anything else gets the bell instead
fn drawable(path: &Path) -> bool {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_lowercase());

    matches!(extension.as_deref(), Some("png" | "jpg" | "jpeg" | "svg"))
}

// a file link writes spaces and other bytes as %20 and the like
fn decode_url(url: &str) -> String {
    let bytes = url.as_bytes();

    let mut decoded = Vec::with_capacity(bytes.len());

    let mut index = 0;

    while index < bytes.len() {
        let escaped = bytes
            .get(index + 1..index + 3)
            .filter(|_| bytes[index] == b'%')
            .and_then(|hex| std::str::from_utf8(hex).ok())
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());

        if let Some(byte) = escaped {
            decoded.push(byte);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }

    String::from_utf8_lossy(&decoded).into_owned()
}

// "do not disturb": while on, new notifications are still kept, but no popup shows
pub struct Quiet {
    on: bool,
}

impl Service for Quiet {
    fn new() -> Self {
        Self { on: false }
    }

    fn listen() {}
}

impl Quiet {
    pub fn on() -> bool {
        Self::read().on
    }

    pub fn toggle() {
        let mut quiet = Self::write();

        quiet.on = !quiet.on;
    }
}
