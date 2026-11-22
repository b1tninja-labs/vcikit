//! Bench harness: drive the VENDOR J2534 DLL (e.g. VXDIAG `VCXPT32.dll`) to send one
//! TesterPresent (`$3E 00`) and print exactly what comes back. READ-ONLY: it sends
//! only `$3E 00` and never a write service.
//!
//! This exists to settle one question on the bench (no vehicle): does the vendor DLL
//! return a positive `$7E` to TesterPresent with nothing on the CAN bus? If it does,
//! the vendor does something the native path must match; if it does not, a `$7E`
//! needs a real ECU.
//!
//! Usage: `bench_tp [DLL_PATH] [REQ_HEX] [RESP_HEX]`
//!   defaults: `VCXPT32.dll` next to this exe, request `7E0`, response `7E8`.

use std::path::Path;
use std::time::Duration;

use vcikit::backend::RawBackend;
use vcikit::link::{CanId, LinkConfig};
use vcikit_j2534::J2534Backend;

fn parse_id(arg: Option<String>, default: u16) -> u16 {
    arg.and_then(|s| {
        let s = s.trim();
        let s = s
            .strip_prefix("0x")
            .or_else(|| s.strip_prefix("0X"))
            .unwrap_or(s);
        u16::from_str_radix(s, 16).ok()
    })
    .unwrap_or(default)
}

fn main() {
    let mut args = std::env::args().skip(1);
    let default_dll = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("VCXPT32.dll")))
        .map_or_else(|| "VCXPT32.dll".to_string(), |p| p.display().to_string());
    let dll = args.next().unwrap_or(default_dll);
    let req = parse_id(args.next(), 0x7E0);
    let resp = parse_id(args.next(), 0x7E8);

    println!("bench_tp - vendor J2534 TesterPresent probe (READ-ONLY $3E 00)");
    println!("  DLL:  {dll}");
    println!("  req:  {req:#05X}   resp: {resp:#05X}");
    println!();

    let mut backend = match J2534Backend::open_library(Path::new(&dll)) {
        Ok(backend) => {
            println!("loaded vendor DLL: {}", backend.library_path());
            backend
        }
        Err(error) => {
            eprintln!("!! could not load the DLL: {error}");
            eprintln!("   put bench_tp.exe next to VCXPT32.dll so its sibling DLLs resolve.");
            std::process::exit(1);
        }
    };

    let config = LinkConfig::iso15765(CanId::standard(req), CanId::standard(resp));
    if let Err(error) = backend.connect(&config) {
        eprintln!("!! connect (PassThruOpen/Connect/StartMsgFilter) failed: {error}");
        std::process::exit(1);
    }
    println!(
        "connected: ISO 15765 @ {} baud, flow-control filter resp {resp:#05X} <- req {req:#05X}",
        config.baud
    );

    println!("sending TesterPresent: $3E 00 ...");
    if let Err(error) = backend.send(&[0x3E, 0x00]) {
        eprintln!("!! PassThruWriteMsgs failed: {error}");
        std::process::exit(1);
    }

    match backend.recv(Duration::from_secs(2)) {
        Ok(app) => {
            println!();
            println!(">> REPLY application bytes: {app:02X?}");
            if app.first() == Some(&0x7E) {
                println!(">> POSITIVE $7E: the vendor DLL GOT a TesterPresent response.");
            } else {
                println!(">> not a positive $7E (a positive reply starts with 0x7E).");
            }
        }
        Err(error) => {
            println!();
            println!(">> no reply within 2s: {error}");
            println!("   (on an empty bus with no ECU, this is the expected result.)");
        }
    }
    let _ = backend.close();
}
