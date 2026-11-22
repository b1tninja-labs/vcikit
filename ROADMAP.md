# vcikit roadmap

`vcikit` is a transport-agnostic model of a vehicle communication interface in **raw mode**: the
ComParam set that configures a ComLogicalLink, the connect → configure → start → send/recv → stop
lifecycle, and the raw-mode request/response PDU convention (the 4-byte CAN-id prefix of ISO
22900-2:2009 §9.2.6.3.6). It is the seam between a diagnostic driver (`gmdiag`) and any VCI backend
(`vcxtoolkit` today, a SAE J2534 pass-through tomorrow). It is not a dongle driver, not ISO-TP, not
UDS, and not GM-specific. See `PLAN.md` for the design and `docs/iso-map.md` for the standards spine.

Rows are `Status | Obligation | Anchor | Test`, the grammar `projecthub sync` reads for Rust
projects. This repository is registered as tooling, public at <https://github.com/b1tninja-labs/vcikit>,
with `publish = false`.

Status:

- `[ok]` the named test exists and passed on 2026-11-22.
- `[open]` not built — **blocked** on `vcxtoolkit` growing a raw-mode channel bring-up so there is one
  real backend to abstract over; triggered by a second backend (J2534 / RP1210). This is a deliberate
  skeleton, not a stalled one (`PLAN.md`, "Trigger for code").
- `[no]` a deliberate non-goal until the named document is imported.

## The crate (skeleton)

| Status | Obligation | Anchor | Test |
|---|---|---|---|
| [ok] | Expose the crate identity so the four gates have something to compile, run and document | `src/lib.rs` | `name_is_the_package_name`; `tests/smoke.rs::public_api_names_the_crate` |

## The raw-mode contract (blocked on vcxtoolkit's raw-mode bring-up)

| Status | Obligation | Anchor | Test |
|---|---|---|---|
| [open] | Prepend / strip the 4-byte CAN id (11- or 29-bit, big-endian) and the optional extended-address byte on a raw-mode request PDU | `iso:22900-2@2009§9.2.6.3.6`; `docs/iso-map.md` | none |
| [open] | Parse a raw-mode response PDU: take the first 4 bytes as the CAN id and the payload after it | `iso:22900-2@2009§11.1.4.18.5`; `docs/iso-map.md` | none |
| [open] | A typed ComParam set (physical/functional request id, response id, CAN baud, P2/P2\* and STmin timing, flow-control block size / separation time) | `iso:22900-2@2009§11.1.4`; `docs/iso-map.md` | none |
| [open] | A ComLogicalLink lifecycle state machine: connect → set ComParams → start → send/recv → stop | `iso:22900-2@2009§9` | none |
| [open] | A `Backend` trait — the one seam each interface implements (open, apply ComParams, start, exchange one raw-mode PDU, stop); `vcxtoolkit`'s channel is the first impl | `PLAN.md` | none |

## Non-goals

| Status | Obligation | Anchor | Test |
|---|---|---|---|
| [no] | The VCX serial frame codec, `VCX_PRO_*` commands, licensing, firmware — all `vcxtoolkit` | `PLAN.md` | none |
| [no] | UDS / GMLAN service PDUs and negative-response decode — all `udskit` | `PLAN.md` | none |
| [no] | ISO-TP segmentation — the dongle firmware does it in raw mode; a raw-CAN backend uses `isotpkit` | `PLAN.md` | none |
| [no] | Non-raw / ODX ComParam databases, the D-Server API (ISO 22900-3), the MVCI object model beyond the raw-mode subset | `PLAN.md` | none |
| [no] | Cite SAE J2534-1 directly: the `sae_*` collection is behind the owner's terms gate on an unmerged branch, so ISO 22900-2 is the stand-in | `docs/iso-map.md` | none |
