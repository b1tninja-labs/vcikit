//! The error type returned across the vcikit backend seam.

use std::fmt;

/// The result of a vcikit operation.
pub type Result<T> = std::result::Result<T, Error>;

/// Why a vcikit backend operation failed.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// The operation is not supported in this build or on this platform — the J2534 backend on a
    /// non-Windows target, or the native-VCX backend, which is not implemented yet (blocked on
    /// `vcxtoolkit`'s raw-mode channel bring-up; see `PLAN.md`). Carries a short reason.
    Unsupported(&'static str),
    /// No registered J2534 pass-through device was found, or none advertised ISO 15765 support.
    NoDevice,
    /// A pass-through DLL could not be loaded, or did not export the J2534-1 entry points.
    DllLoad(String),
    /// A J2534 pass-through call returned a non-zero status code.
    PassThru {
        /// The J2534-1 function that failed (e.g. `"PassThruConnect"`).
        call: &'static str,
        /// The returned status code (see `j2534::consts`).
        status: u32,
        /// The device's own `PassThruGetLastError` text, when available.
        detail: String,
    },
    /// A raw-mode response was shorter than the 4-byte CAN id the convention requires. Carries the
    /// length that was seen.
    ShortResponse(usize),
    /// A read timed out with no message queued.
    Timeout,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported(why) => write!(f, "unsupported: {why}"),
            Self::NoDevice => {
                f.write_str("no registered J2534 ISO 15765 pass-through device found")
            }
            Self::DllLoad(why) => write!(f, "pass-through DLL load failed: {why}"),
            Self::PassThru {
                call,
                status,
                detail,
            } => write!(f, "{call} returned status 0x{status:02X}: {detail}"),
            Self::ShortResponse(len) => {
                write!(
                    f,
                    "response of {len} byte(s) is shorter than the 4-byte CAN id"
                )
            }
            Self::Timeout => f.write_str("read timed out with no message"),
        }
    }
}

impl std::error::Error for Error {}
