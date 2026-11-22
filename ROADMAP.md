# vcikit roadmap

`vcikit` is a transport-agnostic model of a vehicle communication interface in **raw mode**: the
link configuration (the raw-mode subset of the ISO 22900-2 ComParam set), the raw-mode
request/response PDU convention (the 4-byte CAN-id prefix of ISO 22900-2:2009 §9.2.6.3.6), and the
backend seam a diagnostic driver (`gmdiag`) drives. Two backends sit behind one trait: a **J2534**
pass-through backend (built) and a **native-VCX** backend over `vcxtoolkit` (blocked). It is not a
dongle driver, not ISO-TP, not UDS, and not GM-specific. See `PLAN.md`, `docs/iso-map.md` and
`docs/j2534.md`.

Rows are `Status | Obligation | Anchor | Test`, the grammar `projecthub sync` reads for Rust
projects. This repository is registered as tooling, public at <https://github.com/b1tninja-labs/vcikit>,
with `publish = false`. The crate is a workspace: `vcikit` (the abstraction, `unsafe` forbidden) and
the `vcikit-j2534` member (the J2534 FFI backend, `unsafe` isolated to one module).

Status:

- `[ok]` the named test exists and passed on 2026-11-22.
- `[open]` not built; **blocked** on `vcxtoolkit`'s raw-mode channel bring-up (`PLAN.md`).
- `[no]` a deliberate non-goal until the named document is imported.

## The raw-mode contract (abstraction crate)

| Status | Obligation | Anchor | Test |
|---|---|---|---|
| [ok] | Prepend the 4-byte big-endian CAN id to a request PDU (`00 00 07 E0 1A C1` for `$1A C1` to 0x7E0) | `iso:22900-2@2009§9.2.6.3.6`; `src/raw.rs` | `raw::request_round_trips_through_the_response_decoder`; `raw::empty_application_pdu_is_just_the_id` |
| [ok] | Parse a response PDU: take the first 4 bytes as the CAN id, the rest as the application PDU; reject a short response | `iso:22900-2@2009§11.1.4.18.5`; `src/raw.rs` | `raw::a_response_shorter_than_the_id_is_rejected` |
| [ok] | Encode a CAN id (11- or 29-bit) as the 4-byte big-endian on-wire prefix | `iso:22900-2@2009§9.2.6.3.6`; `src/link.rs` | `link::standard_id_encodes_big_endian_with_leading_zeros`; `link::extended_id_round_trips_and_is_wide` |
| [ok] | A typed link config (request/response id, CAN baud, frame padding) — the raw-mode ComParam subset | `iso:22900-2@2009§11.1.4`; `src/link.rs` | `link::iso15765_config_defaults_to_diagnostic_baud_and_padding` |
| [ok] | The `RawBackend` seam (connect → send → recv → close) and the `BackendKind` selector | `src/backend.rs` | `backend::only_the_j2534_backend_is_implemented_so_far` |
| [open] | Extended-addressing byte after the CAN id, and non-ISO-15765 D-PDU protocols | `iso:22900-2@2009§9.2.6.3.6` | none |

## The J2534 pass-through backend (`vcikit-j2534` member)

| Status | Obligation | Anchor | Test |
|---|---|---|---|
| [ok] | The J2534-1 v04.04 message model (`PASSTHRU_MSG`, protocol/flag/status constants, tx flags) | `docs/j2534.md`; `src/j2534.rs` | `j2534::iso15765_message_carries_the_payload_and_protocol`; `j2534::tx_flags_pad_and_extended`; `j2534::status_names_cover_the_common_codes` |
| [ok] | The ISO 15765 flow-control filter triple (mask `FF FF FF FF`, pattern = response id, flow = request id) | `docs/j2534.md`; `src/j2534.rs` | `j2534::flow_control_filter_is_mask_pattern_flow` |
| [ok] | Discover registered pass-through DLLs under `PassThruSupport.04.04` (and `WOW6432Node`); read `FunctionLibrary` and `ISO15765` | `j2534/src/discovery.rs` | `discovery::supports_iso15765_reads_the_capability_value`; `discovery::has_library_requires_a_non_empty_path`; `discovery::discovery_is_empty_off_windows` |
| [ok] | Load a pass-through DLL and bind the J2534-1 entry points; a bad path is a clear `DllLoad` error | `j2534/src/ffi.rs` | `backend::loading_a_non_dll_path_is_a_dll_load_error` |
| [ok] | `open_registered` returns `NoDevice` when nothing is registered (every non-Windows platform) | `j2534/src/backend.rs` | `backend::open_registered_reports_no_device_off_windows` |
| [ok] | Isolate the unsafe FFI to one audited module; the abstraction crate keeps `unsafe` forbidden | `j2534/src/ffi.rs` (`#![allow(unsafe_code)]`); `Cargo.toml` lint tables | compiler (lint tables) |
| [open] | A device-backed end-to-end read on Windows against the registered DLL (owner's vehicle; see `vcx_j2534_capture.py` session note) | `j2534/src/backend.rs` | none — needs hardware |

## The native-VCX backend

| Status | Obligation | Anchor | Test |
|---|---|---|---|
| [ok] | Expose `BackendKind::NativeVcx` through the same `RawBackend` seam, marked not-yet-implemented | `src/backend.rs` | `backend::only_the_j2534_backend_is_implemented_so_far` |
| [open] | Implement the native-VCX backend over `vcxtoolkit`'s `VCX_PRO_Open`/`Send`/`Recv` codecs | `PLAN.md`; `vcxtoolkit` | none — **blocked** on `vcxtoolkit` growing the raw-mode channel bring-up (`SetComParam`/`SetFilter`/`Start` + flow-control filter) |

## Non-goals

| Status | Obligation | Anchor | Test |
|---|---|---|---|
| [no] | Cite SAE J2534-1 clause by clause: the `sae_*` collection is behind the owner's terms gate, so the API is grounded in public J2534-1 knowledge + the recovered native DLL + ISO 22900-2 | `docs/j2534.md` | none |
| [no] | UDS / GMLAN service semantics and the read-first gate ($3E/$1A only, never $27) — the transport carries caller PDUs; the gate is `gmdiag`'s | `src/backend.rs` (`RawBackend` doc) | none |
| [no] | ISO-TP segmentation (`isotpkit`), the VCX serial frame codec (`vcxtoolkit`), ODX / non-raw ComParam databases | `PLAN.md` | none |
