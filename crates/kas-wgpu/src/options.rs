// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Options

use std::env::var;
use std::path::PathBuf;
pub use wgpu::{Backends, PowerPreference};

/// Shell options
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Options {
    /// Adapter power preference. Default value: low power.
    pub power_preference: PowerPreference,
    /// Adapter backend. Default value: PRIMARY (Vulkan/Metal/DX12).
    pub backends: Backends,
    /// WGPU's API tracing path
    pub wgpu_trace_path: Option<PathBuf>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            power_preference: PowerPreference::LowPower,
            backends: Backends::all(),
            wgpu_trace_path: None,
        }
    }
}

impl Options {
    /// Construct a new instance, reading from environment variables
    ///
    /// The following environment variables are read, in case-insensitive mode.
    ///
    /// # Graphics options
    ///
    /// The `KAS_POWER_PREFERENCE` variable supports:
    ///
    /// -   `Default`
    /// -   `LowPower`
    /// -   `HighPerformance`
    ///
    /// The `KAS_BACKENDS` variable supports:
    ///
    /// -   `Vulkan`
    /// -   `GL`
    /// -   `Metal`
    /// -   `DX11`
    /// -   `DX12`
    /// -   `BROWSER_WEBGPU`: web target through webassembly
    /// -   `PRIMARY`: any of Vulkan, Metal or DX12
    /// -   `SECONDARY`: any of GL or DX11
    /// -   `FALLBACK`: force use of fallback (CPU) rendering
    ///
    /// WGPU has an [API tracing] feature for debugging. To use this, ensure the
    /// `wgpu/trace` feature is enabled and set the output path:
    /// ```sh
    /// export KAS_WGPU_TRACE_PATH="api_trace"
    /// ```
    ///
    /// [API tracing]: https://github.com/gfx-rs/wgpu/wiki/Debugging-wgpu-Applications#tracing-infrastructure
    pub fn from_env() -> Self {
        let mut options = Options::default();

        if let Ok(mut v) = var("KAS_POWER_PREFERENCE") {
            v.make_ascii_uppercase();
            options.power_preference = match v.as_str() {
                "DEFAULT" | "LOWPOWER" => PowerPreference::LowPower,
                "HIGHPERFORMANCE" => PowerPreference::HighPerformance,
                other => {
                    log::error!("from_env: bad var KAS_POWER_PREFERENCE={other}");
                    log::error!(
                        "from_env: supported power modes: DEFAULT, LOWPOWER, HIGHPERFORMANCE"
                    );
                    options.power_preference
                }
            }
        }

        if let Ok(mut v) = var("KAS_BACKENDS") {
            v.make_ascii_uppercase();
            options.backends = match v.as_str() {
                "VULKAN" => Backends::VULKAN,
                "GL" => Backends::GL,
                "METAL" => Backends::METAL,
                "DX11" => Backends::DX11,
                "DX12" => Backends::DX12,
                "BROWSER_WEBGPU" => Backends::BROWSER_WEBGPU,
                "PRIMARY" => Backends::PRIMARY,
                "SECONDARY" => Backends::SECONDARY,
                "FALLBACK" => Backends::empty(),
                other => {
                    log::error!("from_env: bad var KAS_BACKENDS={other}");
                    log::error!("from_env: supported backends: VULKAN, GL, METAL, DX11, DX12, BROWSER_WEBGPU, PRIMARY, SECONDARY, FALLBACK");
                    options.backends
                }
            }
        }

        if let Ok(v) = var("KAS_WGPU_TRACE_PATH") {
            options.wgpu_trace_path = Some(v.into());
        }

        options
    }

    pub(crate) fn adapter_options(&self) -> wgpu::RequestAdapterOptions {
        wgpu::RequestAdapterOptions {
            power_preference: self.power_preference,
            force_fallback_adapter: self.backends.is_empty(),
            compatible_surface: None,
        }
    }

    pub(crate) fn backend(&self) -> Backends {
        if self.backends.is_empty() {
            Backends::all()
        } else {
            self.backends
        }
    }
}
