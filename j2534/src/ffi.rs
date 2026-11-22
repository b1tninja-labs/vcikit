//! The SAE J2534-1 C-ABI binding — the one module that holds `unsafe`.
//!
//! J2534-1 is a C API exported from a registered pass-through DLL. Loading that DLL and calling its
//! entry points is irreducibly `unsafe` FFI; it is confined here so the rest of this crate, and the
//! `vcikit` abstraction crate, stay safe. Every `unsafe` block is a single foreign call whose
//! arguments match the J2534-1 v04.04 prototype: a device/channel id, a `u32` out-pointer, and/or a
//! `#[repr(C)]` [`PassThruMsg`] pointer the DLL reads or writes.
#![allow(unsafe_code)]

use std::ffi::c_void;
use std::os::raw::c_char;
use std::path::Path;

use libloading::Library;
use vcikit::error::{Error, Result};
use vcikit::j2534::{PassThruMsg, consts};

type OpenFn = unsafe extern "system" fn(*mut c_void, *mut u32) -> u32;
type CloseFn = unsafe extern "system" fn(u32) -> u32;
type ConnectFn = unsafe extern "system" fn(u32, u32, u32, u32, *mut u32) -> u32;
type DisconnectFn = unsafe extern "system" fn(u32) -> u32;
type StartMsgFilterFn = unsafe extern "system" fn(
    u32,
    u32,
    *const PassThruMsg,
    *const PassThruMsg,
    *const PassThruMsg,
    *mut u32,
) -> u32;
type WriteMsgsFn = unsafe extern "system" fn(u32, *const PassThruMsg, *mut u32, u32) -> u32;
type ReadMsgsFn = unsafe extern "system" fn(u32, *mut PassThruMsg, *mut u32, u32) -> u32;
type GetLastErrorFn = unsafe extern "system" fn(*mut c_char) -> u32;

/// A loaded pass-through DLL and its J2534-1 entry points. Methods are safe wrappers over the
/// foreign calls; they return [`Error::PassThru`] with the device's own last-error text on a
/// non-zero status.
#[derive(Debug)]
pub struct Dll {
    lib: Library,
}

impl Dll {
    /// Load the pass-through DLL at `path` and verify it exports the J2534-1 entry points this
    /// backend uses.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DllLoad`] if the DLL cannot be loaded or is missing an entry point.
    pub fn load(path: &Path) -> Result<Self> {
        // SAFETY: loading a DLL runs its initialisers; `path` is a J2534 pass-through the user has
        // registered. This is the irreducible risk of a pass-through backend.
        let lib = unsafe { Library::new(path) }.map_err(|e| Error::DllLoad(e.to_string()))?;
        let dll = Self { lib };
        // Fail early and clearly if the DLL is not a J2534 pass-through.
        for symbol in [
            "PassThruOpen",
            "PassThruClose",
            "PassThruConnect",
            "PassThruDisconnect",
            "PassThruStartMsgFilter",
            "PassThruWriteMsgs",
            "PassThruReadMsgs",
        ] {
            dll.require(symbol)?;
        }
        Ok(dll)
    }

    fn require(&self, symbol: &str) -> Result<()> {
        let mut name = symbol.as_bytes().to_vec();
        name.push(0);
        // SAFETY: a symbol lookup reads only the export table; it runs no foreign code.
        unsafe { self.lib.get::<*const c_void>(&name) }
            .map(|_| ())
            .map_err(|_| Error::DllLoad(format!("{symbol} not exported")))
    }

    fn last_error(&self) -> String {
        let mut buf = [0_u8; 80];
        // SAFETY: `PassThruGetLastError` writes an ASCII string of up to 80 bytes into `buf`.
        let Ok(get) = (unsafe { self.lib.get::<GetLastErrorFn>(b"PassThruGetLastError\0") }) else {
            return "(no detail)".to_string();
        };
        // SAFETY: `buf` is 80 bytes, the size the J2534-1 spec requires the caller to provide.
        unsafe { get(buf.as_mut_ptr().cast::<c_char>()) };
        let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        String::from_utf8_lossy(&buf[..end]).into_owned()
    }

    fn check(&self, status: u32, call: &'static str) -> Result<()> {
        if status == consts::STATUS_NOERROR {
            Ok(())
        } else {
            Err(Error::PassThru {
                call,
                status,
                detail: self.last_error(),
            })
        }
    }

