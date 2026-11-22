# Instructions for coding agents

`vcikit` is a Rust crate (edition 2024, MSRV 1.85): a transport-agnostic model of a vehicle
communication interface in **raw mode** — the ComParam set, the ComLogicalLink lifecycle, and the
4-byte CAN-id-prefixed raw-mode PDU convention that sit between a diagnostic driver (`gmdiag`) and any
VCI backend (`vcxtoolkit` today, a SAE J2534 pass-through tomorrow). `PLAN.md` is the design, `ROADMAP.md`
the work queue, `docs/iso-map.md` the standards spine. This file takes precedence over `D:\code\AGENTS.md`
for work in this project. Read `docs/layout.md` before adding modules.

## Status: skeleton

This crate is a **registered, shelled skeleton**. The raw-mode PDU codec (4-byte CAN-id
prepend/strip), the ComParam catalog, the ComLogicalLink state machine and the `Backend` trait are
**not implemented yet**: implementation is blocked on `vcxtoolkit` growing a raw-mode channel bring-up,
and is triggered by a second backend (J2534 / RP1210) — see `PLAN.md`, "Trigger for code". Do not add
the backend or the id-prefix logic ahead of that trigger; a trait with one impl and no second caller is
the premature shell `PORTFOLIO.md` §7 warns against. Keep the crate minimal and compiling.

## Commands

Run these from this directory; all four must pass before a change is done.

    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    cargo test
    cargo doc --no-deps

`python -m projecthub shells verify --rust D:/code/vcikit` runs them in one pass.

## Invariants

- **Protocol wording comes from `primarysource`, never memory.** The raw-mode contract is ISO
  22900-2:2009 (§9.2.6.3.6 request, §11.1.4.18.5 response, Annex A J2534-1 mapping); confirm every
  citation with the `iso_*` tools and record it in `docs/iso-map.md`. SAE J2534-1 is behind the owner's
  terms gate, so ISO 22900-2 is the stand-in — do not cite J2534 from memory.
- `unsafe` is forbidden by the lint table in `Cargo.toml`; do not relax it to make something compile.
- Never run `cargo fmt` or `cargo clippy --fix` outside this directory.
- Do not raise `rust-version` without updating `clippy.toml`, `docs/msrv.md` and CI together.
- Do not add a dependency without saying why in the commit message.
- Write scratch output outside the repository; `git add -A` will commit it.
- A test that cannot fail is not a test: break the code once, watch it fail, revert.
- Commit messages carry no AI attribution trailer.
