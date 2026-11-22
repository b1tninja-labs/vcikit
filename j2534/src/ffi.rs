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

/// If `path` is a PE DLL whose architecture does not match this process, return a clear
/// message; otherwise `None`. Reads only the PE header bytes (no `unsafe`, no load).
///
/// A 64-bit process cannot load a 32-bit DLL and vice versa — the common failure with the
/// VXDIAG pass-through, whose DLLs are 32-bit, so a reader must be built `i686`.
fn bitness_mismatch(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    // DOS header "MZ", e_lfanew (u32 LE) at 0x3C points to the PE header; the machine
    // word is a u16 LE at PE+4 (PE\0\0 signature, then the COFF header).
    if bytes.len() < 0x40 || &bytes[0..2] != b"MZ" {
        return None;
    }
    let pe = u32::from_le_bytes(bytes[0x3C..0x40].try_into().ok()?) as usize;
    if bytes.len() < pe + 6 || &bytes[pe..pe + 4] != b"PE\0\0" {
        return None;
    }
    let machine = u16::from_le_bytes(bytes[pe + 4..pe + 6].try_into().ok()?);
    let (dll_bits, dll_arch) = match machine {
        0x014C => (32, "x86 (32-bit)"),
        0x8664 => (64, "x64 (64-bit)"),
        0xAA64 => (64, "ARM64 (64-bit)"),
        _ => return None,
    };
    let proc_bits = if cfg!(target_pointer_width = "64") {
        64
    } else {
        32
    };
    if dll_bits == proc_bits {
        return None;
    }
    Some(format!(
        "{} is {dll_arch}, but this process is {proc_bits}-bit — a {proc_bits}-bit process \
         cannot load a {dll_bits}-bit DLL. The VXDIAG pass-through DLLs are 32-bit, so run the \
         32-bit (i686) reader.",
        path.display()
    ))
}

/// Load the DLL, resolving its dependent DLLs from its own directory.
///
/// On Windows this uses `LOAD_WITH_ALTERED_SEARCH_PATH` so a pass-through DLL (e.g.
/// VXDIAG's `VCXPT32.dll`) finds its sibling DLLs in the same folder regardless of the
/// process's current directory — the flag needs an absolute path, so the path is
/// canonicalised first.
#[cfg(windows)]
fn load_library(path: &Path) -> std::result::Result<Library, libloading::Error> {
    use libloading::os::windows::{LOAD_WITH_ALTERED_SEARCH_PATH, Library as WinLibrary};
    let absolute = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    // Put the DLL's own directory on the process search path BEFORE loading. A pass-through
    // DLL (VXDIAG's VCXPT32.dll) does its own runtime `LoadLibrary("VCX.DLL")`, which
    // LOAD_WITH_ALTERED_SEARCH_PATH does not cover (that flag only governs the direct DLL's
    // static imports). Without the sibling directory on the search path, VCXPT32 fails with
    // a modal "LoadLibrary(VCX.DLL) ErrorCode = 126" dialog that blocks the process.
    if let Some(dir) = absolute.parent() {
        set_dll_directory(dir);
    }
    // SAFETY: loading a DLL runs its initialisers; `path` is a J2534 pass-through the caller
    // chose. The altered search path only changes where its static imports resolve from.
    let library = unsafe { WinLibrary::load_with_flags(&absolute, LOAD_WITH_ALTERED_SEARCH_PATH) }?;
    Ok(library.into())
}

/// Add `dir` to the process DLL search path (`SetDllDirectoryW`), so a loaded DLL's own
/// runtime `LoadLibrary` calls find their siblings there regardless of the current directory.
#[cfg(windows)]
fn set_dll_directory(dir: &Path) {
    use std::os::windows::ffi::OsStrExt;
    unsafe extern "system" {
        fn SetDllDirectoryW(path: *const u16) -> i32;
    }
    let wide: Vec<u16> = dir
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // SAFETY: `wide` is a NUL-terminated UTF-16 path valid for the duration of the call.
    unsafe {
        SetDllDirectoryW(wide.as_ptr());
    }
}

#[cfg(not(windows))]
fn load_library(path: &Path) -> std::result::Result<Library, libloading::Error> {
    // SAFETY: loading a shared object runs its initialisers; this is the irreducible risk of
    // a pass-through backend.
    unsafe { Library::new(path) }
}

impl Dll {
    /// Load the pass-through DLL at `path` and verify it exports the J2534-1 entry points this
    /// backend uses.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DllLoad`] if the DLL cannot be loaded or is missing an entry point.
    pub fn load(path: &Path) -> Result<Self> {
        let lib = match load_library(path) {
            Ok(lib) => lib,
            Err(error) => {
                // A 32/64-bit mismatch is the most common load failure (the VXDIAG
                // pass-through DLLs are 32-bit); turn the cryptic OS error into a clear one.
                if let Some(message) = bitness_mismatch(path) {
                    return Err(Error::DllLoad(message));
                }
                return Err(Error::DllLoad(error.to_string()));
            }
        };
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
