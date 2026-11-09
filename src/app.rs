use adw::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub fn build(application: &adw::Application) {
    adw::StyleManager::default().set_color_scheme(adw::ColorScheme::Default);

    let window = adw::ApplicationWindow::builder()
        .application(application)
        .title("Patch")
        .default_width(1180)
        .default_height(780)
        .build();

    let root = gtk::Paned::new(gtk::Orientation::Horizontal);
    root.set_wide_handle(true);
    root.set_position(255);
    let content = gtk::Paned::new(gtk::Orientation::Horizontal);
    content.set_wide_handle(true);
    content.set_position(760);

    let project_name = gtk::Label::new(Some("No project open"));
    project_name.add_css_class("heading");

    let active_chat = Rc::new(Cell::new(None::<i64>));
    let chat_messages = Rc::new(RefCell::new(Vec::<crate::providers::ollama::ChatMessage>::new()));
    let transcript_buffer = Rc::new(RefCell::new(None::<gtk::TextBuffer>));
    let transcript_scroll_ref = Rc::new(RefCell::new(None::<gtk::ScrolledWindow>));
    let welcome_ref = Rc::new(RefCell::new(None::<gtk::DrawingArea>));
    let file_list = gtk::ListBox::new();
    file_list.add_css_class("navigation-sidebar");
    file_list.set_selection_mode(gtk::SelectionMode::None);

    let sidebar = build_sidebar(
        &window, &project_name, active_chat.clone(), chat_messages.clone(),
        transcript_buffer.clone(), transcript_scroll_ref.clone(), welcome_ref.clone(),
        file_list.clone(),
    );
    sidebar.set_size_request(210, -1);
    let workspace = build_workspace(
        project_name, active_chat, chat_messages, transcript_buffer,
        transcript_scroll_ref, welcome_ref, root.clone(), content.clone(),
    );

    let files_panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
    files_panel.set_size_request(180, -1);
    files_panel.set_margin_top(12);
    files_panel.set_margin_bottom(12);
    files_panel.set_margin_start(10);
    files_panel.set_margin_end(10);
    let files_title = gtk::Label::new(Some("PROJECT FILES"));
    files_title.add_css_class("caption-heading");
    files_title.add_css_class("dim-label");
    files_title.set_halign(gtk::Align::Start);
    let files_scroll = gtk::ScrolledWindow::new();
    files_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    files_scroll.set_child(Some(&file_list));
    files_scroll.set_vexpand(true);
    files_panel.append(&files_title);
    files_panel.append(&files_scroll);

    content.set_start_child(Some(&workspace));
    content.set_end_child(Some(&files_panel));
    content.set_resize_start_child(true);
    content.set_resize_end_child(false);
    content.set_shrink_start_child(false);
    content.set_shrink_end_child(true);
    root.set_start_child(Some(&sidebar));
    root.set_end_child(Some(&content));
    root.set_resize_start_child(false);
    root.set_resize_end_child(true);
    root.set_shrink_start_child(true);
    root.set_shrink_end_child(false);
    window.set_content(Some(&root));
    window.present();
}

fn patch_logo(width: i32, height: i32) -> gtk::DrawingArea {
    // Render the tiny logo as pixel art. GtkPicture's normal texture scaling
    // interpolates pixels, which makes this 21x4 asset look blurred when
    // enlarged. Cairo's Nearest filter keeps every source pixel crisp.
    let mut png = std::io::Cursor::new(include_bytes!("../assets/patch.png"));
    let surface = match cairo::ImageSurface::create_from_png(&mut png) {
        Ok(surface) => surface,
        Err(error) => {
            eprintln!("Could not load Patch logo: {error}");
            let area = gtk::DrawingArea::new();
            area.set_content_width(width);
            area.set_content_height(height);
            return area;
        }
    };

    let source_width = surface.width();
    let source_height = surface.height();
    let area = gtk::DrawingArea::new();
    area.set_content_width(width);
    area.set_content_height(height);
    area.set_size_request(width, height);
    area.set_draw_func(move |_, context, allocated_width, allocated_height| {
        if source_width <= 0 || source_height <= 0 {
            return;
        }

        // Use an integer scale so source pixels are never stretched by
        // fractional amounts. Centre the result in the allocated area.
        let scale = (allocated_width / source_width)
            .min(allocated_height / source_height)
            .max(1);
        let rendered_width = source_width * scale;
        let rendered_height = source_height * scale;
        let x = (allocated_width - rendered_width) as f64 / 2.0;
        let y = (allocated_height - rendered_height) as f64 / 2.0;

        context.save().ok();
        context.translate(x, y);
        context.scale(scale as f64, scale as f64);
        if context.set_source_surface(&surface, 0.0, 0.0).is_ok() {
            let _ = context.source().set_filter(cairo::Filter::Nearest);
            let _ = context.paint();
        }
        context.restore().ok();
    });
    area
}

