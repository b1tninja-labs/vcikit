# vcikit

A transport-agnostic MVCI / D-PDU raw-mode (PassThru) layer: the 4-byte CAN-id-prefixed
request/response convention, the link config (the raw-mode ComParam subset), and the backend seam
between a diagnostic driver (`gmdiag`) and any vehicle communication interface. It is not a dongle
driver (`vcxtoolkit`), not ISO-TP (`isotpkit`), not UDS (`udskit`), and not GM-specific (`gmdiag`); it
carries whole application PDUs and owns no service semantics.

Two backends sit behind one `RawBackend` trait, and `gmdiag` selects:

- **J2534** (built, `vcikit-j2534` member): discovers a registered SAE J2534 pass-through DLL under
  `HKLM\SOFTWARE\PassThruSupport.04.04`, loads it, and drives the J2534-1 ISO 15765 API. The DLL may
  be VXDIAG's today or a pass-through hosted by `vcxtoolkit` tomorrow — vcikit consumes whatever is
  registered.
- **Native-VCX** (blocked): the native VCX protocol via `vcxtoolkit`'s codecs. The selector is in
  place; the implementation waits on `vcxtoolkit`'s raw-mode channel bring-up (`PLAN.md`).

The crate keeps `unsafe` forbidden; the DLL load and C-ABI calls are isolated to one audited module in
the `vcikit-j2534` member. The standards spine is ISO 22900-2:2009 (`docs/iso-map.md`); SAE J2534-1 is
behind the owner's terms gate, so the J2534 constants are grounded in public knowledge and the
recovered native DLL (`docs/j2534.md`).

## Develop

    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    cargo test
    cargo doc --no-deps
    cargo clippy -p vcikit-j2534 --all-targets -- -D warnings   # the J2534 backend member
    cargo test -p vcikit-j2534

Minimum supported Rust version: 1.85 (see `docs/msrv.md`). Contributing: `CONTRIBUTING.md`.
