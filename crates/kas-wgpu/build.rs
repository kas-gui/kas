// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Build script — shader compiler
//!
//! This script scans the directory (of the Cargo.toml manifest) for `*.vert`
//! and `*.frag` files, and compiles each to `*.vert.spv` etc., but only if
//! missing or out-of-date.
//!
//! To enable shader compilation, install a compiler such as glslc and set
//! `SHADERC=<PATH>`. For example, add this to your `~/.bash_profile`:
//! ```
//! export SHADERC=glslc
//! ```
//!
//! Warning: change detection is not perfect: this script will not automatically
//! be run when new `.vert` or `.frag` files are created. The easiest way to fix
//! this is to touch (re-save) any existing `.vert`/`.frag` file.

#![deny(warnings)]

use glob::glob;
use std::env;
use std::path::PathBuf;
use std::process::{Child, Command};

fn main() {
    let mut runners = Vec::new();

    println!("cargo:rerun-if-env-changed=SHADERC");
    let shaderc = match env::var("SHADERC") {
        Ok(s) => Some(s),
        Err(env::VarError::NotPresent) => None,
        Err(e) => panic!("failed to read env var SHADERC: {e}"),
    };

    let mut pat = env::var("CARGO_MANIFEST_DIR").unwrap();
    pat.push_str("/**/*.vert");
    walk(&pat, &shaderc, &mut runners);
    pat.replace_range((pat.len() - 4).., "frag");
    walk(&pat, &shaderc, &mut runners);

    for mut r in runners {
        let status = r.wait().unwrap();
        if !status.success() {
            panic!("Shader compilation failed (exit code {:?})", status.code());
        }
    }
}

fn walk(pat: &str, shaderc: &Option<String>, runners: &mut Vec<Child>) {
    for path in glob(pat).unwrap().filter_map(Result::ok) {
        println!("cargo:rerun-if-changed={}", path.display());

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
            if let Some(bin) = shaderc.as_ref() {
                let mut cmd = Command::new(bin);
                cmd.arg(&path).arg("-o").arg(&path_spv);
                eprintln!("Launching: {cmd:?}");
                runners.push(cmd.spawn().expect("shader compiler failed to start"));
            } else {
                eprintln!(
                    "cargo:warning=Shader compilation required: {}",
                    path.display()
                );
                eprintln!("cargo:warning=No shader found. If you have a shader compiler such as glslc installed, try setting SHADERC=glslc");
            }
        }
    }
}
