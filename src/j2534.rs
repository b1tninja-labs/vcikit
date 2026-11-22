//! The platform-independent SAE J2534 message model the J2534 backend marshals into a loaded
//! pass-through DLL.
//!
//! The DLL load and the C-ABI calls are the `vcikit-j2534` member's job (isolated `unsafe`);
//! everything here is pure byte layout and constants, so it compiles and is tested on any platform.
//! [`PassThruMsg`] is `#[repr(C)]` and is the exact struct the C API takes by pointer.
//!
//! Every constant and the `PASSTHRU_MSG` layout are **verified against SAE J2534 (2002)** via
//! `primarysource` (`sae:j2534-2002`); the clause ref is on each item and the full map is in
//! `docs/j2534.md`. The SAE source is a scanned PDF with OCR errors, so each value is confirmed by its
//! figure and cross-checked against the recovered native pass-through `VCXPT32.dll` (session notes).
//! ISO 22900-2:2009 independently confirms the raw-mode CAN-id convention and maps D-PDU ↔ J2534-1
//! one-to-one (Annex A, Table A.1).

use crate::link::LinkConfig;

/// SAE J2534-1 numeric constants used by the ISO 15765 path.
///
/// Every value is verified against SAE J2534 (2002) via `primarysource` `sae:j2534-2002` (clause refs
/// below); none is from memory. The OCR of the scanned PDF is imperfect, so each value is confirmed by
/// its figure and cross-checked against the recovered native pass-through `VCXPT32.dll`. See
/// `docs/j2534.md`.
pub mod consts {
    /// `STATUS_NOERROR`: the call succeeded (`sae:j2534-2002§9.2.2.1`, Figure 47 — error codes).
    pub const STATUS_NOERROR: u32 = 0x00;
    /// `ERR_BUFFER_EMPTY`: a read returned with nothing queued (benign; stop or retry)
    /// (`sae:j2534-2002§9.2.2.1`, Figure 47).
    pub const ERR_BUFFER_EMPTY: u32 = 0x10;

    /// `ISO15765`: ISO 15765-2 (UDS / GMLAN over CAN) — the diagnostic read protocol id
    /// (`sae:j2534-2002§7.2.1.1`, Figure 4 — Protocol ID Values).
    pub const PROTO_ISO15765: u32 = 6;
    /// `CAN`: raw CAN, for reference (`sae:j2534-2002§7.2.1.1`, Figure 4).
    pub const PROTO_CAN: u32 = 5;

    /// `ISO15765_FRAME_PAD`: pad flow-controlled messages to a full CAN frame (GM ECUs expect this).
    /// `TxFlags` bit 6 (`sae:j2534-2002§8.4`, Figure 43 — TxFlags).
    pub const ISO15765_FRAME_PAD: u32 = 0x40;
    /// `CAN_29BIT_ID`: the ids are 29-bit extended. `TxFlags` / connect-flags bit 8
    /// (`sae:j2534-2002§8.4`, Figure 43; `§7.2.1.1`, Figure 3 — connect Flags).
    pub const CAN_29BIT_ID: u32 = 0x0100;

    /// `FLOW_CONTROL_FILTER`: the filter type ISO 15765 needs to bind a response id to a request id
    /// (`sae:j2534-2002§7.2.1.1`, Figure 11 — Filter Type Values).
    pub const FLOW_CONTROL_FILTER: u32 = 3;

    /// `RxStatus` bit 0 (`TX_MSG_TYPE`): 1 = transmitted, so this is the transmit loopback / `TxDone`
    /// echo of our own request, not an ECU reply, and is skipped (`sae:j2534-2002§8.4`, Figure 42 —
    /// RxStatus; a transmission's completion is confirmed through this bit, `§9.2.2.1` Appendix A).
    pub const RX_STATUS_TX_MSG: u32 = 0x01;

    /// `RxStatus` bit 1 (`ISO15765_FIRST_FRAME`): a zero-length indication that a multi-frame message
    /// has begun; it carries no application data, so it is skipped and the reassembled frame is read
    /// next (`sae:j2534-2002§8.4`, Figure 42; `§7.2.3` — `PassThruReadMsgs` returns FirstFrame and
    /// TxDone indications as messages; `§9.2.2.1` Appendix A/B).
    pub const RX_STATUS_ISO15765_FIRST_FRAME: u32 = 0x02;

