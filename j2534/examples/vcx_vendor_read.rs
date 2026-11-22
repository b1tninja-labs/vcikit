//! The VENDOR-DLL read path: TesterPresent ($3E), then the GMLAN $1A identity DIDs,
//! driven through a registered/located J2534 pass-through DLL (VXDIAG `VCXPT32.dll`).
//! READ-ONLY: only `$3E` and `$1A` are ever sent — never a write service.
//!
//! It runs piece by piece (load, connect, each config step, the probe, each DID) and
//! **auto-saves a timestamped session log to the current directory**, so a read can be
//! troubleshot from the record. Build it 32-bit (`i686-pc-windows-msvc`) — the vendor
//! DLL is x86.
//!
//! Usage: `vcx_vendor_read [--port N] [--dll PATH] [--req HEX] [--resp HEX]`
//!   defaults: `VCXPT32.dll` next to this exe, request `7E0`, response `7E8`.

use std::path::Path;
use std::time::Duration;

use vcikit::backend::RawBackend;
use vcikit::link::{CanId, LinkConfig};
use vcikit_j2534::J2534Backend;

/// The GMLAN part-number / hardware DIDs a VIT1 collect reads (the same set gmdiag uses).
const LIDS: &[u8] = &[
    0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xCB, 0xCC,
];

/// Tees every line to stdout and to an in-memory log that is written to the cwd at exit.
struct Session {
    log: String,
}

impl Session {
    fn new() -> Self {
        Self { log: String::new() }
    }

    fn line(&mut self, text: impl AsRef<str>) {
        let text = text.as_ref();
        println!("{text}");
        self.log.push_str(text);
        self.log.push('\n');
    }

    /// Write the collected log to `vcx-vendor-read-<unixsecs>.log` in the cwd.
    fn save(&self) {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let path = std::path::PathBuf::from(format!("vcx-vendor-read-{secs}.log"));
        match std::fs::write(&path, &self.log) {
            Ok(()) => println!("\n[session log saved: {}]", path.display()),
            Err(error) => eprintln!("\n[could not write the session log: {error}]"),
        }
    }
}

fn parse_hex(value: &str, default: u16) -> u16 {
    let value = value.trim();
    let value = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .unwrap_or(value);
    u16::from_str_radix(value, 16).unwrap_or(default)
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn main() {
    let mut s = Session::new();
    let code = run(&mut s);
    s.save();
    std::process::exit(code);
}

fn run(s: &mut Session) -> i32 {
    let default_dll = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("VCXPT32.dll")))
        .map_or_else(|| "VCXPT32.dll".to_string(), |p| p.display().to_string());
    let mut dll = default_dll;
    let mut req = 0x7E0_u16;
    let mut resp = 0x7E8_u16;
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--dll" => dll = it.next().unwrap_or(dll),
            "--req" => req = parse_hex(&it.next().unwrap_or_default(), req),
            "--resp" => resp = parse_hex(&it.next().unwrap_or_default(), resp),
            "--port" => {
                let _ = it.next();
            }
            _ => {}
        }
    }

    s.line("vcx vendor-DLL read  (READ-ONLY: $3E / $1A only, no $27, no writes)");
    s.line(format!("  DLL:  {dll}"));
    s.line(format!(
        "  req:  {req:#05X}   resp: {resp:#05X}   lids {}",
        hex(LIDS)
    ));
    s.line("");

    s.line("step 1/4 — load the vendor DLL");
    let mut backend = match J2534Backend::open_library(Path::new(&dll)) {
        Ok(backend) => {
            s.line(format!("  OK: loaded {}", backend.library_path()));
            backend
        }
        Err(error) => {
            s.line(format!("  FAIL: could not load the DLL: {error}"));
            s.line("  (this exe must sit next to VCXPT32.dll and be 32-bit.)");
            return 1;
        }
    };

    s.line("step 2/4 — connect ISO 15765 (PassThruOpen / Connect / StartMsgFilter)");
    let config = LinkConfig::iso15765(CanId::standard(req), CanId::standard(resp));
    if let Err(error) = backend.connect(&config) {
        s.line(format!("  FAIL: connect: {error}"));
        return 1;
    }
    s.line(format!(
        "  OK: ISO 15765 @ {} baud, flow-control filter resp {resp:#05X} <- req {req:#05X}",
        config.baud
    ));
    s.line("");

    s.line("step 3/4 — TesterPresent probe ($3E 00)");
    if let Err(error) = backend.send(&[0x3E, 0x00]) {
        s.line(format!("  FAIL: write: {error}"));
        return 1;
    }
    match backend.recv(Duration::from_secs(2)) {
        Ok(app) if app.first() == Some(&0x7E) => {
            s.line(format!("  OK: positive $7E: {}", hex(&app)));
        }
        Ok(app) => {
            s.line(format!("  FAIL: not a positive $7E: {}", hex(&app)));
            return 1;
        }
        Err(error) => {
            s.line(format!("  FAIL: no reply: {error}"));
            s.line(
                "  (on an empty bench bus with no ECU this is expected; a $7E needs a real car.)",
            );
            return 1;
        }
    }
    s.line("");

    s.line("step 4/4 — $1A identity sweep");
    let mut ok = 0u32;
    for &lid in LIDS {
        if let Err(error) = backend.send(&[0x1A, lid]) {
            s.line(format!("  lid {lid:02X}: write failed: {error}"));
            continue;
        }
        match backend.recv(Duration::from_secs(2)) {
            Ok(app) if app.first() == Some(&0x5A) && app.get(1) == Some(&lid) => {
                s.line(format!("  lid {lid:02X}: {}", hex(&app)));
                ok += 1;
            }
            Ok(app) => s.line(format!("  lid {lid:02X}: unexpected reply {}", hex(&app))),
            Err(error) => s.line(format!("  lid {lid:02X}: no reply ({error})")),
        }
    }

    s.line("");
    s.line(format!(
        "DONE. {ok}/{} identities read via the vendor DLL.",
        LIDS.len()
    ));
    let _ = backend.close();
    i32::from(ok == 0)
}
