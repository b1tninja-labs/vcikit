//! The backend seam: the one trait every vehicle communication interface implements, and the
//! enumeration `gmdiag` uses to pick one.

use std::time::Duration;

use crate::error::Result;
use crate::link::LinkConfig;

/// A raw-mode VCI link — the seam between a diagnostic driver (`gmdiag`) and the hardware that
/// reaches the bus.
///
/// A backend opens the interface, applies the [`LinkConfig`], brings the ComLogicalLink up
/// (connect → configure → start → flow-control filter), then carries whole **application** PDUs:
/// [`send`](RawBackend::send) prepends the request CAN id and [`recv`](RawBackend::recv) strips the
/// response id, so the caller never sees the raw-mode 4-byte prefix. [`close`](RawBackend::close)
/// tears the link down.
///
/// This is a dumb transport. It originates no traffic of its own and knows nothing about UDS /
/// GMLAN service semantics (those are `udskit`'s, and the diagnostic layer's); the read-first /
/// gated ethos — `$3E` and `$1A` only, never `$27`, no writes beyond an explicit gate — lives in
/// the caller that decides which PDUs to hand this trait, not here.
///
/// `vcikit-j2534`'s `J2534Backend` is one implementation; a native-VCX backend over `vcxtoolkit`'s
/// codecs is the planned second (see [`BackendKind`] and `PLAN.md`).
pub trait RawBackend {
    /// Open the interface and bring the link up for `config`.
    ///
    /// # Errors
    ///
    /// Returns a [`crate::Error`] if the device cannot be opened, connected, or configured.
    fn connect(&mut self, config: &LinkConfig) -> Result<()>;

    /// Send one application PDU. The backend prepends the configured request CAN id.
    ///
    /// # Errors
    ///
    /// Returns a [`crate::Error`] if the link is not connected or the write fails.
    fn send(&mut self, app_pdu: &[u8]) -> Result<()>;

    /// Receive one application response PDU (the backend strips the CAN id).
    ///
    /// # Errors
    ///
    /// Returns [`Error::Timeout`](crate::error::Error::Timeout) when nothing arrives in `timeout`,
    /// or another [`crate::Error`] if the read fails or the response is malformed.
    fn recv(&mut self, timeout: Duration) -> Result<Vec<u8>>;

    /// Tear the link down (disconnect and close the device).
    ///
    /// # Errors
    ///
    /// Returns a [`crate::Error`] if teardown reports a failure.
    fn close(&mut self) -> Result<()>;
}

/// Which VCI backend to drive. `gmdiag` selects one; both are consumed through [`RawBackend`], so
/// the driver holds a `Box<dyn RawBackend>` and does not care which is behind it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BackendKind {
    /// A registered SAE J2534 pass-through DLL — VXDIAG's today, a pass-through hosted by
    /// `vcxtoolkit` tomorrow. vcikit consumes whatever is registered. Implemented in the
    /// `vcikit-j2534` member.
    J2534,
    /// The native VCX protocol via `vcxtoolkit`'s `VCX_PRO_Open` / `Send` / `Recv` codecs. Not
    /// implemented yet — blocked on `vcxtoolkit` growing the raw-mode channel bring-up
    /// (`SetComParam` / `SetFilter` / `Start` plus a flow-control filter); see `PLAN.md`.
    NativeVcx,
}

impl BackendKind {
    /// A short, human-readable name for the backend.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::J2534 => "j2534",
            Self::NativeVcx => "native-vcx",
        }
    }

    /// Whether a built implementation of this backend exists in the workspace today.
    #[must_use]
    pub const fn is_implemented(self) -> bool {
        matches!(self, Self::J2534)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_j2534_backend_is_implemented_so_far() {
        assert!(BackendKind::J2534.is_implemented());
        assert!(!BackendKind::NativeVcx.is_implemented());
        assert_eq!(BackendKind::J2534.as_str(), "j2534");
        assert_eq!(BackendKind::NativeVcx.as_str(), "native-vcx");
    }
}
