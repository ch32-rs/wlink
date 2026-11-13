# wlink Documentation

This site is the documentation for [wlink](https://github.com/ch32-rs/wlink), the
command-line flash and debug tool for WCH-Link probes and WCH RISC-V MCUs.

Its main part is a **protocol reference for the WCH-LinkRV USB protocol** — the
private protocol spoken between the host and the WCH-Link probe. The old
`protocol.md` at the repository root was written in 2023 and had stopped
tracking the implementation; this reference is rebuilt from the source code so
that the byte-level description and `src/commands/` cannot silently diverge.

> **This is a reverse-engineered description, not an official WCH specification.**
> WCH does not publish a protocol specification for the WCH-Link. Everything
> here comes from reading wlink's implementation, from USB captures, and from
> other open-source implementations listed in [External References](references.md).
> Entries marked `unverified` are observations only.

## Where to start

| If you want to… | Read |
| --- | --- |
| Understand the wire format | [USB Transport](protocol/usb-transport.md), then [Frame Format](protocol/frame-format.md) |
| Implement a command | [Command Reference](protocol/commands.md) and [Control Subcommands](protocol/control-subcommands.md) |
| Understand how a flash operation is sequenced | [Operation Flows](protocol/flows.md) |
| Work with the RISC-V debug module | [DMI and the Debug Module](protocol/dmi.md) |
| Look up an ID, flag or constant | [Constants and Chip Families](protocol/constants.md) |
| Find unimplemented or uncertain commands | [Unverified Commands and Captures](protocol/unverified.md) |
| Wire up a specific MCU | The *Chip Notes* section in the sidebar |

## Source of truth

Protocol knowledge is derived from these files in the repository. When the
documentation and the code disagree, **the code wins** — please open a PR that
corrects the documentation.

| File | What it defines |
| --- | --- |
| `src/commands/mod.rs` | Frame construction and parsing, command IDs, payload layouts |
| `src/commands/control.rs` | The complete `0x0d` control subcommand set, firmware version encoding |
| `src/probe.rs` | USB IDs, endpoints, raw send/receive, mode switching |
| `src/operations.rs` | Command sequences for attach, erase, program, read, reset |
| `src/chips.rs` | Chip family IDs and the ChipID-to-part-name database |
| `src/flash_op.rs` | Per-family flash algorithm blobs sent over the data endpoint |
| `src/dmi.rs`, `src/regs.rs` | DMI operations and RISC-V debug register addresses |

## Building this site locally

The site is built with [mdBook](https://rust-lang.github.io/mdBook/):

```bash
cargo install mdbook --locked
mdbook build          # output in book/
mdbook serve          # live preview at http://localhost:3000
```

The rendered site is published from the `gh-pages` branch; see
`.github/workflows/docs.yml`. The intended public URL is
<https://ch32-rs.github.io/wlink/>.
