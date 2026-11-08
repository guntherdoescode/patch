# Patch

A native Rust coding agent built around a simple principle: **the model may propose actions, but the application owns permission enforcement.**

Patch is being developed as a desktop GUI first. The initial interface is a native GTK4/libadwaita shell, follows the desktop light/dark preference, and is intended to sit comfortably on COSMIC and other GTK desktops.

## Current status

Early foundation. The repository contains the GTK desktop shell, workspace-scoped SQLite storage, local Ollama discovery, token accounting primitives and an initial permission policy. The full agent loop, streaming, editing and hosted API adapters are not implemented yet.

## Design principles

- **Plan means read-only.** File writes, deletes, patches and mutating commands are denied in Plan mode, even if an approval flag is supplied.
- **Build is gated.** Mutating operations require explicit approval at the operation boundary.
- **Workspace-scoped history.** Conversations belong to a project directory; the sidebar only shows chats for the active workspace.
- **Local-first.** Ollama is a first-class provider, with model discovery rather than a hard-coded model list.
- **Reviewable changes.** File edits are presented as diffs before approval and application.
- **Small and responsive.** Stream output, bound retained conversation context, and virtualise long histories and tool output.
- **Honest usage accounting.** Keep context occupancy separate from cumulative input/output tokens. Cost is calculated from an explicit provider/model rate card; unknown rates remain unknown.

## Planned architecture

| Area | Responsibility |
| --- | --- |
| app | GTK application and view composition |
| agent | Conversation loop, streaming events and context management |
| providers | Provider interface, Ollama discovery and remote APIs |
| permissions | Plan/Build policy and approval decisions |
| tools | Read-only and mutating tool implementations |
| workspace | Project roots, path validation and external-path consent |
| diff | Unified diff parsing and review presentation |
| storage | SQLite chats, messages, settings and migrations |
| secrets | API-key storage through the operating system secret service |
| usage | Provider-reported token counts, context occupancy and model-specific cost estimates |

## Development

System GTK4 and libadwaita development packages are required.

    cargo run

Run the permission policy tests with:

    cargo test

## Security model

The UI is not a security boundary. Every tool call must be checked by a central permission policy immediately before execution. Path access must be canonicalised and checked against the active workspace; external paths require a separate user grant. Plan mode must not expose mutating tool implementations to the agent at all.

API keys must never be written to chat history, logs, crash reports or ordinary application settings. The eventual secrets implementation should use the desktop secret service, with a carefully documented fallback if unavailable.

## Usage accounting

The usage ledger accepts provider-reported input/output token counts and separately tracks the latest context size. Context occupancy is only displayed when both the used-token count and the model context limit are known. Model prices are stored as separate input/output USD-per-million rates with a source and verification timestamp. Patch must not infer prices from token counts or assume every provider exposes a pricing endpoint; where a rate card is missing, the UI must display cost as unavailable rather than show a misleading estimate. Ollama requests have no hosted API token charge.
