//! The link-configuration types a raw-mode VCI link needs: the CAN ids and the raw-mode subset of
//! the ISO 22900-2 ComParam set (request/response id, CAN baud, frame options).

/// A CAN arbitration id, either 11-bit (standard) or 29-bit (extended).
///
/// On the wire in raw mode the id is always the high-to-low 4-byte big-endian prefix of the PDU
/// (ISO 22900-2:2009 §9.2.6.3.6); the 11-bit vs 29-bit distinction rides in the backend's transmit
/// flags, not in the byte count. So `0x7E0` encodes as `00 00 07 E0` whether standard or extended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CanId {
    raw: u32,
    extended: bool,
}

impl CanId {
    /// An 11-bit standard CAN id. The low 11 bits are kept.
    #[must_use]
    pub const fn standard(id: u16) -> Self {
        Self {
            raw: (id as u32) & 0x7FF,
            extended: false,
        }
    }

    /// A 29-bit extended CAN id. The low 29 bits are kept.
    #[must_use]
    pub const fn extended(id: u32) -> Self {
        Self {
            raw: id & 0x1FFF_FFFF,
            extended: true,
        }
    }

    /// The numeric id.
    #[must_use]
    pub const fn value(self) -> u32 {
        self.raw
    }

    /// Whether this is a 29-bit extended id.
    #[must_use]
    pub const fn is_extended(self) -> bool {
        self.extended
    }

    /// The 4-byte big-endian on-wire prefix (ISO 22900-2:2009 §9.2.6.3.6).
    #[must_use]
    pub const fn to_be_bytes(self) -> [u8; 4] {
        self.raw.to_be_bytes()
    }

    /// Rebuild a `CanId` from its 4-byte big-endian prefix. The id is read as extended when it does
    /// not fit in 11 bits, which is the only inference the 4 bytes allow; pass a response-id
    /// `extended` hint through [`LinkConfig`] when the exact width matters.
    #[must_use]
    pub const fn from_be_bytes(bytes: [u8; 4]) -> Self {
        let raw = u32::from_be_bytes(bytes);
        Self {
            raw,
            extended: raw > 0x7FF,
        }
    }
}

/// The transport protocol a link speaks. Raw mode over ISO 15765 (UDS / GMLAN over CAN) is all this
/// crate models today; other D-PDU protocols are future `ROADMAP` rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Protocol {
    /// ISO 15765-2 (UDS / GMLAN diagnostics over CAN) — a Volt diagnostic read.
    Iso15765,
}

/// The configuration for one raw-mode ComLogicalLink: the request / response CAN ids, the CAN baud,
/// and the frame options. This is the raw-mode subset of the ISO 22900-2 ComParam set; each backend
/// maps it onto its own config call (for J2534, the `PassThruConnect` baud/flags and the
/// flow-control `PassThruStartMsgFilter`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkConfig {
    /// The protocol the link speaks.
    pub protocol: Protocol,
    /// The physical request id (tester → ECU).
    pub request_id: CanId,
    /// The response id (ECU → tester).
    pub response_id: CanId,
    /// The CAN bit rate. 500 000 is HS-GMLAN / diagnostic CAN.
    pub baud: u32,
    /// Pad single frames to 8 bytes (GM ECUs expect this; J2534 `ISO15765_FRAME_PAD`).
    pub pad_frames: bool,
}

impl LinkConfig {
    /// The default diagnostic CAN bit rate (HS-GMLAN).
    pub const DEFAULT_BAUD: u32 = 500_000;

    /// An ISO 15765 link at the default baud, with frame padding on (the usual GM read setup).
    #[must_use]
    pub const fn iso15765(request_id: CanId, response_id: CanId) -> Self {
        Self {
            protocol: Protocol::Iso15765,
            request_id,
            response_id,
            baud: Self::DEFAULT_BAUD,
            pad_frames: true,
        }
    }

    /// Whether the link uses 29-bit ids (taken from the request id).
    #[must_use]
    pub const fn is_extended(self) -> bool {
        self.request_id.is_extended()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_id_encodes_big_endian_with_leading_zeros() {
        // The recovered native read puts 0x7E0 on the wire as 00 00 07 E0.
        assert_eq!(
            CanId::standard(0x7E0).to_be_bytes(),
            [0x00, 0x00, 0x07, 0xE0]
        );
    }

    #[test]
    fn standard_id_masks_to_eleven_bits() {
        assert_eq!(CanId::standard(0xFFFF).value(), 0x7FF);
        assert!(!CanId::standard(0x7E0).is_extended());
    }

    #[test]
    fn extended_id_round_trips_and_is_wide() {
        let id = CanId::extended(0x18DA_10F1);
        assert_eq!(id.value(), 0x18DA_10F1);
        assert!(id.is_extended());
        assert_eq!(CanId::from_be_bytes(id.to_be_bytes()).value(), 0x18DA_10F1);
    }

    #[test]
    fn from_be_bytes_infers_extended_above_eleven_bits() {
        assert!(!CanId::from_be_bytes([0x00, 0x00, 0x07, 0xE8]).is_extended());
        assert!(CanId::from_be_bytes([0x18, 0xDA, 0x10, 0xF1]).is_extended());
    }

    #[test]
    fn iso15765_config_defaults_to_diagnostic_baud_and_padding() {
        let cfg = LinkConfig::iso15765(CanId::standard(0x7E0), CanId::standard(0x7E8));
        assert_eq!(cfg.baud, 500_000);
        assert!(cfg.pad_frames);
        assert_eq!(cfg.protocol, Protocol::Iso15765);
    }
}
