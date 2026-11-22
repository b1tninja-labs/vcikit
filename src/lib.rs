//! `vcikit` — a transport-agnostic model of a vehicle communication interface in **raw mode**.
//!
//! This is the seam between a diagnostic driver (`gmdiag`) and whatever hardware actually reaches
//! the bus (`vcxtoolkit`'s `VCX-NANO` today; a SAE J2534 pass-through or `RP1210a` interface
//! tomorrow). It owns exactly the part of the stack that is a published standard and is the same
//! across every interface: the `ComParam` set that configures a `ComLogicalLink`, the connect →
//! configure → start → send/recv → stop lifecycle, and the raw-mode request/response PDU convention
//! — the 4-byte CAN-id prefix of ISO 22900-2:2009 §9.2.6.3.6 (raw mode / `PassThru`).
//!
//! It is **not** a dongle driver (`vcxtoolkit`), **not** ISO-TP (`isotpkit`), **not** UDS
//! (`udskit`), and **not** GM-specific (`gmdiag`).
//!
//! # Status: skeleton
//!
//! This crate is a registered, shelled skeleton only. The raw-mode PDU codec (4-byte CAN-id
//! prepend/strip), the `ComParam` catalog, the `ComLogicalLink` state machine, and the `Backend`
//! trait are **not implemented yet**: implementation is blocked on `vcxtoolkit` growing a raw-mode
//! channel bring-up (`SetComParam` / `SetFilter` / `Start` plus a flow-control filter) so there is
//! one real backend to abstract over, and is triggered by a second backend (J2534 / `RP1210`)
//! putting the id-prefix and `ComParam` logic at risk of being duplicated. Until then this crate
//! exposes only its identity, so the gates have something to compile and run. See `PLAN.md` for the
//! design and `docs/iso-map.md` for the standards spine.
//!
//! ```
//! assert_eq!(vcikit::name(), "vcikit");
//! ```

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
