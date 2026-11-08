use adw::prelude::*;
use gtk::prelude::*;

pub fn build(application: &adw::Application) {
    // Follow the desktop light/dark preference.
    adw::StyleManager::default().set_color_scheme(adw::ColorScheme::Default);

    let window = adw::ApplicationWindow::builder()
        .application(application)
        .title("Patch")
        .default_width(1180)
        .default_height(780)
        .build();

    let root = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    let sidebar = build_sidebar();
    sidebar.set_size_request(250, -1);

    root.append(&sidebar);
    root.append(&build_workspace());
    window.set_content(Some(&root));
    window.present();
}

fn build_sidebar() -> gtk::Box {
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

    sidebar.append(&brand);
    sidebar.append(&new_chat);
    sidebar.append(&project_label);
    sidebar.append(&project);
    sidebar.append(&history_label);
    sidebar.append(&history);
    sidebar
}

fn build_workspace() -> gtk::Box {
    let workspace = gtk::Box::new(gtk::Orientation::Vertical, 0);
    workspace.set_hexpand(true);
    workspace.set_vexpand(true);

    let header = gtk::HeaderBar::new();
    let project_name = gtk::Label::new(Some("No project open"));
    project_name.add_css_class("heading");
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
    model.append_text("Select a model");
    model.set_active(Some(0));
    header.pack_start(&model);

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
