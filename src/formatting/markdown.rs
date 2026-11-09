//! Native Markdown rendering for the chat transcript.
//!
//! Markdown supplies document structure; Syntect supplies language-aware
//! colouring inside fenced code blocks. Unknown languages remain monospaced.

use std::collections::HashMap;

use gtk::prelude::*;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

use super::syntax::highlight_code;

pub fn render_conversation(
    buffer: &gtk::TextBuffer,
    messages: &[crate::providers::ollama::ChatMessage],
) {
    buffer.set_text("");
    let table = buffer.tag_table();
    let mut syntax_tags = HashMap::new();

    let heading = make_tag(&table, "patch-md-heading");
    heading.set_weight(700);
    heading.set_scale(1.12);

    let bold = make_tag(&table, "patch-md-bold");
    bold.set_weight(700);

    let italic = make_tag(&table, "patch-md-italic");
    italic.set_style(gtk::pango::Style::Italic);

    let inline_code = make_tag(&table, "patch-md-inline-code");
    inline_code.set_family(Some("monospace"));
    inline_code.set_foreground(Some("#e6a15a"));
    inline_code.set_background(Some("#292d35"));

    let code = make_tag(&table, "patch-md-code");
    code.set_family(Some("monospace"));
    code.set_foreground(Some("#d7dae0"));
    code.set_background(Some("#20242b"));

    let added = make_tag(&table, "patch-md-diff-added");
    added.set_family(Some("monospace"));
    added.set_foreground(Some("#a6e3a1"));
    added.set_background(Some("#203328"));

    let removed = make_tag(&table, "patch-md-diff-removed");
    removed.set_family(Some("monospace"));
    removed.set_foreground(Some("#f38ba8"));
    removed.set_background(Some("#3b252d"));

    let hunk = make_tag(&table, "patch-md-diff-hunk");
    hunk.set_family(Some("monospace"));
    hunk.set_foreground(Some("#89b4fa"));

    let role_tag = make_tag(&table, "patch-md-role");
    role_tag.set_weight(700);
    role_tag.set_foreground(Some("#8ab4f8"));

    for (index, message) in messages.iter().enumerate() {
        let mut end = buffer.end_iter();
        if index > 0 {
            buffer.insert(&mut end, "\n");
        }
        let role = if message.role == "user" { "You" } else { "Patch" };
        buffer.insert_with_tags(&mut end, role, &[&role_tag]);
        buffer.insert(&mut end, "\n");
        render_markdown(
            buffer, &message.content, &table, &mut syntax_tags,
            &heading, &bold, &italic, &inline_code, &code, &added, &removed, &hunk,
        );
        let mut end = buffer.end_iter();
        buffer.insert(&mut end, "\n");
    }
}

fn make_tag(table: &gtk::TextTagTable, name: &str) -> gtk::TextTag {
    if let Some(tag) = table.lookup(name) {
        return tag;
    }
    let tag = gtk::TextTag::builder().name(name).build();
    table.add(&tag);
    tag
}

#[allow(clippy::too_many_arguments)]
fn render_markdown(
    buffer: &gtk::TextBuffer,
    source: &str,
    table: &gtk::TextTagTable,
    syntax_tags: &mut HashMap<String, gtk::TextTag>,
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
    let mut code_language: Option<String> = None;
    let mut code_content = String::new();
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
                code_content.clear();
                code_language = match kind {
                    CodeBlockKind::Fenced(info) => info
                        .split_whitespace()
                        .next()
                        .map(str::to_ascii_lowercase),
                    CodeBlockKind::Indented => None,
                };
                is_diff = code_language.as_deref()
                    .is_some_and(|language| language == "diff" || language == "patch");
                insert(buffer, "\n", &active);
            }
            Event::End(TagEnd::CodeBlock) => {
                if is_diff {
                    for line in code_content.split_inclusive('\n') {
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
                } else if let Some(language) = code_language.as_deref() {
                    if let Some(spans) = highlight_code(language, &code_content) {
                        for span in spans {
                            let tag = syntax_tags.entry(span.foreground.clone()).or_insert_with(|| {
                                let tag_name = format!("patch-syntax-{}", span.foreground.trim_start_matches('#'));
                                let tag = make_tag(table, &tag_name);
                                tag.set_family(Some("monospace"));
                                tag.set_foreground(Some(&span.foreground));
                                tag
                            });
                            insert(buffer, &span.text, &[code, tag]);
                        }
                    } else {
                        insert(buffer, &code_content, &[code]);
                    }
                } else {
                    insert(buffer, &code_content, &[code]);
                }
                code_language = None;
                code_content.clear();
                in_code_block = false;
                is_diff = false;
                insert(buffer, "\n", &active);
            }
            Event::Text(text) if in_code_block => code_content.push_str(&text),
            Event::Start(Tag::Item) => insert(buffer, "• ", &active),
            Event::End(TagEnd::Item) => insert(buffer, "\n", &active),
            Event::End(TagEnd::Paragraph) => insert(buffer, "\n\n", &active),
            Event::Start(Tag::Table(_)) => insert(buffer, "\n", &active),
            Event::End(TagEnd::Table) => insert(buffer, "\n", &active),
            Event::End(TagEnd::TableRow) => insert(buffer, "\n", &active),
            Event::End(TagEnd::TableCell) => insert(buffer, "  ", &active),
            Event::Rule => insert(buffer, "────────────────────────\n", &active),
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
