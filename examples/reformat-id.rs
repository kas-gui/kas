// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Parse an [`Id`] as an integer, and reformat as a path

use kas_core::Id;

fn main() {
    let mut args = std::env::args();
    if args.len() != 2 {
        eprintln!("Usage: {} CODE", args.next().unwrap());
        eprintln!("where CODE is #HEX_PATH or DECIMAL");
        return;
    }

    let s = args.skip(1).next().unwrap();

    if s.starts_with("#") {
        print!("[");
        let mut first = true;
        let mut i = 1;
        let mut n = 0;
        while i < s.len() {
            let b = match u8::from_str_radix(&s[i..i + 1], 16) {
                Ok(b) => b,
                Err(err) => {
                    println!();
                    eprintln!("Parse error: {err}");
                    return;
                }
            };
            i += 1;

            n |= b & 7;
            if b & 8 != 0 {
                n <<= 3;
                continue;
            }

            if !first {
                print!(", ");
            }
            first = false;

            print!("{n}");
            n = 0;
        }
        println!("]");
        return;
    }

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
            eprintln!(
                "Note: long paths are stack allocated and cannot be reconstructed outside of the constructing thread"
            );
        }
    }
}
