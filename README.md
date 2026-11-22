# vcikit

A transport-agnostic MVCI / D-PDU raw-mode (PassThru) layer: the 4-byte CAN-id-prefixed request/response convention and the ComParam plus ComLogicalLink lifecycle between a diagnostic driver and any vehicle communication interface backend (skeleton)

Status: scaffold. Nothing here is implemented beyond `name()`.

## Use

    cargo add vcikit    # after the first publish; the name is not yet reserved
    cargo run

## Develop

    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    cargo test
    cargo doc --no-deps

Minimum supported Rust version: 1.85 (see `docs/msrv.md`). Contributing: `CONTRIBUTING.md`.
