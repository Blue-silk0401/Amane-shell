// the bar's parts, each in src/bar/<name>.rs
mod quick;
mod shape;
mod stats;
mod system;
mod workspaces;

use amane::{
    Full, Layer, LayerWindow, Monitor, Parent, Rectangle, Stack, Vertical, Zone, children,
};

use shape::Side;

// the bar window and every group on it
pub const HEIGHT: f32 = 40.0;

/*
 * no background of its own: three groups, each on a shape of its own, with the
 * wallpaper showing through the gaps between them
 */
pub fn view(monitor: &Monitor) -> LayerWindow {
    let width = monitor.width as f32;

    let (left, left_width) = workspaces::view(monitor);
    let (right, right_width) = system::view();

    let groups = Stack::new(children![
        shape::group(Side::Left, left, left_width, width, crate::power::reach()),
        shape::group(Side::Right, right, right_width, width, crate::panel::reach()),
    ])
    .width(Parent)
    .height(Parent);

    LayerWindow::new()
        .width(Full)
        .height(HEIGHT)
        .anchor_vertical(Vertical::Top)
        .layer(Layer::Top)
        // keeps other windows from sliding under the bar
        .space(Zone::Reserve)
        .child(Rectangle::new().width(Parent).height(Parent).child(groups))
}
