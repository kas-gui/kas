// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Parse an [`Id`] as an integer, and reformat as a path

use kas_core::Id;

fn main() {
    let mut args = std::env::args();
    if args.len() != 2 {
        eprintln!("Usage: {} DECIMAL", args.next().unwrap());
        return;
    }

    let s = args.skip(1).next().unwrap();
    let n: u64 = match s.parse() {
        Ok(n) => n,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };

    if let Some(id) = Id::try_from_u64(n) {
        let path: Vec<usize> = id.iter().collect();
        println!("{id}: {path:?}");
    } else {
        eprintln!("Failed to convert {n}");
        if n & 3 == 2 {
            eprintln!("Note: long paths are stack allocated and cannot be reconstructed outside of the constructing thread");
        }
    }
}