    /// The human-readable name of a J2534-1 status code, or `None` if it is not one we name
    /// (`sae:j2534-2002§9.2.2.1`, Figure 47).
    #[must_use]
    pub const fn status_name(code: u32) -> Option<&'static str> {
        Some(match code {
            0x00 => "NOERROR",
            0x01 => "ERR_NOT_SUPPORTED",
            0x02 => "ERR_INVALID_CHANNEL_ID",
            0x03 => "ERR_INVALID_PROTOCOL_ID",
            0x04 => "ERR_NULL_PARAMETER",
            0x08 => "ERR_INVALID_FLAGS",
            0x09 => "ERR_FAILED",
            0x0A => "ERR_DEVICE_NOT_CONNECTED",
            0x0B => "ERR_TIMEOUT",
            0x0C => "ERR_INVALID_MSG",
            0x10 => "ERR_BUFFER_EMPTY",
            0x11 => "ERR_BUFFER_FULL",
            0x12 => "ERR_BUFFER_OVERFLOW",
            _ => return None,
        })
    }
}

/// The maximum `PASSTHRU_MSG.Data` length (`sae:j2534-2002§8.1`, `Data[4128]`).
pub const MAX_DATA: usize = 4128;

/// The SAE J2534-1 `PASSTHRU_MSG` (`sae:j2534-2002§8.1`), `#[repr(C)]` so it is the exact struct the
/// pass-through DLL takes by pointer. The backend fills `data[..data_size]`; for ISO 15765 those bytes
/// are the 4-byte CAN id followed by the application PDU (`sae:j2534-2002§8.3.2` — ISO 15765-4 data
/// format; `§8.3.1` — CAN data format).
#[repr(C)]
#[derive(Clone)]
pub struct PassThruMsg {
    /// The protocol id (`consts::PROTO_ISO15765`).
    pub protocol_id: u32,
    /// Receive status flags (`consts::RX_STATUS_*`); 0 on transmit.
    pub rx_status: u32,
    /// Transmit flags (`consts::ISO15765_FRAME_PAD`, `consts::CAN_29BIT_ID`).
    pub tx_flags: u32,
    /// Device timestamp in microseconds (set by the device on receive).
    pub timestamp: u32,
    /// The number of valid bytes in `data`.
    pub data_size: u32,
    /// The index of the first byte after any extra data (unused on this path; 0).
    pub extra_data_index: u32,
    /// The message bytes; only the first `data_size` are valid.
    pub data: [u8; MAX_DATA],
}

impl Default for PassThruMsg {
    fn default() -> Self {
        Self {
            protocol_id: consts::PROTO_ISO15765,
            rx_status: 0,
            tx_flags: 0,
            timestamp: 0,
            data_size: 0,
            extra_data_index: 0,
            data: [0; MAX_DATA],
        }
    }
}

impl PassThruMsg {
    /// An ISO 15765 message carrying `payload` with the given transmit flags. `payload` longer than
    /// [`MAX_DATA`] is truncated (it never is on this path — a diagnostic PDU is tiny).
    #[must_use]
    pub fn iso15765(payload: &[u8], tx_flags: u32) -> Self {
        let n = payload.len().min(MAX_DATA);
        let mut msg = Self {
            tx_flags,
            ..Self::default()
        };
        msg.data[..n].copy_from_slice(&payload[..n]);
        msg.data_size = u32::try_from(n).unwrap_or(0);
        msg
    }

    /// The valid message bytes (`data[..data_size]`).
    #[must_use]
    pub fn payload(&self) -> &[u8] {
        let n = (self.data_size as usize).min(MAX_DATA);
        &self.data[..n]
    }

    /// Whether this received message is the transmit loopback echo of our own request
    /// (`RxStatus` `TX_MSG_TYPE` bit; `sae:j2534-2002§8.4`, Figure 42).
    #[must_use]
    pub const fn is_tx_echo(&self) -> bool {
        self.rx_status & consts::RX_STATUS_TX_MSG != 0
    }

    /// Whether this received message is the ISO 15765 FirstFrame indication — a zero-length marker
    /// that a multi-frame reply has begun, carrying no application data. `PassThruReadMsgs` returns it
    /// as a message (`sae:j2534-2002§7.2.3`), so a reader must skip it and take the reassembled frame
    /// that follows (`RxStatus` `ISO15765_FIRST_FRAME` bit; `sae:j2534-2002§8.4`, Figure 42).
    #[must_use]
    pub const fn is_first_frame_indication(&self) -> bool {
        self.rx_status & consts::RX_STATUS_ISO15765_FIRST_FRAME != 0
    }
}

