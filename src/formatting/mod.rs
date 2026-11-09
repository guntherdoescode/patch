//! Shared formatting pipeline for Patch's chat and source views.

mod markdown;
mod syntax;

pub use markdown::render_conversation;
