//! Native Markdown rendering for the chat transcript.
//!
//! Markdown is parsed into GTK text-buffer tags rather than displayed as
//! literal source. Code fences remain monospace; diff fences colour additions,
//! removals, and hunk headers distinctly.

use gtk::prelude::*;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

pub fn render_conversation(
    buffer: &gtk::TextBuffer,
    messages: &[crate::providers::ollama::ChatMessage],
) {
    buffer.set_text("");
    let table = buffer.tag_table();

    let heading = make_tag("patch-md-heading");
    heading.set_weight(700);
    heading.set_scale(1.12);
    table.add(&heading);

    let bold = make_tag("patch-md-bold");
    bold.set_weight(700);
    table.add(&bold);

    let italic = make_tag("patch-md-italic");
    italic.set_style(gtk::pango::Style::Italic);
    table.add(&italic);

    let inline_code = make_tag("patch-md-inline-code");
    inline_code.set_family(Some("monospace"));
    inline_code.set_foreground(Some("#e6a15a"));
    inline_code.set_background(Some("#292d35"));
    table.add(&inline_code);

    let code = make_tag("patch-md-code");
    code.set_family(Some("monospace"));
    code.set_foreground(Some("#d7dae0"));
    code.set_background(Some("#20242b"));
    table.add(&code);

    let added = make_tag("patch-md-diff-added");
    added.set_family(Some("monospace"));
    added.set_foreground(Some("#a6e3a1"));
    added.set_background(Some("#203328"));
    table.add(&added);

    let removed = make_tag("patch-md-diff-removed");
    removed.set_family(Some("monospace"));
    removed.set_foreground(Some("#f38ba8"));
    removed.set_background(Some("#3b252d"));
    table.add(&removed);

    let hunk = make_tag("patch-md-diff-hunk");
    hunk.set_family(Some("monospace"));
    hunk.set_foreground(Some("#89b4fa"));
    table.add(&hunk);

    let role_tag = make_tag("patch-md-role");
    role_tag.set_weight(700);
    role_tag.set_foreground(Some("#8ab4f8"));
    table.add(&role_tag);

    for (index, message) in messages.iter().enumerate() {
        let mut end = buffer.end_iter();
        if index > 0 {
            buffer.insert(&mut end, "\n");
        }
        let role = if message.role == "user" { "You" } else { "Patch" };
        buffer.insert_with_tags(&mut end, role, &[&role_tag]);
        buffer.insert(&mut end, "\n");
        render_markdown(buffer, &message.content, &heading, &bold, &italic,
            &inline_code, &code, &added, &removed, &hunk);
        let mut end = buffer.end_iter();
        buffer.insert(&mut end, "\n");
    }
}

fn make_tag(name: &str) -> gtk::TextTag {
    gtk::TextTag::builder().name(name).build()
}

#[allow(clippy::too_many_arguments)]
fn render_markdown(
    buffer: &gtk::TextBuffer,
    source: &str,
    heading: &gtk::TextTag,
    bold: &gtk::TextTag,
    italic: &gtk::TextTag,
    inline_code: &gtk::TextTag,
    code: &gtk::TextTag,
    added: &gtk::TextTag,
    removed: &gtk::TextTag,
    hunk: &gtk::TextTag,
) {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES;
    let parser = Parser::new_ext(source, options);
    let mut active: Vec<&gtk::TextTag> = Vec::new();
    let mut in_code_block = false;
    let mut is_diff = false;

    for event in parser {
        match event {
            Event::Start(Tag::Heading { .. }) => active.push(heading),
            Event::End(TagEnd::Heading(_)) => {
                active.pop();
                insert(buffer, "\n", &active);
            }
            Event::Start(Tag::Strong) => active.push(bold),
            Event::End(TagEnd::Strong) => { active.pop(); }
            Event::Start(Tag::Emphasis) => active.push(italic),
            Event::End(TagEnd::Emphasis) => { active.pop(); }
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                is_diff = match kind {
                    CodeBlockKind::Fenced(info) => {
                        let language = info.trim().to_ascii_lowercase();
                        language == "diff" || language == "patch" || language.starts_with("diff ")
                    }
                    CodeBlockKind::Indented => false,
                };
                insert(buffer, "\n", &active);
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                is_diff = false;
                insert(buffer, "\n", &active);
            }
            Event::Start(Tag::Item) => insert(buffer, "• ", &active),
            Event::End(TagEnd::Item) => insert(buffer, "\n", &active),
            Event::End(TagEnd::Paragraph) => insert(buffer, "\n\n", &active),
            Event::Start(Tag::Table(_)) => insert(buffer, "\n", &active),
            Event::End(TagEnd::Table) => insert(buffer, "\n", &active),
            Event::End(TagEnd::TableRow) => insert(buffer, "\n", &active),
            Event::End(TagEnd::TableCell) => insert(buffer, "  ", &active),
            Event::Rule => insert(buffer, "────────────────────────\n", &active),
            Event::Text(text) if in_code_block && is_diff => {
                for line in text.lines_inclusive() {
                    let tag = if line.starts_with('+') && !line.starts_with("+++") {
                        added
                    } else if line.starts_with('-') && !line.starts_with("---") {
                        removed
                    } else if line.starts_with("@@") {
                        hunk
                    } else {
                        code
                    };
                    insert(buffer, line, &[tag]);
                }
            }
            Event::Text(text) if in_code_block => insert(buffer, &text, &[code]),
            Event::Text(text) => insert(buffer, &text, &active),
            Event::Code(text) => insert(buffer, &text, &[inline_code]),
            Event::SoftBreak | Event::HardBreak => insert(buffer, "\n", &active),
            Event::Html(html) | Event::InlineHtml(html) => insert(buffer, &html, &active),
            _ => {}
        }
    }
}

fn insert(buffer: &gtk::TextBuffer, text: &str, tags: &[&gtk::TextTag]) {
    if text.is_empty() {
        return;
    }
    let mut end = buffer.end_iter();
    buffer.insert_with_tags(&mut end, text, tags);
}