/// The transmit flags for a link: frame padding when enabled, plus the 29-bit flag for extended ids.
#[must_use]
pub fn tx_flags(config: &LinkConfig) -> u32 {
    let mut flags = 0;
    if config.pad_frames {
        flags |= consts::ISO15765_FRAME_PAD;
    }
    if config.is_extended() {
        flags |= consts::CAN_29BIT_ID;
    }
    flags
}

/// The connect flags for a link (the 29-bit flag when the ids are extended).
#[must_use]
pub fn connect_flags(config: &LinkConfig) -> u32 {
    if config.is_extended() {
        consts::CAN_29BIT_ID
    } else {
        0
    }
}

/// The three messages an ISO 15765 flow-control filter needs: `[mask, pattern, flow]`.
///
/// The mask is `FF FF FF FF`, the pattern is the response id and the flow is the request id, each as
/// a 4-byte big-endian CAN id — the binding that routes the ECU's reply back to our request
/// (recovered from the native pass-through; `docs/j2534.md`).
#[must_use]
pub fn flow_control_filter(config: &LinkConfig) -> [PassThruMsg; 3] {
    let mask = PassThruMsg::iso15765(&[0xFF, 0xFF, 0xFF, 0xFF], 0);
    let pattern = PassThruMsg::iso15765(&config.response_id.to_be_bytes(), 0);
    let flow = PassThruMsg::iso15765(&config.request_id.to_be_bytes(), 0);
    [mask, pattern, flow]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::link::CanId;

    fn cfg() -> LinkConfig {
        LinkConfig::iso15765(CanId::standard(0x7E0), CanId::standard(0x7E8))
    }

    #[test]
    fn iso15765_message_carries_the_payload_and_protocol() {
        let msg = PassThruMsg::iso15765(
            &[0x00, 0x00, 0x07, 0xE0, 0x1A, 0xC1],
            consts::ISO15765_FRAME_PAD,
        );
        assert_eq!(msg.protocol_id, consts::PROTO_ISO15765);
        assert_eq!(msg.data_size, 6);
        assert_eq!(msg.payload(), &[0x00, 0x00, 0x07, 0xE0, 0x1A, 0xC1]);
        assert_eq!(msg.tx_flags, consts::ISO15765_FRAME_PAD);
    }

    #[test]
    fn tx_flags_pad_and_extended() {
        assert_eq!(tx_flags(&cfg()), consts::ISO15765_FRAME_PAD);
        let ext = LinkConfig::iso15765(CanId::extended(0x18DA_10F1), CanId::extended(0x18DA_F110));
        assert_eq!(
            tx_flags(&ext),
            consts::ISO15765_FRAME_PAD | consts::CAN_29BIT_ID
        );
        assert_eq!(connect_flags(&ext), consts::CAN_29BIT_ID);
        assert_eq!(connect_flags(&cfg()), 0);
    }

    #[test]
    fn flow_control_filter_is_mask_pattern_flow() {
        let [mask, pattern, flow] = flow_control_filter(&cfg());
        assert_eq!(mask.payload(), &[0xFF, 0xFF, 0xFF, 0xFF]);
        assert_eq!(pattern.payload(), &[0x00, 0x00, 0x07, 0xE8]); // response id
        assert_eq!(flow.payload(), &[0x00, 0x00, 0x07, 0xE0]); // request id
    }

    #[test]
    fn tx_echo_is_detected_from_rx_status() {
        let mut msg = PassThruMsg::default();
        assert!(!msg.is_tx_echo());
        msg.rx_status = consts::RX_STATUS_TX_MSG;
        assert!(msg.is_tx_echo());
    }

    #[test]
    fn first_frame_indication_is_detected_and_distinct_from_the_echo() {
        let mut msg = PassThruMsg::default();
        assert!(!msg.is_first_frame_indication());
        msg.rx_status = consts::RX_STATUS_ISO15765_FIRST_FRAME;
        assert!(msg.is_first_frame_indication());
        assert!(!msg.is_tx_echo()); // the FirstFrame indication is not the transmit echo
    }

    #[test]
    fn status_names_cover_the_common_codes() {
        assert_eq!(consts::status_name(0x00), Some("NOERROR"));
        assert_eq!(consts::status_name(0x10), Some("ERR_BUFFER_EMPTY"));
        assert_eq!(consts::status_name(0xFF), None);
    }
}
