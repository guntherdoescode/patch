use adw::prelude::*;
use gtk::prelude::*;
use std::cell::Cell;
use std::rc::Rc;

pub fn build(application: &adw::Application) {
    adw::StyleManager::default().set_color_scheme(adw::ColorScheme::Default);

    let window = adw::ApplicationWindow::builder()
        .application(application)
        .title("Patch")
        .default_width(1180)
        .default_height(780)
        .build();

    let root = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    let project_name = gtk::Label::new(Some("No project open"));
    project_name.add_css_class("heading");

    let sidebar = build_sidebar(&window, &project_name);
    sidebar.set_size_request(250, -1);
    root.append(&sidebar);
    root.append(&build_workspace(project_name));
    window.set_content(Some(&root));
    window.present();
}

fn build_sidebar(
    window: &adw::ApplicationWindow,
    project_name: &gtk::Label,
) -> gtk::Box {
    let sidebar = gtk::Box::new(gtk::Orientation::Vertical, 12);
    sidebar.set_margin_top(16);
    sidebar.set_margin_bottom(16);
    sidebar.set_margin_start(14);
    sidebar.set_margin_end(14);
    sidebar.add_css_class("sidebar");

    let brand = gtk::Label::new(Some("PATCH"));
    brand.add_css_class("title-2");
    brand.set_halign(gtk::Align::Start);

    let new_chat = gtk::Button::with_label("＋  New chat");
    new_chat.add_css_class("suggested-action");

    let project_label = gtk::Label::new(Some("WORKSPACE"));
    project_label.add_css_class("dim-label");
    project_label.add_css_class("caption-heading");
    project_label.set_halign(gtk::Align::Start);

    let project = gtk::Button::with_label("Open a project…");
    project.set_halign(gtk::Align::Fill);

    let history_label = gtk::Label::new(Some("CHATS"));
    history_label.add_css_class("dim-label");
    history_label.add_css_class("caption-heading");
    history_label.set_halign(gtk::Align::Start);

    let history = gtk::ListBox::new();
    history.add_css_class("boxed-list");
    history.set_selection_mode(gtk::SelectionMode::Single);
    let empty = gtk::Label::new(Some("Chats for this folder will appear here."));
    empty.add_css_class("dim-label");
    empty.set_wrap(true);
    empty.set_margin_top(12);
    empty.set_margin_bottom(12);
    empty.set_margin_start(8);
    empty.set_margin_end(8);
    history.append(&empty);

    let active_workspace = Rc::new(Cell::new(None::<i64>));
    let parent = window.clone();
    let name_label = project_name.clone();
    let history_list = history.clone();
    let workspace_id = active_workspace.clone();

    project.connect_clicked(move |_| {
        let dialog = gtk::FileChooserNative::new(
            Some("Open workspace"),
            Some(&parent),
            gtk::FileChooserAction::SelectFolder,
            Some("Open"),
            Some("Cancel"),
        );
        let name_label = name_label.clone();
        let history_list = history_list.clone();
        let workspace_id = workspace_id.clone();

        dialog.connect_response(move |dialog, response| {
            if response != gtk::ResponseType::Accept {
                return;
            }
            let Some(path) = dialog.file().and_then(|file| file.path()) else {
                return;
            };
            match crate::storage::register_workspace(&path) {
                Ok(id) => {
                    workspace_id.set(Some(id));
                    name_label.set_text(path.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("Workspace"));
                    while let Some(child) = history_list.first_child() {
                        history_list.remove(&child);
                    }
                    match crate::storage::list_chat_titles(id) {
                        Ok(chats) if !chats.is_empty() => {
                            for (_, title) in chats {
                                let label = gtk::Label::new(Some(&title));
                                label.set_halign(gtk::Align::Start);
                                label.set_margin_top(8);
                                label.set_margin_bottom(8);
                                label.set_margin_start(8);
                                label.set_margin_end(8);
                                history_list.append(&label);
                            }
                        }
                        Ok(_) => {
                            let label = gtk::Label::new(Some("No chats in this workspace yet."));
                            label.add_css_class("dim-label");
                            label.set_margin_top(10);
                            label.set_margin_bottom(10);
                            history_list.append(&label);
                        }
                        Err(error) => eprintln!("Could not load chat history: {error}"),
                    }
                }
                Err(error) => eprintln!("Could not register workspace: {error}"),
            }
        });
        dialog.show();
    });

    let history_for_new = history.clone();
    new_chat.connect_clicked(move |_| {
        let Some(id) = active_workspace.get() else {
            return;
        };
        match crate::storage::create_chat(id) {
            Ok(_) => {
                let label = gtk::Label::new(Some("New chat"));
                label.set_halign(gtk::Align::Start);
                label.set_margin_top(8);
                label.set_margin_bottom(8);
                label.set_margin_start(8);
                label.set_margin_end(8);
                history_for_new.append(&label);
            }
            Err(error) => eprintln!("Could not create chat: {error}"),
        }
    });

    sidebar.append(&brand);
    sidebar.append(&new_chat);
    sidebar.append(&project_label);
    sidebar.append(&project);
    sidebar.append(&history_label);
    sidebar.append(&history);
    sidebar
}

