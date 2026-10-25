#![allow(dead_code)]

// `mod` tells Rust to load a file. Each of these is src/<name>.rs
mod wallpaper;
mod calendar;
mod hang;
mod power;
mod lock;
mod bar;
mod clock;
mod date;
mod face;
mod weather;
mod clipboard;
mod osd;
mod panel;
mod screen_mask;
mod theme;
mod launcher;
mod notifications;

use amane::App;

fn main() {
    wallpaper::warm_up();
    wallpaper::theme_now();

    // window_per_monitor calls bar::view once for every screen you have
    App::new()
    .ipc("launcher", launcher::ipc)
    .ipc("panel", panel::ipc)
    .ipc("power", power::ipc)
    .ipc("calendar", calendar::ipc)
    .ipc("clipboard", clipboard::ipc)
    .ipc("wallpaper", wallpaper::ipc)
    .ipc("lock", lock::ipc)
    .window(launcher::view)
    .window(launcher::scrim)
    .window(notifications::view)
    .window(panel::view)
    .window(panel::scrim)
    .window(power::view)
    .window(power::scrim)
    .window(clipboard::scrim)
    .window(clipboard::view)
    .window_per_monitor(calendar::widget)
    .window(panel::dim::view)
    .window(osd::view)
    .lock(lock::view)
    .window_per_monitor(wallpaper::background)
    .window_per_monitor(face::view)
    .window_per_monitor(wallpaper::view)
    .window(wallpaper::scrim)
    .window_per_monitor(screen_mask::view)
    .window_per_monitor(bar::view)
    .run();
}
