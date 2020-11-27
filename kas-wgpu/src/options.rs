// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Options

use log::warn;
use std::env::var;
pub use wgpu::{BackendBit, PowerPreference};

/// Shell options
#[derive(Clone, PartialEq, Hash)]
pub struct Options {
    /// Adapter power preference. Default value: low power.
    pub power_preference: PowerPreference,
    /// Adapter backend. Default value: PRIMARY (Vulkan/Metal/DX12).
    pub backends: BackendBit,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            power_preference: PowerPreference::LowPower,
            backends: BackendBit::PRIMARY,
        }
    }
}

impl Options {
    /// Construct a new instance, reading from environment variables
    ///
    /// The following environment variables are read, in case-insensitive mode.
    ///
    /// ### Power preference
    ///
    /// The `KAS_POWER_PREFERENCE` variable supports:
    ///
    /// -   `Default`
    /// -   `LowPower`
    /// -   `HighPerformance`
    ///
    /// ### Backend
    ///
    /// The `KAS_BACKENDS` variable supports:
    ///
    /// -   `Vulkan`
    /// -   `GL`
    /// -   `Metal`
    /// -   `DX11`
    /// -   `DX12`
    /// -   `PRIMARY`: any of Vulkan, Metal or DX12
    /// -   `SECONDARY`: any of GL or DX11
    pub fn from_env() -> Self {
        let mut options = Options::default();

        if let Ok(mut v) = var("KAS_POWER_PREFERENCE") {
            v.make_ascii_uppercase();
            options.power_preference = match v.as_str() {
                "DEFAULT" => PowerPreference::Default,
                "LOWPOWER" => PowerPreference::LowPower,
                "HIGHPERFORMANCE" => PowerPreference::HighPerformance,
                other => {
                    warn!(
                        "Unexpected environment value: KAS_POWER_PREFERENCE={}",
                        other
                    );
                    options.power_preference
                }
            }
        }

        if let Ok(mut v) = var("KAS_BACKENDS") {
            v.make_ascii_uppercase();
            options.backends = match v.as_str() {
                "VULKAN" => BackendBit::VULKAN,
                "GL" => BackendBit::GL,
                "METAL" => BackendBit::METAL,
                "DX11" => BackendBit::DX11,
                "DX12" => BackendBit::DX12,
                "PRIMARY" => BackendBit::PRIMARY,
                "SECONDARY" => BackendBit::SECONDARY,
                other => {
                    warn!("Unexpected environment value: KAS_BACKENDS={}", other);
                    options.backends
                }
            }
        }

        options
    }

    pub(crate) fn adapter_options(&self) -> wgpu::RequestAdapterOptions {
        wgpu::RequestAdapterOptions {
            power_preference: self.power_preference,
            compatible_surface: None,
        }
    }

    pub(crate) fn backend(&self) -> BackendBit {
        self.backends
    }
}
