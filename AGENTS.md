# Instructions for coding agents

`vcikit` is a Rust **workspace** (edition 2024, MSRV 1.85): a transport-agnostic model of a vehicle
communication interface in **raw mode** — the link config (the raw-mode ComParam subset), the 4-byte
CAN-id-prefixed raw-mode PDU convention, and the backend seam that sit between a diagnostic driver
(`gmdiag`) and any VCI backend. `PLAN.md` is the design, `ROADMAP.md` the work queue, `docs/iso-map.md`
and `docs/j2534.md` the standards spine. This file takes precedence over `D:\code\AGENTS.md` for work
in this project. Read `docs/layout.md` before adding modules.

## Layout: two crates, unsafe isolated

- **`vcikit`** (root crate, `unsafe` **forbidden**): the `RawBackend` trait and `BackendKind`, the
  `LinkConfig` / `CanId` types, the raw-mode PDU codec (`raw`), and the platform-independent J2534
  message model (`j2534`). All of this is pure and tested on the Ubuntu CI.
- **`vcikit-j2534`** (the `j2534/` member, `unsafe` **deny** + `#![allow(unsafe_code)]` on `ffi.rs`
  only): the registry discovery and the J2534-1 DLL FFI. It is the one place `unsafe` lives, so the
  abstraction crate keeps `forbid`. The device paths are `cfg`-gated, so it compiles and tests on
  Ubuntu without a device or Windows.

Two backends sit behind `RawBackend`: **J2534** (built, in `vcikit-j2534`) and **native-VCX** (the
`BackendKind::NativeVcx` selector is in place; the impl over `vcxtoolkit`'s codecs is **blocked** on
`vcxtoolkit` growing the raw-mode channel bring-up — see `PLAN.md` and `ROADMAP.md`). Do not edit
`vcxtoolkit` from here, and do not build the native-VCX backend until that bring-up lands.

## Commands

Run these from this directory. The default gates build the root crate; the member has its own `-p`.

    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    cargo test
    cargo doc --no-deps
    cargo clippy -p vcikit-j2534 --all-targets -- -D warnings
    cargo test -p vcikit-j2534
    cargo doc -p vcikit-j2534 --no-deps

`python -m projecthub shells verify --rust D:/code/vcikit` runs the root gates in one pass; CI runs the
member's `-p` gates in a separate job.

## Invariants

- **Protocol wording comes from `primarysource`, never memory.** The raw-mode contract is ISO
  22900-2:2009 (§9.2.6.3.6 request, §11.1.4.18.5 response, Annex A J2534-1 mapping); confirm every
  citation with the `iso_*` tools and record it in `docs/iso-map.md`. The J2534 constants are verified
  clause by clause against SAE J2534 (`sae:j2534-2002`, enabled 2026-11-22) with the `sae_*` tools and
  recorded in `docs/j2534.md`; that source is a scanned PDF with OCR errors, so confirm each value by
  its figure and cross-check against the recovered native DLL, and never quote its OCR prose verbatim.
  Do not cite either standard from memory.
- **`unsafe` is forbidden in `vcikit` and isolated in `vcikit-j2534`.** New FFI goes in `ffi.rs` under
  its audited `#![allow(unsafe_code)]`, each `unsafe` block with a `// SAFETY:` note; never relax the
  root crate's `forbid` and never widen the member's `deny` beyond that one module.
- vcikit is a dumb transport: it carries whole application PDUs and owns no UDS/GMLAN semantics. The
  read-first / gated ethos ($3E/$1A only, never $27) lives in the caller (`gmdiag`), not here.
- Never run `cargo fmt` or `cargo clippy --fix` outside this directory.
- Do not raise `rust-version` without updating `clippy.toml`, `docs/msrv.md` and CI together.
- Do not add a dependency without saying why in the commit message.
- Write scratch output outside the repository; `git add -A` will commit it.
- A test that cannot fail is not a test: break the code once, watch it fail, revert.
- Commit messages carry no AI attribution trailer.
