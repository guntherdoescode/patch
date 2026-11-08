mod app;
mod permissions;

use adw::prelude::*;

fn main() -> gtk::glib::ExitCode {
    let application = adw::Application::builder()
        .application_id("dev.patch.Patch")
        .build();

    application.connect_activate(app::build);
    application.run()
}