fn build_sidebar(
    window: &adw::ApplicationWindow,
    project_name: &gtk::Label,
    active_chat: Rc<Cell<Option<i64>>>,
    chat_messages: Rc<RefCell<Vec<crate::providers::ollama::ChatMessage>>>,
    transcript_buffer: Rc<RefCell<Option<gtk::TextBuffer>>>,
    transcript_scroll_ref: Rc<RefCell<Option<gtk::ScrolledWindow>>>,
    welcome_ref: Rc<RefCell<Option<gtk::DrawingArea>>>,
    file_list: gtk::ListBox,
) -> gtk::Box {
    let sidebar = gtk::Box::new(gtk::Orientation::Vertical, 12);
    sidebar.set_margin_top(16);
    sidebar.set_margin_bottom(16);
    sidebar.set_margin_start(14);
    sidebar.set_margin_end(14);
    sidebar.add_css_class("sidebar");

    let brand = patch_logo(126, 24);
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
    let active_chat_for_project = active_chat.clone();
    let chat_messages_for_project = chat_messages.clone();
    let transcript_buffer_for_project = transcript_buffer.clone();
    let transcript_scroll_for_project = transcript_scroll_ref.clone();
    let welcome_for_project = welcome_ref.clone();
    let file_list_for_project = file_list.clone();

    project.connect_clicked(move |_| {
        // Use GTK's in-process dialog rather than FileChooserNative. The
        // native chooser can trigger a GDK monitor-scale assertion on some
        // Wayland/compositor combinations (notably when the portal reports an
        // invalid scale factor), and has caused the application to crash.
        let dialog = gtk::FileChooserDialog::new(
            Some("Open workspace"),
            Some(&parent),
            gtk::FileChooserAction::SelectFolder,
            &[
                ("Cancel", gtk::ResponseType::Cancel),
                ("Open", gtk::ResponseType::Accept),
            ],
        );
        dialog.set_modal(true);
        let name_label = name_label.clone();
        let history_list = history_list.clone();
        let workspace_id = workspace_id.clone();
        let chooser = dialog.clone();

        dialog.connect_response(move |dialog, response| {
            if response != gtk::ResponseType::Accept {
                dialog.close();
                return;
            }
            let Some(path) = chooser.file().and_then(|file| file.path()) else {
                dialog.close();
                return;
            };
            dialog.close();
            match crate::storage::register_workspace(&path) {
                Ok(id) => {
                    workspace_id.set(Some(id));
                    active_chat_for_project.set(None);
                    chat_messages_for_project.borrow_mut().clear();
                    if let Some(buffer) = transcript_buffer_for_project.borrow().as_ref() {
                        buffer.set_text("");
                    }
                    if let Some(scroll) = transcript_scroll_for_project.borrow().as_ref() {
                        scroll.set_visible(false);
                    }
                    if let Some(logo) = welcome_for_project.borrow().as_ref() {
                        logo.set_visible(true);
                    }
                    name_label.set_text(path.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("Workspace"));
                    while let Some(child) = history_list.first_child() {
                        history_list.remove(&child);
                    }
                    match crate::storage::list_chat_titles(id) {
                        Ok(chats) if !chats.is_empty() => {
                            for (chat_id, title) in chats {
                                append_chat_row(
                                    &history_list, &parent, chat_id, &title,
                                    &workspace_id, &active_chat_for_project, &chat_messages_for_project,
                                    &transcript_buffer_for_project, &transcript_scroll_for_project, &welcome_for_project,
                                );
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
                    populate_file_list(&file_list_for_project, &path);
                }
                Err(error) => eprintln!("Could not register workspace: {error}"),
            }
        });
        dialog.show();
    });

    let history_for_new = history.clone();
    let parent_for_new = window.clone();
    let active_chat_for_new = active_chat.clone();
    let chat_messages_for_new = chat_messages.clone();
    let transcript_buffer_for_new = transcript_buffer.clone();
    let transcript_scroll_for_new = transcript_scroll_ref.clone();
    let welcome_for_new = welcome_ref.clone();
    let workspace_for_new = active_workspace.clone();
    new_chat.connect_clicked(move |_| {
        let Some(workspace_id) = workspace_for_new.get() else {
            return;
        };
        match crate::storage::create_chat(workspace_id) {
            Ok(chat_id) => {
                active_chat_for_new.set(Some(chat_id));
                chat_messages_for_new.borrow_mut().clear();
                if let Some(buffer) = transcript_buffer_for_new.borrow().as_ref() {
                    buffer.set_text("");
                }
                if let Some(scroll) = transcript_scroll_for_new.borrow().as_ref() {
                    scroll.set_visible(true);
                }
                if let Some(welcome) = welcome_for_new.borrow().as_ref() {
                    welcome.set_visible(false);
                }
                append_chat_row(
                    &history_for_new, &parent_for_new, chat_id, "New chat",
                    &workspace_for_new, &active_chat_for_new, &chat_messages_for_new,
                    &transcript_buffer_for_new, &transcript_scroll_for_new, &welcome_for_new,
                );
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


/// Add a selectable chat row with rename and delete actions.
fn append_chat_row(
    history: &gtk::ListBox,
    parent: &adw::ApplicationWindow,
    chat_id: i64,
    title: &str,
    _workspace_id: &Rc<Cell<Option<i64>>>,
    active_chat: &Rc<Cell<Option<i64>>>,
    chat_messages: &Rc<RefCell<Vec<crate::providers::ollama::ChatMessage>>>,
    transcript_buffer: &Rc<RefCell<Option<gtk::TextBuffer>>>,
    transcript_scroll: &Rc<RefCell<Option<gtk::ScrolledWindow>>>,
    welcome: &Rc<RefCell<Option<gtk::DrawingArea>>>,
) {
    let row = gtk::ListBoxRow::new();
    let row_for_delete = row.clone();
    let title_owned = title.to_owned();
    let line = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    line.set_margin_top(4);
    line.set_margin_bottom(4);
    line.set_margin_start(4);
    line.set_margin_end(4);

    let select = gtk::Button::with_label(title);
    select.set_hexpand(true);
    select.set_halign(gtk::Align::Fill);
    select.add_css_class("flat");
    let active_chat_for_select = active_chat.clone();
    let chat_messages_for_select = chat_messages.clone();
    let transcript_buffer_for_select = transcript_buffer.clone();
    let transcript_scroll_for_select = transcript_scroll.clone();
    let welcome_for_select = welcome.clone();
    select.connect_clicked(move |_| {
        match crate::storage::list_messages(chat_id) {
            Ok(messages) => {
                active_chat_for_select.set(Some(chat_id));
                let mut restored = Vec::with_capacity(messages.len());
                let mut rendered = String::new();
                for (role, content) in messages {
                    let label = if role == "user" { "You" } else { "Patch" };
                    rendered.push_str(label);
                    rendered.push('\n');
                    rendered.push_str(&content);
                    rendered.push_str("\n\n");
                    restored.push(crate::providers::ollama::ChatMessage { role, content });
                }
                *chat_messages_for_select.borrow_mut() = restored;
                if let Some(buffer) = transcript_buffer_for_select.borrow().as_ref() {
                    buffer.set_text(&rendered);
                }
                if let Some(scroll) = transcript_scroll_for_select.borrow().as_ref() {
                    scroll.set_visible(true);
                }
                if let Some(logo) = welcome_for_select.borrow().as_ref() {
                    logo.set_visible(false);
                }
            }
            Err(error) => eprintln!("Could not restore chat: {error}"),
        }
    });

    let rename = gtk::Button::with_label("✎");
    rename.add_css_class("flat");
    rename.set_tooltip_text(Some("Rename chat"));
    let parent_for_rename = parent.clone();
    let select_for_rename = select.clone();
    rename.connect_clicked(move |_| {
        let dialog = gtk::Dialog::with_buttons(
            Some("Rename chat"),
            Some(&parent_for_rename),
            gtk::DialogFlags::MODAL,
            &[("Cancel", gtk::ResponseType::Cancel), ("Rename", gtk::ResponseType::Accept)],
        );
        let entry = gtk::Entry::new();
        entry.set_text(&title_owned);
        entry.set_activates_default(true);
        dialog.set_default_response(gtk::ResponseType::Accept);
        dialog.content_area().append(&entry);
        let entry_for_response = entry.clone();
        let select_for_response = select_for_rename.clone();
        dialog.connect_response(move |dialog, response| {
            if response == gtk::ResponseType::Accept {
                let new_title = entry_for_response.text().trim().to_owned();
                if !new_title.is_empty() {
                    match crate::storage::rename_chat(chat_id, &new_title) {
                        Ok(()) => select_for_response.set_label(&new_title),
                        Err(error) => eprintln!("Could not rename chat: {error}"),
                    }
                }
            }
            dialog.close();
        });
        dialog.show();
    });

    let delete = gtk::Button::with_label("×");
    delete.add_css_class("flat");
    delete.add_css_class("destructive-action");
    delete.set_tooltip_text(Some("Delete chat"));
    let parent_for_delete = parent.clone();
    let history_for_delete = history.clone();
    let row_for_delete = row_for_delete.clone();
    let active_chat_for_delete = active_chat.clone();
    let chat_messages_for_delete = chat_messages.clone();
    let transcript_buffer_for_delete = transcript_buffer.clone();
    let transcript_scroll_for_delete = transcript_scroll.clone();
    let welcome_for_delete = welcome.clone();
    delete.connect_clicked(move |_| {
        let dialog = gtk::MessageDialog::new(
            Some(&parent_for_delete),
            gtk::DialogFlags::MODAL,
            gtk::MessageType::Warning,
            gtk::ButtonsType::Cancel,
            "Delete this chat and its message history?",
        );
        dialog.add_button("Delete", gtk::ResponseType::Accept);
        let history = history_for_delete.clone();
        let active_chat = active_chat_for_delete.clone();
        let chat_messages = chat_messages_for_delete.clone();
        let transcript_buffer = transcript_buffer_for_delete.clone();
        let transcript_scroll = transcript_scroll_for_delete.clone();
        let welcome = welcome_for_delete.clone();
        let row_for_response = row_for_delete.clone();
        dialog.connect_response(move |dialog, response| {
            if response == gtk::ResponseType::Accept {
                match crate::storage::delete_chat(chat_id) {
                    Ok(()) => {
                        if active_chat.get() == Some(chat_id) {
                            active_chat.set(None);
                            chat_messages.borrow_mut().clear();
                            if let Some(buffer) = transcript_buffer.borrow().as_ref() {
                                buffer.set_text("");
                            }
                            if let Some(scroll) = transcript_scroll.borrow().as_ref() {
                                scroll.set_visible(false);
                            }
                            if let Some(logo) = welcome.borrow().as_ref() {
                                logo.set_visible(true);
                            }
                        }
                        history.remove(&row_for_response);
                    }
                    Err(error) => eprintln!("Could not delete chat: {error}"),
                }
            }
            dialog.close();
        });
        dialog.show();
    });

    line.append(&select);
    line.append(&rename);
    line.append(&delete);
    row.set_child(Some(&line));
    history.append(&row);
}

/// Populate a compact, depth-limited project tree. Directories are listed
/// before files and common build/VCS directories are omitted.
fn populate_file_list(list: &gtk::ListBox, root: &std::path::Path) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    fn walk(list: &gtk::ListBox, dir: &std::path::Path, depth: usize, root: &std::path::Path) {
        if depth > 4 { return; }
        let Ok(entries) = std::fs::read_dir(dir) else { return; };
        let mut entries: Vec<_> = entries.filter_map(Result::ok).collect();
        entries.sort_by_key(|entry| {
            let is_dir = entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
            (!is_dir, entry.file_name())
        });
        for entry in entries {
            let name = entry.file_name().to_string_lossy().into_owned();
            if matches!(name.as_str(), ".git" | "target" | "node_modules" | ".patch") {
                continue;
            }
            let path = entry.path();
            let is_dir = entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
            let relative = path.strip_prefix(root).unwrap_or(&path);
            let label = gtk::Label::new(Some(&format!(
                "{}{} {}",
                "  ".repeat(depth),
                if is_dir { "▸" } else { "·" },
                relative.file_name().unwrap_or_default().to_string_lossy()
            )));
            label.set_halign(gtk::Align::Start);
            label.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
            label.set_margin_top(2);
            label.set_margin_bottom(2);
            list.append(&label);
            if is_dir { walk(list, &path, depth + 1, root); }
        }
    }
    walk(list, root, 0, root);
}

fn build_workspace(
    project_name: gtk::Label,
    active_chat: Rc<Cell<Option<i64>>>,
    chat_messages: Rc<RefCell<Vec<crate::providers::ollama::ChatMessage>>>,
    transcript_buffer_ref: Rc<RefCell<Option<gtk::TextBuffer>>>,
    transcript_scroll_ref: Rc<RefCell<Option<gtk::ScrolledWindow>>>,
    welcome_ref: Rc<RefCell<Option<gtk::DrawingArea>>>,
    root_paned: gtk::Paned,
    content_paned: gtk::Paned,
) -> gtk::Box {
    let workspace = gtk::Box::new(gtk::Orientation::Vertical, 0);
    workspace.set_hexpand(true);
    workspace.set_vexpand(true);

    let header = gtk::HeaderBar::new();
    header.set_title_widget(Some(&project_name));

    let sidebar_toggle = gtk::Button::from_icon_name("sidebar-show-symbolic");
    sidebar_toggle.set_tooltip_text(Some("Show or hide chat sidebar"));
    let root_for_toggle = root_paned.clone();
    sidebar_toggle.connect_clicked(move |_| {
        if let Some(sidebar) = root_for_toggle.start_child() {
            sidebar.set_visible(!sidebar.is_visible());
        }
    });
    header.pack_start(&sidebar_toggle);

    let model = gtk::ComboBoxText::new();
    model.append_text("Discovering local models…");
    model.set_active(Some(0));
    header.pack_start(&model);

    let files_toggle = gtk::Button::from_icon_name("view-list-symbolic");
    files_toggle.set_tooltip_text(Some("Show or hide project files"));
    // The file browser is the end child of the workspace's parent Paned.
    let content_for_click = content_paned.clone();
    files_toggle.connect_clicked(move |_| {
        if let Some(files) = content_for_click.end_child() {
            files.set_visible(!files.is_visible());
        }
    });
    header.pack_start(&files_toggle);

    let usage = UsageGauge::new();
    header.pack_end(&usage.widget);

    // Network discovery runs off the GTK main thread. The UI is updated only
    // after the result is sent back through the main context.
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let discovered = crate::providers::ollama::OllamaClient::default().list_models();
        let _ = sender.send(discovered);
    });
    let model_for_update = model.clone();
    gtk::glib::timeout_add_local(std::time::Duration::from_millis(150), move || {
        match receiver.try_recv() {
            Ok(discovered) => {
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
                gtk::glib::ControlFlow::Break
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => gtk::glib::ControlFlow::Continue,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => gtk::glib::ControlFlow::Break,
        }
    });

    let conversation = gtk::Box::new(gtk::Orientation::Vertical, 14);
    // A centred GtkBox can shrink to its children's natural size when the
    // transcript becomes visible. Fill the available workspace instead.
    conversation.set_valign(gtk::Align::Fill);
    conversation.set_halign(gtk::Align::Fill);
    conversation.set_hexpand(true);
    conversation.set_vexpand(true);

    let welcome = patch_logo(210, 40);
    welcome.set_halign(gtk::Align::Center);
    conversation.append(&welcome);
    *welcome_ref.borrow_mut() = Some(welcome.clone());

    let transcript_scroll = gtk::ScrolledWindow::new();
    transcript_scroll.set_hexpand(true);
    transcript_scroll.set_vexpand(true);
    transcript_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    let transcript = gtk::TextView::new();
    transcript.set_editable(false);
    transcript.set_cursor_visible(false);
    transcript.set_wrap_mode(gtk::WrapMode::WordChar);
    transcript.set_left_margin(18);
    transcript.set_right_margin(18);
    transcript.set_top_margin(18);
    transcript.set_bottom_margin(18);
    transcript_scroll.set_child(Some(&transcript));
    transcript_scroll.set_hexpand(true);
    transcript_scroll.set_vexpand(true);
    transcript_scroll.set_visible(false);
    *transcript_buffer_ref.borrow_mut() = Some(transcript.buffer());
    *transcript_scroll_ref.borrow_mut() = Some(transcript_scroll.clone());
    conversation.append(&transcript_scroll);
    // The composer owns the mode selector: there is no longer a separate
    // Plan/Build control in the header.
    if let Some(display) = gtk::gdk::Display::default() {
        let css = gtk::CssProvider::new();
        css.load_from_data(
            ".patch-composer { border: 1px solid alpha(@theme_fg_color, 0.22); border-radius: 12px; padding: 5px; }\
             .patch-composer.plan-mode { border-color: #3987d7; background: alpha(#3987d7, 0.055); }\
             .patch-composer.build-mode { border-color: #f07832; background: alpha(#f07832, 0.075); }\
             .patch-mode.plan-mode { color: #65a9f2; background: alpha(#3987d7, 0.16); }\
             .patch-mode.build-mode { color: #ff9858; background: alpha(#f07832, 0.18); }",
        );
        gtk::style_context_add_provider_for_display(
            &display,
            &css,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    let composer_outer = gtk::Box::new(gtk::Orientation::Vertical, 8);
    composer_outer.set_margin_start(28);
    composer_outer.set_margin_end(28);
    composer_outer.set_margin_bottom(12);
    composer_outer.set_vexpand(false);

    let composer = gtk::Box::new(gtk::Orientation::Vertical, 4);
    composer.add_css_class("patch-composer");
    composer.add_css_class("plan-mode");
    composer.set_size_request(-1, 92);
    composer.set_vexpand(false);

    let prompt_scroll = gtk::ScrolledWindow::new();
    prompt_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    prompt_scroll.set_vexpand(false);
    prompt_scroll.set_min_content_height(42);

    let prompt = gtk::TextView::new();
    prompt.set_wrap_mode(gtk::WrapMode::WordChar);
    prompt.set_top_margin(2);
    prompt.set_bottom_margin(2);
    prompt.set_left_margin(8);
    prompt.set_right_margin(8);
    prompt.set_hexpand(true);
    prompt.set_vexpand(false);

    // GtkTextView has no built-in placeholder property. Overlay a dim label
    // and track the buffer so the hint disappears as soon as text is entered.
    let prompt_overlay = gtk::Overlay::new();
    prompt_overlay.set_child(Some(&prompt_scroll));
    prompt_scroll.set_child(Some(&prompt));
    let placeholder = gtk::Label::new(Some("Ask Patch about your code…"));
    placeholder.add_css_class("dim-label");
    placeholder.set_halign(gtk::Align::Start);
    placeholder.set_valign(gtk::Align::Start);
    placeholder.set_margin_start(12);
    placeholder.set_margin_top(10);
    placeholder.set_can_target(false);
    prompt_overlay.add_overlay(&placeholder);
    let prompt_buffer = prompt.buffer();
    {
        let placeholder = placeholder.clone();
        prompt_buffer.connect_changed(move |buffer| {
            let empty = buffer
                .text(&buffer.start_iter(), &buffer.end_iter(), false)
                .is_empty();
            placeholder.set_visible(empty);
        });
    }
    // Prompt submission is connected to the local Ollama streaming API below.

    let composer_footer = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    composer_footer.set_halign(gtk::Align::Fill);

    let mode_button = gtk::Button::with_label("Plan");
    mode_button.add_css_class("flat");
    mode_button.add_css_class("patch-mode");
    mode_button.add_css_class("plan-mode");
    mode_button.set_tooltip_text(Some("Plan mode · press Tab to switch to Build"));

    let mode_hint = gtk::Label::new(Some("Tab to switch"));
    mode_hint.add_css_class("dim-label");
    mode_hint.add_css_class("caption");

    let send = gtk::Button::with_label("Send");
    send.add_css_class("suggested-action");
    send.set_sensitive(false);
    send.set_tooltip_text(Some("Send prompt to the selected local Ollama model"));

    #[derive(Debug)]
    enum StreamEvent {
        Chunk(String),
        Finished(Result<crate::providers::ollama::ChatUsage, String>),
    }

    let busy = Rc::new(Cell::new(false));
    {
        let send = send.clone();
        let prompt_buffer = prompt_buffer.clone();
        let busy = busy.clone();
        prompt_buffer.connect_changed(move |buffer| {
            let has_text = !buffer
                .text(&buffer.start_iter(), &buffer.end_iter(), false)
                .trim()
                .is_empty();
            send.set_sensitive(has_text && !busy.get());
        });
    }

    let build_mode = Rc::new(Cell::new(false));
    let set_mode = {
        let build_mode = build_mode.clone();
        let composer = composer.clone();
        let mode_button = mode_button.clone();
        move |build: bool| {
            build_mode.set(build);
            composer.remove_css_class(if build { "plan-mode" } else { "build-mode" });
            composer.add_css_class(if build { "build-mode" } else { "plan-mode" });
            mode_button.remove_css_class(if build { "plan-mode" } else { "build-mode" });
            mode_button.add_css_class(if build { "build-mode" } else { "plan-mode" });
            mode_button.set_label(if build { "Build" } else { "Plan" });
            mode_button.set_tooltip_text(Some(if build {
                "Build mode · press Tab to switch to Plan"
            } else {
                "Plan mode · press Tab to switch to Build"
            }));
        }
    };

    {
        let build_mode = build_mode.clone();
        let set_mode = set_mode.clone();
        mode_button.connect_clicked(move |_| set_mode(!build_mode.get()));
    }

    let key_controller = gtk::EventControllerKey::new();
    {
        let build_mode = build_mode.clone();
        key_controller.connect_key_pressed(move |_, key, _, _| {
            if key == gtk::gdk::Key::Tab {
                set_mode(!build_mode.get());
                gtk::glib::Propagation::Stop
            } else {
                gtk::glib::Propagation::Proceed
            }
        });
    }
    prompt.add_controller(key_controller);

    {
        let prompt_buffer = prompt_buffer.clone();
        let model = model.clone();
        let active_chat = active_chat.clone();
        let transcript = transcript.clone();
        let transcript_scroll = transcript_scroll.clone();
        let welcome = welcome.clone();
        let chat_messages = chat_messages.clone();
        let build_mode = build_mode.clone();
        let busy = busy.clone();
        let send_button = send.clone();
        let usage_widget = usage.widget.clone();
        let usage_ledger = usage.ledger.clone();
        send.connect_clicked(move |_| {
            if busy.get() {
                return;
            }
            let prompt_text = prompt_buffer
                .text(&prompt_buffer.start_iter(), &prompt_buffer.end_iter(), false)
                .trim()
                .to_owned();
            if prompt_text.is_empty() {
                return;
            }
            let Some(chat_id) = active_chat.get() else {
                transcript_scroll.set_visible(true);
                welcome.set_visible(false);
                let buffer = transcript.buffer();
                buffer.set_text("Open a project and create a chat before sending a prompt.\n");
                transcript_scroll.queue_draw();
                return;
            };
            let Some(model_name) = model.active_text().map(|value| value.to_string()) else {
                return;
            };
            if model_name.starts_with("Discovering") || model_name.starts_with("No local")
                || model_name.starts_with("Ollama unavailable")
            {
                return;
            }

            busy.set(true);
            send_button.set_sensitive(false);
            prompt_buffer.set_text("");
            welcome.set_visible(false);
            transcript_scroll.set_visible(true);

            if let Err(error) = crate::storage::save_message(chat_id, "user", &prompt_text, None, None) {
                eprintln!("Could not save user message: {error}");
            }
            let transcript_buffer = transcript.buffer();
            let mut end = transcript_buffer.end_iter();
            transcript_buffer.insert(&mut end, &format!("You\n{prompt_text}\n\nPatch\n"));

            let mut request_messages = chat_messages.borrow().clone();
            let mode_instruction = if build_mode.get() {
                "You are Patch, a coding assistant in Build mode. Help implement requested changes. Do not claim to have edited files or run commands unless tools actually performed those actions."
            } else {
                "You are Patch, a coding assistant in Plan mode. Analyse and explain; do not propose that you have modified files or run commands."
            };
            request_messages.insert(0, crate::providers::ollama::ChatMessage {
                role: "system".to_owned(),
                content: mode_instruction.to_owned(),
            });
            request_messages.push(crate::providers::ollama::ChatMessage {
                role: "user".to_owned(),
                content: prompt_text.clone(),
            });
            chat_messages.borrow_mut().push(crate::providers::ollama::ChatMessage {
                role: "user".to_owned(),
                content: prompt_text,
            });

            let (tx, rx) = std::sync::mpsc::channel::<StreamEvent>();
            std::thread::spawn(move || {
                let result = crate::providers::ollama::OllamaClient::default().chat_stream(
                    &model_name,
                    &request_messages,
                    |chunk| { let _ = tx.send(StreamEvent::Chunk(chunk)); },
                ).map_err(|error| error.to_string());
                let _ = tx.send(StreamEvent::Finished(result));
            });

            let transcript_buffer = transcript.buffer();
            let transcript_scroll = transcript_scroll.clone();
            let chat_messages = chat_messages.clone();
            let busy = busy.clone();
            let send = send_button.clone();
            let prompt_buffer = prompt_buffer.clone();
            let usage_widget = usage_widget.clone();
            let usage_ledger = usage_ledger.clone();
            let mut assistant_text = String::new();
            gtk::glib::timeout_add_local(std::time::Duration::from_millis(25), move || {
                loop {
                    match rx.try_recv() {
                        Ok(StreamEvent::Chunk(chunk)) => {
                            assistant_text.push_str(&chunk);
                            let mut end = transcript_buffer.end_iter();
                            transcript_buffer.insert(&mut end, &chunk);
                        }
                        Ok(StreamEvent::Finished(Ok(stats))) => {
                            let mut end = transcript_buffer.end_iter();
                            transcript_buffer.insert(&mut end, "\n\n");
                            if let Err(error) = crate::storage::save_message(
                                chat_id, "assistant", &assistant_text,
                                stats.prompt_tokens, stats.completion_tokens,
                            ) {
                                eprintln!("Could not save assistant message: {error}");
                            }
                            chat_messages.borrow_mut().push(crate::providers::ollama::ChatMessage {
                                role: "assistant".to_owned(),
                                content: assistant_text.clone(),
                            });
                            if let (Some(input), Some(output)) = (stats.prompt_tokens, stats.completion_tokens) {
                                usage_ledger.borrow_mut().record(
                                    input, output,
                                    Some(input.saturating_add(output)), None,
                                );
                                usage_widget.queue_draw();
                            }
                            busy.set(false);
                            let has_text = !prompt_buffer
                                .text(&prompt_buffer.start_iter(), &prompt_buffer.end_iter(), false)
                                .trim().is_empty();
                            send.set_sensitive(has_text);
                            return gtk::glib::ControlFlow::Break;
                        }
                        Ok(StreamEvent::Finished(Err(error))) => {
                            let mut end = transcript_buffer.end_iter();
                            transcript_buffer.insert(&mut end, &format!("\n\n[Ollama error: {error}]\n\n"));
                            busy.set(false);
                            let has_text = !prompt_buffer
                                .text(&prompt_buffer.start_iter(), &prompt_buffer.end_iter(), false)
                                .trim().is_empty();
                            send.set_sensitive(has_text);
                            return gtk::glib::ControlFlow::Break;
                        }
                        Err(std::sync::mpsc::TryRecvError::Empty) => {
                            transcript_scroll.queue_draw();
                            return gtk::glib::ControlFlow::Continue;
                        }
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                            busy.set(false);
                            send.set_sensitive(true);
                            return gtk::glib::ControlFlow::Break;
                        }
                    }
                }
            });
        });
    }

    composer_footer.append(&mode_button);
    composer_footer.append(&mode_hint);
    let footer_spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    footer_spacer.set_hexpand(true);
    composer_footer.append(&footer_spacer);
    composer_footer.append(&send);

    composer.append(&prompt_overlay);
    composer.append(&composer_footer);
    composer_outer.append(&composer);
    workspace.append(&header);
    workspace.append(&conversation);
    workspace.append(&composer_outer);
    workspace
}

/// Circular context indicator with a hover breakdown. Updates must be fed
/// provider-reported usage and an explicitly configured model rate card.
pub struct UsageGauge {
    pub widget: gtk::DrawingArea,
    ledger: Rc<RefCell<crate::usage::UsageLedger>>,
    pricing: Rc<Cell<Option<crate::usage::TokenPricing>>>,
}

impl UsageGauge {
    pub fn new() -> Self {
        let widget = gtk::DrawingArea::new();
        widget.set_content_width(40);
        widget.set_content_height(40);

        let ledger = Rc::new(RefCell::new(crate::usage::UsageLedger::default()));
        let pricing = Rc::new(Cell::new(None));
        let draw_ledger = ledger.clone();

        widget.set_draw_func(move |_, context, width, height| {
            use std::f64::consts::PI;
            let cx = width as f64 / 2.0;
            let cy = height as f64 / 2.0;
            let radius = (width.min(height) as f64 / 2.0) - 4.0;
            let counts = draw_ledger.borrow().counts();
            let fraction = counts.context_fraction();

            context.set_line_width(3.5);
            context.set_source_rgba(0.5, 0.5, 0.5, 0.28);
            context.arc(cx, cy, radius, 0.0, 2.0 * PI);
            let _ = context.stroke();

            if let Some(fraction) = fraction {
                context.set_source_rgba(0.92, 0.32, 0.08, 0.96);
                context.set_line_width(3.5);
                context.arc(cx, cy, radius, -PI / 2.0, -PI / 2.0 + fraction * 2.0 * PI);
                let _ = context.stroke();
            }

            context.select_font_face(
                "Sans",
                gtk::cairo::FontSlant::Normal,
                gtk::cairo::FontWeight::Bold,
            );
            context.set_font_size(10.0);
            context.set_source_rgba(0.85, 0.85, 0.85, 0.95);
            let label = fraction
                .map(|value| format!("{:.0}%", value * 100.0))
                .unwrap_or_else(|| "—".to_owned());
            if let Ok(extents) = context.text_extents(&label) {
                context.move_to(cx - extents.width() / 2.0, cy + extents.height() / 2.0);
                let _ = context.show_text(&label);
            }
        });

        let gauge = Self { widget, ledger, pricing };
        gauge.refresh_tooltip();
        gauge
    }

    /// Record one completed model response using provider-supplied usage
    /// metadata. Do not call this with locally estimated token counts.
    pub fn record_provider_usage(
        &self,
        input_tokens: u64,
        output_tokens: u64,
        context_tokens: Option<u64>,
        context_limit: Option<u64>,
    ) {
        self.ledger.borrow_mut().record(
            input_tokens,
            output_tokens,
            context_tokens,
            context_limit,
        );
        self.refresh();
    }

    /// Set rates for the selected provider/model. Pass zero rates for a local
    /// Ollama model; pass None when hosted pricing is not verified.
    pub fn set_pricing(&self, pricing: Option<crate::usage::TokenPricing>) {
        self.pricing.set(pricing);
        self.refresh();
    }

    fn refresh(&self) {
        self.widget.queue_draw();
        self.refresh_tooltip();
    }

    fn refresh_tooltip(&self) {
        let ledger = self.ledger.borrow();
        let counts = ledger.counts();
        let context = match (counts.context, counts.context_limit) {
            (Some(used), Some(limit)) if limit > 0 => {
                format!(
                    "{} / {} tokens ({:.1}%)",
                    crate::usage::compact_tokens(used),
                    crate::usage::compact_tokens(limit),
                    (used as f64 / limit as f64 * 100.0).min(100.0)
                )
            }
            _ => "Unavailable from model metadata".to_owned(),
        };
        let cost = match ledger.cost(self.pricing.get()) {
            crate::usage::CostDisplay::Known(value) => format!("${value:.6} USD"),
            crate::usage::CostDisplay::Unknown => "Unavailable — no verified rate card".to_owned(),
        };
        let tooltip = format!(
            "Context: {context}\nTokens burned: {}\nInput: {} · Output: {}\nEstimated cost: {cost}",
            crate::usage::compact_tokens(counts.total_burned()),
            crate::usage::compact_tokens(counts.input),
            crate::usage::compact_tokens(counts.output),
        );
        self.widget.set_tooltip_text(Some(&tooltip));
    }
}
