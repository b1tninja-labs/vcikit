//! Discovery of registered SAE J2534 v04.04 pass-through devices.
//!
//! A J2534 device advertises itself under `HKLM\SOFTWARE\PassThruSupport.04.04` (and, for a 32-bit
//! DLL on 64-bit Windows, the `WOW6432Node` view). Each vendor subkey carries a `FunctionLibrary`
//! (the DLL path) and per-protocol capability values (`ISO15765`, `CAN`, …). Reading the registry is
//! Windows-only and uses no `unsafe`; on other platforms [`discover`] returns an empty list.

use std::collections::BTreeMap;

/// A registered J2534 v04.04 pass-through device, as read from one vendor subkey.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PassThruDevice {
    /// The device `Name` value.
    pub name: Option<String>,
    /// The `Vendor` value.
    pub vendor: Option<String>,
    /// The `FunctionLibrary` value — the path to the pass-through DLL to load.
    pub function_library: Option<String>,
    /// The `ConfigApplication` value, when present.
    pub config_application: Option<String>,
    /// Per-protocol capability values present under the subkey (`ISO15765`, `CAN`, …).
    pub protocols: BTreeMap<String, u32>,
    /// The registry subkey path the entry was read from.
    pub registry_key: String,
}

impl PassThruDevice {
    /// Whether the device advertises ISO 15765 support (a non-zero `ISO15765` value).
    #[must_use]
    pub fn supports_iso15765(&self) -> bool {
        self.protocols.get("ISO15765").is_some_and(|&v| v != 0)
    }

    /// Whether the entry names a DLL to load.
    #[must_use]
    pub fn has_library(&self) -> bool {
        self.function_library
            .as_ref()
            .is_some_and(|p| !p.is_empty())
    }
}

/// The protocol capability value names read from each subkey. Only the Windows registry path reads
/// them, so the constant is Windows-only (it would be dead code elsewhere).
#[cfg(windows)]
const PROTOCOL_VALUES: &[&str] = &[
    "ISO15765",
    "CAN",
    "ISO9141",
    "J1850PWM",
    "J1850VPW",
    "SCI_A_ENGINE",
];

/// Every registered J2534 v04.04 pass-through device. Empty on a non-Windows platform, or when none
/// is registered.
#[must_use]
pub fn discover() -> Vec<PassThruDevice> {
    platform::discover()
}

#[cfg(windows)]
mod platform {
    use super::{PROTOCOL_VALUES, PassThruDevice};
    use winreg::RegKey;
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ};

    const ROOTS: &[&str] = &[
        r"SOFTWARE\WOW6432Node\PassThruSupport.04.04",
        r"SOFTWARE\PassThruSupport.04.04",
    ];

    pub fn discover() -> Vec<PassThruDevice> {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let mut found = Vec::new();
        for base in ROOTS {
            let Ok(root) = hklm.open_subkey_with_flags(base, KEY_READ) else {
                continue;
            };
            for sub in root.enum_keys().flatten() {
                let Ok(key) = root.open_subkey_with_flags(&sub, KEY_READ) else {
                    continue;
                };
                let mut device = PassThruDevice {
                    registry_key: format!("{base}\\{sub}"),
                    ..PassThruDevice::default()
                };
                device.name = key.get_value("Name").ok();
                device.vendor = key.get_value("Vendor").ok();
                device.function_library = key.get_value("FunctionLibrary").ok();
                device.config_application = key.get_value("ConfigApplication").ok();
                for proto in PROTOCOL_VALUES {
                    if let Ok(value) = key.get_value::<u32, _>(proto) {
                        device.protocols.insert((*proto).to_string(), value);
                    }
                }
                found.push(device);
            }
        }
        found
    }
}

#[cfg(not(windows))]
mod platform {
    use super::PassThruDevice;

    pub fn discover() -> Vec<PassThruDevice> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supports_iso15765_reads_the_capability_value() {
        let mut device = PassThruDevice::default();
        assert!(!device.supports_iso15765());
        device.protocols.insert("ISO15765".to_string(), 0);
        assert!(!device.supports_iso15765());
        device.protocols.insert("ISO15765".to_string(), 0x0000_0C00);
        assert!(device.supports_iso15765());
    }

    #[test]
    fn has_library_requires_a_non_empty_path() {
        let mut device = PassThruDevice::default();
        assert!(!device.has_library());
        device.function_library = Some(String::new());
        assert!(!device.has_library());
        device.function_library = Some(r"C:\Windows\System32\VCXPT32.dll".to_string());
        assert!(device.has_library());
    }

    #[test]
    #[cfg(not(windows))]
    fn discovery_is_empty_off_windows() {
        let found = discover();
        assert!(
            found.is_empty(),
            "discovery should be empty off Windows, got {found:?}"
        );
    }
}
