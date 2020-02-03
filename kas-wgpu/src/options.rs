// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Options

/// Toolkit options
pub struct Options {
    /// Adapter power preference. Default value: low power.
    pub power_preference: wgpu::PowerPreference,
    /// Adapter backend. Default value: PRIMARY (Vulkan/Metal/DX12).
    pub backends: wgpu::BackendBit,
}

impl Options {
    /// Construct a new instance with default values
    pub fn new() -> Self {
        Options {
            power_preference: wgpu::PowerPreference::LowPower,
            backends: wgpu::BackendBit::PRIMARY,
        }
    }

    pub(crate) fn adapter_options(&self) -> wgpu::RequestAdapterOptions {
        wgpu::RequestAdapterOptions {
            power_preference: self.power_preference,
            backends: self.backends,
        }
    }
}
