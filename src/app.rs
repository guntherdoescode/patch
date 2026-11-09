use gpui::{App, Context, IntoElement, Render, Window, div, prelude::*, px, rgb};

const BG: u32 = 0x111315;
const SIDEBAR: u32 = 0x191c1f;
const PANEL: u32 = 0x202428;
const BORDER: u32 = 0x30363b;
const TEXT: u32 = 0xe8e9e9;
const MUTED: u32 = 0x92999f;
const ACCENT: u32 = 0xff5100;
const BLUE: u32 = 0x3987d7;

#[derive(Default)]
pub struct PatchApp {
    build_mode: bool,
    chat_count: usize,
    status: String,
}

impl PatchApp {
    fn start_chat(&mut self, _event: &gpui::ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.chat_count += 1;
        self.status = format!("Chat {} ready", self.chat_count);
        cx.notify();
    }

    fn toggle_mode(&mut self, _event: &gpui::ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.build_mode = !self.build_mode;
        self.status = if self.build_mode {
            "Build mode selected · changes will require approval".into()
        } else {
            "Plan mode selected · read-only operations only".into()
        };
        cx.notify();
    }
}

impl Render for PatchApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mode_label = if self.build_mode { "BUILD" } else { "PLAN" };
        let mode_colour = if self.build_mode { ACCENT } else { BLUE };
        let status = if self.status.is_empty() {
            "Choose a workspace to begin".to_owned()
        } else {
            self.status.clone()
        };

        div()
            .size_full()
            .flex()
            .bg(rgb(BG))
            .text_color(rgb(TEXT))
            .child(
                div()
                    .w(px(248.))
                    .h_full()
                    .flex()
                    .flex_col()
                    .gap_5()
                    .p_4()
                    .bg(rgb(SIDEBAR))
                    .border_r_1()
                    .border_color(rgb(BORDER))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .w(px(9.))
                                    .h(px(9.))
                                    .rounded_sm()
                                    .bg(rgb(ACCENT)),
                            )
                            .child(div().text_size(px(22.)).font_weight(gpui::FontWeight::BOLD).child("PATCH")),
                    )
                    .child(
                        div()
                            .id("new-chat")
                            .w_full()
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .bg(rgb(ACCENT))
                            .text_color(rgb(0xffffff))
                            .cursor_pointer()
                            .child("+   New chat")
                            .on_click(cx.listener(Self::start_chat)),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(div().text_size(px(10.)).font_weight(gpui::FontWeight::BOLD).text_color(rgb(MUTED)).child("WORKSPACE"))
                            .child(
                                div()
                                    .px_3()
                                    .py_3()
                                    .rounded_md()
                                    .bg(rgb(PANEL))
                                    .border_1()
                                    .border_color(rgb(BORDER))
                                    .text_color(rgb(MUTED))
                                    .child("No project open"),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(div().text_size(px(10.)).font_weight(gpui::FontWeight::BOLD).text_color(rgb(MUTED)).child("RECENT CHATS"))
                            .child(div().text_size(px(12.)).text_color(rgb(MUTED)).child(if self.chat_count == 0 {
                                "Your conversations will appear here".to_owned()
                            } else {
                                format!("{} chat(s) created this session", self.chat_count)
                            })),
                    )
                    .child(div().flex_1())
                    .child(div().text_size(px(11.)).text_color(rgb(MUTED)).child("PATCH  ·  GPUI PREVIEW")),
            )
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .h(px(58.))
                            .w_full()
                            .px_6()
                            .flex()
                            .items_center()
                            .justify_between()
                            .border_b_1()
                            .border_color(rgb(BORDER))
                            .child(div().text_size(px(13.)).text_color(rgb(MUTED)).child("No project open"))
                            .child(
                                div()
                                    .px_3()
                                    .py_1()
                                    .rounded_md()
                                    .bg(rgb(PANEL))
                                    .border_1()
                                    .border_color(rgb(BORDER))
                                    .text_size(px(11.))
                                    .child("LOCAL  ·  OLLAMA"),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .items_center()
                            .justify_center()
                            .gap_4()
                            .child(
                                div()
                                    .text_size(px(34.))
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .child("What are we fixing today?"),
                            )
                            .child(
                                div()
                                    .max_w(px(520.))
                                    .text_center()
                                    .text_size(px(14.))
                                    .text_color(rgb(MUTED))
                                    .child("Open a code workspace and connect a local model to start working with Patch."),
                            )
                            .child(
                                div()
                                    .mt_3()
                                    .flex()
                                    .gap_3()
                                    .child(
                                        div()
                                            .px_4()
                                            .py_2()
                                            .rounded_md()
                                            .bg(rgb(PANEL))
                                            .border_1()
                                            .border_color(rgb(BORDER))
                                            .text_size(px(12.))
                                            .text_color(rgb(MUTED))
                                            .child("Workspace picker · coming next"),
                                    )
                                    .child(
                                        div()
                                            .px_4()
                                            .py_2()
                                            .rounded_md()
                                            .bg(rgb(PANEL))
                                            .border_1()
                                            .border_color(rgb(BORDER))
                                            .text_size(px(12.))
                                            .text_color(rgb(MUTED))
                                            .child("Ollama discovery · coming next"),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .w_full()
                            .px_6()
                            .pb_4()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .w_full()
                                    .p_3()
                                    .rounded_lg()
                                    .bg(rgb(PANEL))
                                    .border_1()
                                    .border_color(rgb(BORDER))
                                    .text_color(rgb(MUTED))
                                    .child("Prompt composer migration is next"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .id("mode-toggle")
                                            .px_3()
                                            .py_1()
                                            .rounded_md()
                                            .bg(rgb(mode_colour))
                                            .text_color(rgb(0xffffff))
                                            .text_size(px(11.))
                                            .cursor_pointer()
                                            .child(mode_label)
                                            .on_click(cx.listener(Self::toggle_mode)),
                                    )
                                    .child(div().text_size(px(11.)).text_color(rgb(MUTED)).child(status)),
                            ),
                    ),
            )
    }
}
