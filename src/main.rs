mod app;
mod permissions;
mod providers;
mod storage;
mod usage;

use adw::prelude::*;

fn main() -> gtk::glib::ExitCode {
    if let Err(error) = storage::open_database() {
        eprintln!("Patch could not initialise its local database: {error}");
    }

    let application = adw::Application::builder()
        .application_id("dev.patch.Patch")
        .build();

    application.connect_activate(app::build);
    application.run()
}
