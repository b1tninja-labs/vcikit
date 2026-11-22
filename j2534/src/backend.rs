//! The [`J2534Backend`]: a [`RawBackend`] driven by a registered SAE J2534 pass-through DLL.

use std::path::Path;
use std::time::Duration;

use vcikit::backend::RawBackend;
use vcikit::error::{Error, Result};
use vcikit::j2534::{self, PassThruMsg, consts};
use vcikit::link::LinkConfig;
use vcikit::raw;

use crate::discovery;
use crate::ffi::Dll;

/// How long a single `PassThruWriteMsgs` may take (ms).
const WRITE_TIMEOUT_MS: u32 = 1000;
/// How many messages to drain while skipping transmit echoes before giving up on a reply.
const MAX_READ_DRAIN: usize = 8;

/// A vehicle communication interface driven through a registered SAE J2534 pass-through DLL.
///
/// It discovers (or is given) a pass-through DLL, loads it, and on [`connect`](RawBackend::connect)
/// brings an ISO 15765 link up: `PassThruOpen` → `PassThruConnect(baud/flags)` →
/// `PassThruStartMsgFilter(flow control)`. [`send`](RawBackend::send) and [`recv`](RawBackend::recv)
/// then carry application PDUs with the 4-byte CAN-id prefix handled by [`vcikit::raw`]. The DLL it
/// loads may be VXDIAG's today or a pass-through hosted by `vcxtoolkit` tomorrow — this backend
/// consumes whatever is registered.
#[derive(Debug)]
pub struct J2534Backend {
    dll: Dll,
    library_path: String,
    device_id: Option<u32>,
    channel_id: Option<u32>,
    config: Option<LinkConfig>,
}

impl J2534Backend {
    /// Open the first registered pass-through DLL that advertises ISO 15765 (falling back to any
    /// registered DLL).
    ///
    /// # Errors
    ///
    /// Returns [`Error::NoDevice`] when none is registered — including on every non-Windows
    /// platform, where discovery is always empty — or [`Error::DllLoad`] if the chosen DLL cannot
    /// be loaded.
    pub fn open_registered() -> Result<Self> {
        let devices = discovery::discover();
        let chosen = devices
            .iter()
            .find(|d| d.has_library() && d.supports_iso15765())
            .or_else(|| devices.iter().find(|d| d.has_library()))
            .ok_or(Error::NoDevice)?;
        let path = chosen.function_library.as_ref().ok_or(Error::NoDevice)?;
        Self::open_library(Path::new(path))
    }

    /// Load a specific pass-through DLL by path, without consulting the registry.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DllLoad`] if the DLL cannot be loaded or is missing a J2534-1 entry point.
    pub fn open_library(path: &Path) -> Result<Self> {
        Ok(Self {
            dll: Dll::load(path)?,
            library_path: path.display().to_string(),
            device_id: None,
            channel_id: None,
            config: None,
        })
    }

    /// The path of the loaded pass-through DLL.
    #[must_use]
    pub fn library_path(&self) -> &str {
        &self.library_path
    }

    fn channel(&self) -> Result<u32> {
        self.channel_id
            .ok_or(Error::Unsupported("link is not connected"))
    }
}

impl RawBackend for J2534Backend {
    fn connect(&mut self, config: &LinkConfig) -> Result<()> {
        let device_id = self.dll.open()?;
        self.device_id = Some(device_id);
        let channel_id = self.dll.connect(
            device_id,
            consts::PROTO_ISO15765,
            j2534::connect_flags(config),
            config.baud,
        )?;
        self.channel_id = Some(channel_id);
        let [mask, pattern, flow] = j2534::flow_control_filter(config);
        self.dll
            .start_flow_control_filter(channel_id, &mask, &pattern, &flow)?;
        self.config = Some(*config);
        Ok(())
    }

    fn send(&mut self, app_pdu: &[u8]) -> Result<()> {
        let config = self
            .config
            .ok_or(Error::Unsupported("link is not connected"))?;
        let channel_id = self.channel()?;
        let framed = raw::encode_request(config.request_id, app_pdu);
        let msg = PassThruMsg::iso15765(&framed, j2534::tx_flags(&config));
        self.dll.write_msg(channel_id, &msg, WRITE_TIMEOUT_MS)
    }

    fn recv(&mut self, timeout: Duration) -> Result<Vec<u8>> {
        let channel_id = self.channel()?;
        let timeout_ms = u32::try_from(timeout.as_millis()).unwrap_or(u32::MAX);
        for _ in 0..MAX_READ_DRAIN {
            let (status, msg) = self.dll.read_msg(channel_id, timeout_ms)?;
            if status == consts::ERR_BUFFER_EMPTY {
                return Err(Error::Timeout);
            }
            if status != consts::STATUS_NOERROR {
                return Err(Error::PassThru {
                    call: "PassThruReadMsgs",
                    status,
                    detail: consts::status_name(status).unwrap_or("unknown").to_string(),
                });
            }
            if msg.is_tx_echo() {
                continue; // our own transmit loopback, not an ECU reply
            }
            let (_id, app) = raw::decode_response(msg.payload())?;
            return Ok(app.to_vec());
        }
        Err(Error::Timeout)
    }

    fn close(&mut self) -> Result<()> {
        if let Some(channel_id) = self.channel_id.take() {
            self.dll.disconnect(channel_id);
        }
        if let Some(device_id) = self.device_id.take() {
            self.dll.close(device_id);
        }
        self.config = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(not(windows))]
    fn open_registered_reports_no_device_off_windows() {
        // Discovery is empty off Windows, so there is nothing to load.
        match J2534Backend::open_registered() {
            Err(Error::NoDevice) => {}
            other => panic!("expected NoDevice, got {other:?}"),
        }
    }

    #[test]
    fn loading_a_non_dll_path_is_a_dll_load_error() {
        match J2534Backend::open_library(Path::new("this-path-does-not-exist.dll")) {
            Err(Error::DllLoad(_)) => {}
            other => panic!("expected DllLoad, got {other:?}"),
        }
    }
}
