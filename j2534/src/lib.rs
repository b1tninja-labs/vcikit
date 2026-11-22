//! `vcikit-j2534` — a SAE J2534-1 pass-through backend for [`vcikit`].
//!
//! This member implements [`vcikit::backend::RawBackend`] over a registered J2534 v04.04
//! pass-through DLL, for ISO 15765 (UDS / GMLAN) diagnostic reads. It [discovers](discovery::discover)
//! the DLL under `HKLM\SOFTWARE\PassThruSupport.04.04` (and the `WOW6432Node` view), loads it, and
//! drives the J2534-1 ISO 15765 API (`PassThruOpen` → `Connect` → flow-control `StartMsgFilter` →
//! `WriteMsgs` / `ReadMsgs`). The DLL may be VXDIAG's today or a pass-through hosted by `vcxtoolkit`
//! tomorrow; this backend consumes whatever is registered.
//!
//! It lives in its own crate, not in `vcikit`, because loading a DLL and calling a C ABI is `unsafe`
//! FFI: `vcikit` keeps `unsafe` forbidden, and here the `unsafe` is denied everywhere and allowed
//! only on the audited [`ffi`] module. The registry read ([`discovery`]) is Windows-only and uses
//! no `unsafe`; off Windows it is empty and [`J2534Backend::open_registered`] returns
//! [`vcikit::Error::NoDevice`].
//!
//! SAE J2534-1 is not in `primarysource` (it is behind the owner's terms gate); the API is grounded
//! in public J2534-1 v04.04 knowledge, cross-checked against the recovered native pass-through
//! `VCXPT32.dll`, and the raw-mode CAN-id convention is confirmed by ISO 22900-2:2009. See
//! `vcikit`'s `docs/j2534.md`.

pub mod backend;
pub mod discovery;
pub mod ffi;

pub use backend::J2534Backend;
pub use discovery::{PassThruDevice, discover};

/// The name of this crate, as Cargo knows it.
#[must_use]
pub const fn name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

#[cfg(test)]
mod tests {
    #[test]
    fn name_is_the_package_name() {
        assert_eq!(super::name(), "vcikit-j2534");
    }
}
