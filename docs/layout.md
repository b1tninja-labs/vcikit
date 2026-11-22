# Workspace layout

A Cargo workspace of two crates, split so the abstraction keeps `unsafe` forbidden while the DLL FFI
is isolated.

- **`vcikit`** (root crate, `Cargo.toml` + `src/` + `tests/`): the abstraction. `unsafe` is
  **forbidden**. Modules:
  - `link` — `CanId` (11/29-bit) and `LinkConfig` (the raw-mode ComParam subset).
  - `raw` — the 4-byte CAN-id raw-mode request/response codec (ISO 22900-2 §9.2.6.3.6 / §11.1.4.18.5).
  - `backend` — the `RawBackend` trait and the `BackendKind` selector.
  - `j2534` — the platform-independent SAE J2534-1 message model (`PASSTHRU_MSG`, constants,
    flow-control filter, tx flags) the J2534 backend marshals. Pure, no FFI.
  - `error` — the shared `Error` / `Result`.
- **`j2534/`** (the `vcikit-j2534` member): the J2534 pass-through backend. `unsafe` is **denied** and
  allowed only on `ffi.rs`. Modules:
  - `discovery` — read the registered `PassThruSupport.04.04` DLLs (Windows-only; empty elsewhere).
  - `ffi` — load the DLL and call the J2534-1 C ABI (`#![allow(unsafe_code)]`, every block with a
    `// SAFETY:` note).
  - `backend` — `J2534Backend`, the `vcikit::RawBackend` impl.

Conventions:

- Keep `lib.rs` to module declarations and re-exports; unit tests sit beside the code in
  `#[cfg(test)]` modules, integration tests in `tests/` use only the public API.
- Lints are configured in each `Cargo.toml` (`[lints]`), not with scattered `#![...]` attributes —
  the single exception is the audited `#![allow(unsafe_code)]` at the top of `j2534/src/ffi.rs`.
- The default gates (no `--workspace`) build only the root crate; the member has its own `-p
  vcikit-j2534` gate job in CI.
