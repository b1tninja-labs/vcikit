//! The raw-mode request / response PDU codec: the 4-byte CAN-id prefix of ISO 22900-2:2009.
//!
//! In raw mode (PassThru) the client owns the on-wire header. For ISO 15765, SAE J1939 and ISO
//! 11898 "the first 4 bytes of the pCopData shall be the CAN ID (11 bit or 29 bit)"
//! (ISO 22900-2:2009 §9.2.6.3.6, request; §11.1.4.18.5, expected response). This module expresses
//! that prefix once, for any backend — it is exactly the fix a raw-mode read needs on send and recv.

use crate::error::{Error, Result};
use crate::link::CanId;

/// Prepend the 4-byte big-endian CAN id to an application PDU, giving the raw-mode request bytes a
/// pass-through backend puts on the wire (ISO 22900-2:2009 §9.2.6.3.6).
///
/// ```
/// use vcikit::{link::CanId, raw::encode_request};
/// // A GMLAN `$1A C1` read to 0x7E0 — the recovered native request bytes.
/// let bytes = encode_request(CanId::standard(0x7E0), &[0x1A, 0xC1]);
/// assert_eq!(bytes, [0x00, 0x00, 0x07, 0xE0, 0x1A, 0xC1]);
/// ```
#[must_use]
pub fn encode_request(id: CanId, app_pdu: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + app_pdu.len());
    out.extend_from_slice(&id.to_be_bytes());
    out.extend_from_slice(app_pdu);
    out
}

/// Split a raw-mode response: the first 4 bytes are the CAN id, the rest is the application PDU
/// (ISO 22900-2:2009 §11.1.4.18.5).
///
/// # Errors
///
/// Returns [`Error::ShortResponse`] when `raw` is shorter than the 4-byte CAN id.
///
/// ```
/// use vcikit::raw::decode_response;
/// // The recovered native reply to the read above.
/// let (id, app) = decode_response(&[0x00, 0x00, 0x07, 0xE8, 0x5A, 0xC1, 0x12, 0x34]).unwrap();
/// assert_eq!(id.value(), 0x7E8);
/// assert_eq!(app, &[0x5A, 0xC1, 0x12, 0x34]);
/// ```
pub fn decode_response(raw: &[u8]) -> Result<(CanId, &[u8])> {
    if raw.len() < 4 {
        return Err(Error::ShortResponse(raw.len()));
    }
    let id = CanId::from_be_bytes([raw[0], raw[1], raw[2], raw[3]]);
    Ok((id, &raw[4..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_round_trips_through_the_response_decoder() {
        let id = CanId::standard(0x7E0);
        let framed = encode_request(id, &[0x22, 0xF1, 0x90]);
        let (back, app) = decode_response(&framed).unwrap();
        assert_eq!(back.value(), id.value());
        assert_eq!(app, &[0x22, 0xF1, 0x90]);
    }

    #[test]
    fn empty_application_pdu_is_just_the_id() {
        assert_eq!(
            encode_request(CanId::standard(0x7DF), &[]),
            [0x00, 0x00, 0x07, 0xDF]
        );
    }

    #[test]
    fn a_response_shorter_than_the_id_is_rejected() {
        match decode_response(&[0x00, 0x00, 0x07]) {
            Err(Error::ShortResponse(3)) => {}
            other => panic!("expected ShortResponse(3), got {other:?}"),
        }
        // Exactly four bytes is an id with an empty application PDU, not an error.
        let (id, app) = decode_response(&[0x00, 0x00, 0x07, 0xE8]).unwrap();
        assert_eq!(id.value(), 0x7E8);
        assert!(
            app.is_empty(),
            "expected an empty application PDU, got {app:?}"
        );
    }
}
