//! `vcikit` — a transport-agnostic model of a vehicle communication interface in **raw mode**.
//!
//! This is the seam between a diagnostic driver (`gmdiag`) and whatever hardware actually reaches
//! the bus. It owns the part of the stack that is a published standard and is the same across every
//! interface: the [`link`] configuration (the raw-mode subset of the ISO 22900-2 ComParam set), the
//! [`raw`]-mode request / response PDU convention (the 4-byte CAN-id prefix of ISO 22900-2:2009
//! §9.2.6.3.6), and the [`backend`] seam a driver drives.
//!
//! It is **not** a dongle driver (`vcxtoolkit`), **not** ISO-TP (`isotpkit`), **not** UDS
//! (`udskit`), and **not** GM-specific (`gmdiag`). It carries whole application PDUs and knows
//! nothing of their service semantics; the read-first / gated ethos lives in the caller.
//!
//! # Backends
//!
//! Two backends sit behind the one [`backend::RawBackend`] trait, and `gmdiag` selects
//! ([`backend::BackendKind`]):
//!
//! - **J2534** — a registered SAE J2534 pass-through DLL (VXDIAG's today, a pass-through hosted by
//!   `vcxtoolkit` tomorrow; vcikit consumes whatever is registered). It discovers the DLL under
//!   `HKLM\SOFTWARE\PassThruSupport.04.04` (and the `WOW6432Node` view), loads it, and drives the
//!   J2534-1 ISO 15765 API. Implemented in the **`vcikit-j2534`** member, which isolates the unsafe
//!   FFI so this crate keeps `unsafe` forbidden. The platform-independent J2534 message model it
//!   marshals is [`j2534`], here and tested on every platform.
//! - **Native-VCX** — the native VCX protocol via `vcxtoolkit`'s codecs. The trait and the
//!   [`backend::BackendKind::NativeVcx`] selector are in place; the implementation is **blocked** on
//!   `vcxtoolkit` growing the raw-mode channel bring-up, and is a `ROADMAP` row (see `PLAN.md`).
//!
//! ```
//! use vcikit::{link::{CanId, LinkConfig}, raw};
//! let cfg = LinkConfig::iso15765(CanId::standard(0x7E0), CanId::standard(0x7E8));
//! // A backend sends application PDUs; the raw-mode id prefix is handled for you.
//! assert_eq!(raw::encode_request(cfg.request_id, &[0x1A, 0xC1]), [0, 0, 7, 0xE0, 0x1A, 0xC1]);
//! ```

pub mod backend;
pub mod error;
pub mod j2534;
pub mod link;
pub mod raw;

pub use backend::{BackendKind, RawBackend};
pub use error::{Error, Result};
pub use link::{CanId, LinkConfig, Protocol};

/// The name of this crate, as Cargo knows it.
#[must_use]
pub const fn name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

#[cfg(test)]
mod tests {
    #[test]
    fn name_is_the_package_name() {
        assert_eq!(super::name(), "vcikit");
    }
}
