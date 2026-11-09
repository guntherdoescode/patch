mod app;
mod permissions;
mod providers;
mod storage;
mod usage;

use gpui::{App, WindowOptions};

fn main() {
    if let Err(error) = storage::open_database() {
        eprintln!("Patch could not initialise its local database: {error}");
    }

    gpui_platform::application().run(|cx: &mut App| {
        cx.open_window(WindowOptions::default(), |_window, cx| {
            cx.new(|_cx| app::PatchApp::default())
        })
        .expect("failed to open Patch window");
        cx.activate(true);
    });
}
