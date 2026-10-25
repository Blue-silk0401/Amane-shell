use amane::{
    Center, Color, Full, Monitor, Pointer, Rectangle, Row, Service, Text, Widget, Workspace,
    Workspaces,
};

use crate::panel::NERD_FONT;
use crate::theme;

const SLOT: f32 = 28.0;
const SLOT_GAP: f32 = 6.0;

// the Nix snowflake from a Nerd Font
const LOGO: &str = "\u{f1105}";
const LOGO_SIZE: f32 = 24.0;
const LOGO_SLOT: f32 = LOGO_SIZE + 14.0;

// between the logo and the workspaces, and from the group's edges
const GROUP_GAP: f32 = 8.0;
const SIDE: f32 = 12.0;

// which workspace the pointer is on
pub struct Hover {
    id: Option<i64>,
}

impl Service for Hover {
    fn new() -> Self {
        Self { id: None }
    }

    fn listen() {}
}

// only writes when something changes, so it doesn't redraw for nothing
fn set_hover(id: i64, inside: bool) {
    let current = Hover::read().id;

    if inside {
        if current != Some(id) {
            Hover::write().id = Some(id);
        }
    } else if current == Some(id) {
        Hover::write().id = None;
    }
}

// the logo and this monitor's workspaces, and how wide they are
pub fn view(monitor: &Monitor) -> (Row, f32) {
    let hovered = Hover::read().id;

    let workspaces = Workspaces::read();

    // every monitor's bar only shows its own workspaces
    let mut own: Vec<&Workspace> = Vec::new();

    for workspace in workspaces.list() {
        if workspace.output() == Some(monitor.name.as_str()) {
            own.push(workspace);
        }
    }

    own.sort_by_key(|workspace| workspace.index());

    let count = own.len();

    let mut items: Vec<Box<dyn Widget>> = vec![Box::new(logo_button())];

    let mut width = SIDE * 2.0 + LOGO_SLOT;

    if count > 0 {
        let mut slots: Vec<Box<dyn Widget>> = Vec::new();

        for workspace in own {
            slots.push(Box::new(slot(workspace, hovered == Some(workspace.id()))));
        }

        items.push(Box::new(Row::new(slots).gap(SLOT_GAP).align(Center)));

        width += GROUP_GAP + count as f32 * SLOT + (count - 1) as f32 * SLOT_GAP;
    }


    (Row::new(items).gap(GROUP_GAP).align(Center), width)
}

// the logo as a button: opens the power menu, lit while the pointer is on it or it is open
fn logo_button() -> Rectangle {
    let lit = crate::power::Power::is_open() || crate::power::Power::logo_hovered();

    let fill = if lit { theme::hover() } else { Color::TRANSPARENT };

    let logo = Text::new(LOGO)
        .size(LOGO_SIZE)
        .font(NERD_FONT)
        .color(theme::accent())
        .tight();

    Rectangle::new()
        .width(LOGO_SLOT)
        .height(LOGO_SIZE + 8.0)
        .radius(Full)
        .fill(fill)
        .cursor(Pointer)
        .on_hover(|inside| crate::power::set_logo_hover(inside))
        .on_click(|_| crate::power::toggle())
        .align_child(Center, Center)
        .child(logo)
}

fn slot(workspace: &Workspace, hovered: bool) -> Rectangle {
    let id = workspace.id();

    let active = workspace.active();
    let urgent = workspace.urgent();

    // only an urgent or hovered slot gets a background
    let background = if urgent {
        theme::danger()
    } else if hovered {
        theme::hover()
    } else {
        Color::TRANSPARENT
    };

    let mark = if urgent {
        Color::BLACK
    } else if active {
        theme::accent()
    } else if hovered {
        theme::text()
    } else {
        theme::muted()
    };

    let cell = Rectangle::new()
        .width(SLOT)
        .height(SLOT)
        .radius(Full)
        .fill(background)
        .cursor(Pointer)
        .on_click(move |_| Workspaces::focus(id))
        .on_hover(move |inside| set_hover(id, inside))
        .align_child(Center, Center);

    if active {
        cell.child(ringed_dot(mark))
    } else {
        cell.child(dot(mark))
    }
}

fn dot(color: Color) -> Rectangle {
    Rectangle::new()
        .width(6.0)
        .height(6.0)
        .radius(Full)
        .fill(color)
}

// a ring with a dot in the middle, for the workspace you are on
fn ringed_dot(color: Color) -> Rectangle {
    Rectangle::new()
        .width(SLOT - 8.0)
        .height(SLOT - 8.0)
        .radius(Full)
        .border(1.5, color)
        .align_child(Center, Center)
        .child(dot(color))
}

