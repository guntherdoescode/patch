use adw::prelude::*;
use gtk::prelude::*;
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

    let model = gtk::ComboBoxText::new();
    model.append_text("Discovering local models…");
    model.set_active(Some(0));
    header.pack_start(&model);

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
    conversation.set_valign(gtk::Align::Center);
    conversation.set_halign(gtk::Align::Center);
    conversation.set_hexpand(true);
    conversation.set_vexpand(true);

    let welcome = patch_logo(210, 40);
    welcome.set_halign(gtk::Align::Center);
    conversation.append(&welcome);

    // The composer owns the mode selector: there is no longer a separate
    // Plan/Build control in the header.
    if let Some(display) = gtk::gdk::Display::default() {
        let css = gtk::CssProvider::new();
        css.load_from_data(
            ".patch-composer { border: 1px solid alpha(@theme_fg_color, 0.22); border-radius: 14px; padding: 8px; }\
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
    composer_outer.set_margin_bottom(22);

    let composer = gtk::Box::new(gtk::Orientation::Vertical, 8);
    composer.add_css_class("patch-composer");
    composer.add_css_class("plan-mode");
    composer.set_size_request(-1, 132);

    let prompt_scroll = gtk::ScrolledWindow::new();
    prompt_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    prompt_scroll.set_vexpand(true);
    prompt_scroll.set_min_content_height(76);

    let prompt = gtk::TextView::new();
    prompt.set_wrap_mode(gtk::WrapMode::WordChar);
    prompt.set_top_margin(6);
    prompt.set_bottom_margin(6);
    prompt.set_left_margin(8);
    prompt.set_right_margin(8);
    prompt.set_hexpand(true);
    prompt.set_vexpand(true);
    prompt.set_placeholder_text(Some("Ask Patch about your code…"));
    // Text entry is enabled so users can draft prompts. Sending remains
    // disabled until the agent request pipeline is implemented.
    prompt_scroll.set_child(Some(&prompt));

    let composer_footer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
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
    send.set_tooltip_text(Some("Sending will be enabled when the agent pipeline is ready"));

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

    composer_footer.append(&mode_button);
    composer_footer.append(&mode_hint);
    let footer_spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    footer_spacer.set_hexpand(true);
    composer_footer.append(&footer_spacer);
    composer_footer.append(&send);

    composer.append(&prompt_scroll);
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