fn build_workspace(project_name: gtk::Label) -> gtk::Box {
    let workspace = gtk::Box::new(gtk::Orientation::Vertical, 0);
    workspace.set_hexpand(true);
    workspace.set_vexpand(true);

    let header = gtk::HeaderBar::new();
    header.set_title_widget(Some(&project_name));

    let mode = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    mode.add_css_class("linked");
    let plan_button = gtk::ToggleButton::with_label("Plan");
    let build_button = gtk::ToggleButton::with_label("Build");
    build_button.set_group(Some(&plan_button));
    plan_button.set_active(true);
    mode.append(&plan_button);
    mode.append(&build_button);
    header.pack_end(&mode);

    let model = gtk::ComboBoxText::new();
    model.append_text("Discovering local models…");
    model.set_active(Some(0));
    header.pack_start(&model);

    let usage = usage_indicator();
    header.pack_end(&usage);

    // Network discovery runs off the GTK main thread. The UI is updated only
    // after the result is sent back through the main context.
    let model_for_update = model.clone();
    std::thread::spawn(move || {
        let discovered = crate::providers::ollama::OllamaClient::default().list_models();
        gtk::glib::MainContext::default().invoke(move || {
            model_for_update.remove_all();
            match discovered {
                Ok(models) if !models.is_empty() => {
                    for item in models {
                        model_for_update.append_text(&item.name);
                    }
                    model_for_update.set_active(Some(0));
                }
                Ok(_) => {
                    model_for_update.append_text("No local Ollama models");
                    model_for_update.set_active(Some(0));
                }
                Err(_) => {
                    model_for_update.append_text("Ollama unavailable");
                    model_for_update.set_active(Some(0));
                }
            }
        });
    });

    let conversation = gtk::Box::new(gtk::Orientation::Vertical, 14);
    conversation.set_valign(gtk::Align::Center);
    conversation.set_halign(gtk::Align::Center);
    conversation.set_hexpand(true);
    conversation.set_vexpand(true);

    let welcome = gtk::Label::new(Some("A better way to work with your code."));
    welcome.add_css_class("title-1");
    let hint = gtk::Label::new(Some(
        "Open a project to begin. Plan mode is read-only; Build mode requires explicit approval.",
    ));
    hint.add_css_class("dim-label");
    hint.set_wrap(true);
    hint.set_justify(gtk::Justification::Center);
    hint.set_max_width_chars(68);
    conversation.append(&welcome);
    conversation.append(&hint);

    let composer_outer = gtk::Box::new(gtk::Orientation::Vertical, 8);
    composer_outer.set_margin_start(28);
    composer_outer.set_margin_end(28);
    composer_outer.set_margin_bottom(22);

    let composer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    composer.add_css_class("toolbar");
    let prompt = gtk::Entry::new();
    prompt.set_placeholder_text(Some("Ask Patch about your code…"));
    prompt.set_hexpand(true);
    prompt.set_sensitive(false);

    let send = gtk::Button::with_label("Send");
    send.add_css_class("suggested-action");
    send.set_sensitive(false);
    composer.append(&prompt);
    composer.append(&send);
    composer_outer.append(&composer);

    workspace.append(&header);
    workspace.append(&conversation);
    workspace.append(&composer_outer);
    workspace
}

fn usage_indicator() -> gtk::DrawingArea {
    let gauge = gtk::DrawingArea::new();
    gauge.set_content_width(38);
    gauge.set_content_height(38);
    gauge.set_tooltip_text(Some(
        "Context: unavailable\nTokens used: 0\nInput: 0 · Output: 0\nCost: unavailable until provider pricing is configured",
    ));
    gauge.set_draw_func(|_, context, width, height| {
        use std::f64::consts::PI;
        let cx = width as f64 / 2.0;
        let cy = height as f64 / 2.0;
        let radius = (width.min(height) as f64 / 2.0) - 4.0;

        context.set_line_width(3.5);
        context.set_source_rgba(0.5, 0.5, 0.5, 0.28);
        context.arc(cx, cy, radius, 0.0, 2.0 * PI);
        let _ = context.stroke();

        // No usage percentage is shown until a provider supplies real context
        // metadata. A neutral ring is preferable to a fabricated figure.
        context.set_source_rgba(0.85, 0.35, 0.12, 0.9);
        context.set_line_width(1.5);
        context.arc(cx, cy, radius, -PI / 2.0, -PI / 2.0 + 0.02);
        let _ = context.stroke();

        context.select_font_face("Sans", gtk::cairo::FontSlant::Normal, gtk::cairo::FontWeight::Bold);
        context.set_font_size(10.0);
        context.set_source_rgba(0.85, 0.85, 0.85, 0.95);
        let text = "—";
        if let Ok(extents) = context.text_extents(text) {
            context.move_to(cx - extents.width() / 2.0, cy + extents.height() / 2.0);
            let _ = context.show_text(text);
        }
    });
    gauge
}
