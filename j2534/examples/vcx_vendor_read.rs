//! The VENDOR-DLL read path: TesterPresent ($3E), then the GMLAN $1A identity DIDs,
//! driven through a registered/located J2534 pass-through DLL (VXDIAG `VCXPT32.dll`).
//! READ-ONLY: only `$3E` and `$1A` are ever sent — never a write service.
//!
//! This is the "read no matter what" backup the voltkit wizard's Vendor backend runs;
//! it mirrors `gmdiag vcx-read` but over the vendor DLL instead of our native serial
//! path. Build it 32-bit (`i686-pc-windows-msvc`) — the vendor DLL is x86.
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
    // Defaults.
    let default_dll = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("VCXPT32.dll")))
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "VCXPT32.dll".to_string());
    let mut dll = default_dll;
    let mut req = 0x7E0_u16;
    let mut resp = 0x7E8_u16;
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--dll" => dll = it.next().unwrap_or(dll),
            "--req" => req = parse_hex(&it.next().unwrap_or_default(), req),
            "--resp" => resp = parse_hex(&it.next().unwrap_or_default(), resp),
            // --port is accepted and ignored: the vendor DLL discovers its own device.
            "--port" => {
                let _ = it.next();
            }
            _ => {}
        }
    }

    println!("vcx vendor-DLL read  (READ-ONLY: $3E / $1A only, no $27, no writes)");
    println!("  DLL:  {dll}");
    println!("  req:  {req:#05X}   resp: {resp:#05X}   lids {}", hex(LIDS));
    println!();

    let mut backend = match J2534Backend::open_library(Path::new(&dll)) {
        Ok(backend) => {
            println!("loaded vendor DLL: {}", backend.library_path());
            backend
        }
        Err(error) => {
            eprintln!("!! could not load the DLL: {error}");
            eprintln!("   this exe must sit next to VCXPT32.dll (and be 32-bit).");
            std::process::exit(1);
        }
    };

    let config = LinkConfig::iso15765(CanId::standard(req), CanId::standard(resp));
    if let Err(error) = backend.connect(&config) {
        eprintln!("!! connect failed: {error}");
        std::process::exit(1);
    }
    println!(
        "connected: ISO 15765 @ {} baud, flow-control filter resp {resp:#05X} <- req {req:#05X}",
        config.baud
    );
    println!();

    // TesterPresent preflight.
    println!("TesterPresent probe ($3E 00):");
    if let Err(error) = backend.send(&[0x3E, 0x00]) {
        eprintln!("  !! write failed: {error}");
        std::process::exit(1);
    }
    match backend.recv(Duration::from_secs(2)) {
        Ok(app) if app.first() == Some(&0x7E) => {
            println!("  -> positive $7E: {}", hex(&app));
        }
        Ok(app) => {
            println!("  !! not a positive $7E: {}", hex(&app));
            println!("\nPREFLIGHT FAILED: the transport is not proven; stopping before the $1A read.");
            std::process::exit(1);
        }
        Err(error) => {
            println!("  !! no reply: {error}");
            println!(
                "\nPREFLIGHT FAILED: no TesterPresent response. On an empty bench bus (no vehicle) \
                 this is expected; a $7E needs a real ECU."
            );
            std::process::exit(1);
        }
    }
    println!();

    // The $1A identity sweep.
    let mut ok = 0u32;
    for &lid in LIDS {
        print!("lid {lid:02X}: ");
        if let Err(error) = backend.send(&[0x1A, lid]) {
            println!("write failed: {error}");
            continue;
        }
        match backend.recv(Duration::from_secs(2)) {
            Ok(app) if app.first() == Some(&0x5A) && app.get(1) == Some(&lid) => {
                println!("{}", hex(&app));
                ok += 1;
            }
            Ok(app) => println!("unexpected reply {}", hex(&app)),
            Err(error) => println!("no reply ({error})"),
        }
    }

    println!();
    println!("DONE. {ok}/{} identities read via the vendor DLL.", LIDS.len());
    let _ = backend.close();
    if ok == 0 {
        std::process::exit(1);
    }
}