    fn symbol<T>(&self, name: &[u8]) -> Result<libloading::Symbol<'_, T>> {
        // SAFETY: a symbol lookup reads only the export table; `load` already verified the names.
        unsafe { self.lib.get::<T>(name) }
            .map_err(|e| Error::DllLoad(format!("symbol lookup failed: {e}")))
    }

    /// `PassThruOpen(NULL, &device_id)`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::PassThru`] if the device cannot be opened.
    pub fn open(&self) -> Result<u32> {
        let open = self.symbol::<OpenFn>(b"PassThruOpen\0")?;
        let mut device_id = 0_u32;
        // SAFETY: the first argument is the documented NULL device name; the second is a valid
        // out-pointer the DLL writes the device id into.
        let status = unsafe { open(std::ptr::null_mut(), &raw mut device_id) };
        self.check(status, "PassThruOpen")?;
        Ok(device_id)
    }

    /// `PassThruConnect(device_id, protocol, flags, baud, &channel_id)`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::PassThru`] if the channel cannot be connected.
    pub fn connect(&self, device_id: u32, protocol: u32, flags: u32, baud: u32) -> Result<u32> {
        let connect = self.symbol::<ConnectFn>(b"PassThruConnect\0")?;
        let mut channel_id = 0_u32;
        // SAFETY: scalar arguments by value; the last is a valid out-pointer for the channel id.
        let status = unsafe { connect(device_id, protocol, flags, baud, &raw mut channel_id) };
        self.check(status, "PassThruConnect")?;
        Ok(channel_id)
    }

    /// `PassThruStartMsgFilter(channel_id, FLOW_CONTROL_FILTER, mask, pattern, flow, &filter_id)`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::PassThru`] if the flow-control filter cannot be set.
    pub fn start_flow_control_filter(
        &self,
        channel_id: u32,
        mask: &PassThruMsg,
        pattern: &PassThruMsg,
        flow: &PassThruMsg,
    ) -> Result<u32> {
        let start = self.symbol::<StartMsgFilterFn>(b"PassThruStartMsgFilter\0")?;
        let mut filter_id = 0_u32;
        // SAFETY: the three message pointers are valid `#[repr(C)] PassThruMsg` the DLL only reads;
        // the last is a valid out-pointer.
        let status = unsafe {
            start(
                channel_id,
                consts::FLOW_CONTROL_FILTER,
                std::ptr::from_ref(mask),
                std::ptr::from_ref(pattern),
                std::ptr::from_ref(flow),
                &raw mut filter_id,
            )
        };
        self.check(status, "PassThruStartMsgFilter")?;
        Ok(filter_id)
    }

    /// `PassThruWriteMsgs(channel_id, &msg, &1, timeout_ms)`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::PassThru`] if the write fails.
    pub fn write_msg(&self, channel_id: u32, msg: &PassThruMsg, timeout_ms: u32) -> Result<()> {
        let write = self.symbol::<WriteMsgsFn>(b"PassThruWriteMsgs\0")?;
        let mut num = 1_u32;
        // SAFETY: `msg` is a valid `#[repr(C)] PassThruMsg` the DLL only reads; `num` is a valid
        // in/out pointer initialised to the message count.
        let status = unsafe {
            write(
                channel_id,
                std::ptr::from_ref(msg),
                &raw mut num,
                timeout_ms,
            )
        };
        self.check(status, "PassThruWriteMsgs")
    }

    /// `PassThruReadMsgs(channel_id, &msg, &1, timeout_ms)`, returning the raw status and the
    /// message (the caller interprets `ERR_BUFFER_EMPTY` and the echo flag).
    ///
    /// # Errors
    ///
    /// Returns [`Error::DllLoad`] if the entry point cannot be resolved; the J2534 status itself is
    /// returned in the tuple for the caller to interpret.
    pub fn read_msg(&self, channel_id: u32, timeout_ms: u32) -> Result<(u32, PassThruMsg)> {
        let read = self.symbol::<ReadMsgsFn>(b"PassThruReadMsgs\0")?;
        let mut msg = PassThruMsg::default();
        let mut num = 1_u32;
        // SAFETY: `msg` is a valid, owned `#[repr(C)] PassThruMsg` the DLL writes into; `num` is a
        // valid in/out pointer initialised to the buffer count.
        let status = unsafe {
            read(
                channel_id,
                std::ptr::from_mut(&mut msg),
                &raw mut num,
                timeout_ms,
            )
        };
        if num == 0 {
            return Ok((consts::ERR_BUFFER_EMPTY, msg));
        }
        Ok((status, msg))
    }

    /// `PassThruDisconnect(channel_id)` — best effort, errors ignored on teardown.
    pub fn disconnect(&self, channel_id: u32) {
        if let Ok(disconnect) = self.symbol::<DisconnectFn>(b"PassThruDisconnect\0") {
            // SAFETY: a scalar channel id by value.
            let _ = unsafe { disconnect(channel_id) };
        }
    }

    /// `PassThruClose(device_id)` — best effort, errors ignored on teardown.
    pub fn close(&self, device_id: u32) {
        if let Ok(close) = self.symbol::<CloseFn>(b"PassThruClose\0") {
            // SAFETY: a scalar device id by value.
            let _ = unsafe { close(device_id) };
        }
    }
}
