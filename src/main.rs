mod app;
mod permissions;
mod providers;
mod storage;
mod usage;

use gpui_kit::gpui::{App, AppContext, WindowOptions};

fn main() {
    if let Err(error) = storage::open_database() {
        eprintln!("Patch could not initialise its local database: {error}");
    }

    gpui_kit::platform::application().run(|cx: &mut App| {
        cx.open_window(WindowOptions::default(), |window, cx| {
            cx.new(|cx| app::PatchApp::new(window, cx))
        })
        .expect("failed to open Patch window");
        cx.activate(true);
    });
}
