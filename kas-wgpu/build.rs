// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Build script — shader compiler
//!
//! This script scans the directory (of the Cargo.toml manifest) for *.vert and
//! *.frag files, and compiles each to *.vert.spv etc., but only if missing or
//! out-of-date.
//!
//! Expects to find `glslc` (part of shaderc) on your path. On Linux this can
//! be installed with `sudo dnf install glslc` or similar. If all shaders are
//! up-to-date then this should not be required.

#![deny(warnings)]

use glob::glob;
use std::env;
use std::path::PathBuf;
use std::process::{Child, Command};

fn main() {
    let mut runners = Vec::new();

    let mut pat = String::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    pat.push_str("/**/*.vert");
    walk(&pat, &mut runners);
    pat.replace_range((pat.len() - 4).., "frag");
    walk(&pat, &mut runners);

    for mut r in runners {
        let status = r.wait().unwrap();
        if !status.success() {
            panic!("Shader compilation failed (exit code {:?})", status.code());
        }
    }
}

fn walk(pat: &str, runners: &mut Vec<Child>) {
    for path in glob(pat).unwrap().filter_map(Result::ok) {
        let mut path_spv = path.clone().into_os_string();
        path_spv.push(".spv");
        let path_spv = PathBuf::from(path_spv);
        let gen = match path_spv.metadata() {
            Ok(meta) => {
                let orig_meta = path.metadata().unwrap();
                orig_meta.modified().unwrap() > meta.modified().unwrap()
            }
            Err(_) => true,
        };
        if gen {
            // Shader compilation uses glslc (part of shaderc).
            let mut cmd = Command::new("glslc");
            cmd.arg(&path).arg("-o").arg(&path_spv);
            println!("Launching: {:?}", cmd);
            runners.push(cmd.spawn().expect("shader compiler failed to start"));
        }
    }
}
