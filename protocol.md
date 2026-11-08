# WCH-LinkRV Protocol

> **This file has been superseded.** The protocol reference is now a
> multi-page mdBook site built from `docs/`.

- Read it on the web: <https://ch32-rs.github.io/wlink/> (once GitHub Pages is
  enabled for this repository).
- Read it locally: [`docs/protocol/overview.md`](docs/protocol/overview.md) is
  the entry point, and [`docs/SUMMARY.md`](docs/SUMMARY.md) is the full table
  of contents.
- Build it locally: `cargo install mdbook --locked && mdbook serve`.

The old 168-line version of this document was last updated in 2023 and no
longer matched the implementation — it had empty sections, `???` markers, an
outdated chip family list and misidentified the `0x0c` command. The new
reference derives every command table from `src/commands/`, `src/probe.rs` and
`src/operations.rs`, and keeps unconfirmed captures explicitly labelled
`unverified` in
[`docs/protocol/unverified.md`](docs/protocol/unverified.md).

See [issue #30](https://github.com/ch32-rs/wlink/issues/30) for the original
request to bring the protocol documentation up to date.
