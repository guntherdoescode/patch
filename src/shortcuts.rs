//! Centralised keyboard shortcuts for the Patch interface.

use adw::prelude::*;

/// Tab toggles Plan/Build; Enter sends; Shift+Enter inserts a newline.
pub fn install_prompt_shortcuts<F>(
    prompt: &gtk::TextView,
    send_button: &gtk::Button,
    toggle_mode: F,
)
where
    F: Fn() + 'static,
{
    let controller = gtk::EventControllerKey::new();
    let send_button = send_button.clone();

    controller.connect_key_pressed(move |_, key, _, modifiers| {
        if key == gtk::gdk::Key::Tab {
            toggle_mode();
            return gtk::glib::Propagation::Stop;
        }

        if key == gtk::gdk::Key::Return || key == gtk::gdk::Key::KP_Enter {
            if modifiers.contains(gtk::gdk::ModifierType::SHIFT_MASK) {
                return gtk::glib::Propagation::Proceed;
            }
            send_button.emit_clicked();
            return gtk::glib::Propagation::Stop;
        }

        gtk::glib::Propagation::Proceed
    });

    prompt.add_controller(controller);
}
