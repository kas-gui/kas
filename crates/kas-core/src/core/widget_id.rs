// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget identifiers

use crate::cast::Conv;
use std::fmt;
use std::iter::once;
use std::mem::size_of;
use std::num::NonZeroU64;
use std::sync::Mutex;

/// Widget identifier
///
/// All widgets are assigned an identifier which is unique within the window.
/// This type may be tested for equality and order.
///
/// This type is small and cheap to copy. Internally it is "NonZero", thus
/// `Option<WidgetId>` is a free extension (requires no extra memory).
///
/// Identifiers are assigned when configured and when re-configured
/// (via [`crate::TkAction::RECONFIGURE`]). Since user-code is not notified of a
/// re-configure, user-code should not store a `WidgetId`.
#[derive(Clone, Copy, Hash, Eq)]
pub struct WidgetId(NonZeroU64);

/// The first byte (head) controls interpretation of the rest
const MASK_HEAD: u64 = 0xC000_0000_0000_0000;
const MASK_LEN: u64 = 0x0F00_0000_0000_0000;
const SHIFT_LEN: u8 = 56;
const BLOCKS: u8 = 14;
const MASK_REST: u64 = 0x00FF_FFFF_FFFF_FFFF;

/// `(x & MASK_HEAD) == USE_BITS`: rest is a sequence of 4-bit blocks; len is number of blocks used
const USE_BITS: u64 = 0x8000_0000_0000_0000;
/// `(x & MASK_HEAD) == USE_DB`: rest is index in DB
const USE_DB: u64 = 0x4000_0000_0000_0000;

#[inline]
fn block_len(x: u64) -> u8 {
    ((x & MASK_LEN) >> SHIFT_LEN) as u8
}

// Returns usize read from x plus blocks used
fn next_from_bits(mut x: u64) -> (usize, u8) {
    const TAKE: u64 = 0x7000_0000_0000_0000;
    const HIGH: u64 = 0x8000_0000_0000_0000;
    let mut y = (x & TAKE) >> 60;
    let mut c = 1;
    while (x & HIGH) != 0 {
        x <<= 4;
        y = (y << 3) | ((x & TAKE) >> 60);
        c += 1;
    }
    (y as usize, c)
}

struct BitsIter(u8, u64);
impl BitsIter {
    fn new(bits: u64) -> Self {
        assert!((bits & USE_BITS) != 0);
        let len = (bits & MASK_LEN) >> SHIFT_LEN;
        BitsIter(len as u8, bits << (64 - SHIFT_LEN))
    }
}
impl Iterator for BitsIter {
    type Item = usize;
    fn next(&mut self) -> Option<usize> {
        if self.0 == 0 {
            return None;
        }
        let (next, blocks) = next_from_bits(self.1);
        self.0 -= blocks;
        self.1 <<= 4 * blocks;
        Some(next)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        ((self.0 != 0) as usize, Some(self.0 as usize))
    }
}

impl WidgetId {
    /// Identifier of the window
    pub(crate) const ROOT: WidgetId = WidgetId(unsafe { NonZeroU64::new_unchecked(USE_BITS) });

    const INVALID: WidgetId = WidgetId(unsafe { NonZeroU64::new_unchecked(MASK_REST) });

    /// Returns true if `self` equals `id` or if `id` is a descendant of `self`
    pub fn is_ancestor_of(self, id: Self) -> bool {
        let self_id = self.0.get();
        let child_id = id.0.get();
        if (child_id & USE_BITS) != 0 {
            let self_blocks = block_len(self_id);
            let child_blocks = block_len(child_id);
            if (self_id & USE_BITS) == 0 || self_blocks > child_blocks {
                return false;
            }

            let shift = 4 * (BLOCKS - self_blocks);
            return (self_id & MASK_REST) >> shift == (child_id & MASK_REST) >> shift;
        }

        if (child_id & USE_DB) == 0 {
            return false;
        }

        let db = DB.lock().unwrap();
        let child_i = usize::conv(child_id & MASK_REST);

        if (self_id & USE_BITS) != 0 {
            let iter = BitsIter::new(self_id);
            iter.zip(db[child_i].iter()).all(|(a, b)| a == *b)
        } else if (self_id & USE_DB) != 0 {
            let self_i = usize::conv(self_id & MASK_REST);
            db[child_i].starts_with(&db[self_i])
        } else {
            false
        }
    }

    /// Get index of `child` relative to `self`
    ///
    /// Returns `None` if `child` is not a descendant of `self`.
    pub fn index_of_child(self, child: Self) -> Option<usize> {
        let self_id = self.0.get();
        let child_id = child.0.get();
        if (child_id & USE_BITS) != 0 {
            let self_blocks = block_len(self_id);
            let child_blocks = block_len(child_id);
            if (self_id & USE_BITS) == 0 || self_blocks >= child_blocks {
                return None;
            }

            let shift = 4 * (BLOCKS - self_blocks);
            let child_rest = child_id & MASK_REST;
            if (self_id & MASK_REST) >> shift != child_rest >> shift {
                return None;
            }

            return Some(next_from_bits(child_rest << 8 + 4 * self_blocks).0);
        }

        if (child_id & USE_DB) == 0 {
            return None;
        }

        let db = DB.lock().unwrap();
        let child_slice = &db[usize::conv(child_id & MASK_REST)];

        if (self_id & USE_BITS) != 0 {
            let iter = BitsIter::new(self_id);
            let mut child_iter = child_slice.iter();
            if iter.zip(&mut child_iter).all(|(a, b)| a == *b) {
                child_iter.next().cloned()
            } else {
                None
            }
        } else if (self_id & USE_DB) != 0 {
            let self_slice = &db[usize::conv(self_id & MASK_REST)];
            if child_slice.starts_with(self_slice) {
                child_slice[self_slice.len()..].iter().next().cloned()
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Make an identifier for the child with the given `index`
    ///
    /// Note: this is not a getter method. Calling multiple times with the same
    /// `index` may or may not return the same value!
    pub fn make_child(self, index: usize) -> Self {
        let self_id = self.0.get();
        let mut path = None;
        if (self_id & USE_BITS) != 0 {
            // TODO(opt): this bit-packing approach is designed for space-optimisation, but it may
            // be better to use a simpler, less-compressed approach, possibly with u128 type.
            let block_len = block_len(self_id);
            let avail_blocks = BLOCKS - block_len;
            let req_bits = 8 * size_of::<usize>() as u8 - index.leading_zeros() as u8;
            if req_bits <= 3 * avail_blocks {
                let mut x = index as u64;
                let mut y = x & 7;
                x >>= 3;
                let mut shift = 4;
                while x != 0 {
                    y |= (8 | (x & 7)) << shift;
                    x >>= 3;
                    shift += 4;
                }
                // Note: zero is encoded with 1 block to force bump to len
                let used_blocks = shift / 4;
                debug_assert_eq!(used_blocks, ((req_bits + 2) / 3).max(1));
                let len = (block_len as u64 + used_blocks as u64) << SHIFT_LEN;
                let rest = y << 4 * avail_blocks - shift;
                let id = USE_BITS | len | (self_id & MASK_REST) | rest;
                return WidgetId(NonZeroU64::new(id).unwrap());
            } else {
                path = Some(BitsIter::new(self_id).chain(once(index)).collect());
            }
        }

        if (self_id & (USE_BITS | USE_DB)) == 0 {
            panic!("WidgetId::make_child: cannot make child of {}", self);
        }

        let mut db = DB.lock().unwrap();

        let path = path.unwrap_or_else(|| {
            let i = usize::conv(self_id & MASK_REST);
            db[i].iter().cloned().chain(once(index)).collect()
        });

        let id = u64::conv(db.len());
        // We can quite safely assume this:
        debug_assert_eq!(id & MASK_HEAD, 0);
        let id = id & MASK_REST;

        db.push(path);

        WidgetId(NonZeroU64::new(USE_DB | id).unwrap())
    }

    /// Convert `Option<WidgetId>` to `u64`
    pub fn opt_to_u64(id: Option<WidgetId>) -> u64 {
        match id {
            None => 0,
            Some(id) => id.into(),
        }
    }

    /// Convert `u64` to `Option<WidgetId>`
    ///
    /// This always "succeeds", though the result may not identify any widget.
    pub fn opt_from_u64(n: u64) -> Option<WidgetId> {
        NonZeroU64::new(n).map(|nz| WidgetId(nz))
    }
}

impl std::cmp::PartialEq for WidgetId {
    fn eq(&self, rhs: &Self) -> bool {
        let self_id = self.0.get();
        let rhs_id = rhs.0.get();
        match (self_id & MASK_HEAD, rhs_id & MASK_HEAD) {
            (USE_DB, USE_DB) => {
                let db = DB.lock().unwrap();

                let self_i = usize::conv(self_id & MASK_REST);
                let child_i = usize::conv(rhs_id & MASK_REST);
                db[self_i] == db[child_i]
            }
            _ => self_id == rhs_id,
        }
    }
}

impl From<WidgetId> for u64 {
    #[inline]
    fn from(id: WidgetId) -> u64 {
        id.0.get().into()
    }
}

impl Default for WidgetId {
    fn default() -> Self {
        WidgetId::INVALID
    }
}

impl fmt::Debug for WidgetId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "WidgetId({})", self)
    }
}

impl fmt::Display for WidgetId {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let self_id = self.0.get();
        match self_id & MASK_HEAD {
            USE_BITS => {
                let len = block_len(self_id);
                if len == 0 {
                    write!(f, "ROOT")
                } else {
                    let bits = (self_id & MASK_REST) >> (4 * (BLOCKS - len));
                    write!(f, "BITS#{1:0>0$x}", len as usize, bits)
                }
            }
            // TODO: encode as above?
            USE_DB => write!(f, "DB#{}", self_id & MASK_REST),
            _ => write!(f, "INVALID"),
        }
    }
}

lazy_static::lazy_static! {
    static ref DB: Mutex<Vec<Vec<usize>>> = Mutex::new(vec![vec![]]);
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn size_of_option_widget_id() {
        use std::mem::size_of;
        assert_eq!(size_of::<WidgetId>(), size_of::<Option<WidgetId>>());
    }

    #[test]
    fn test_next_from_bits() {
        const REST: u64 = 0x0FFF_FFFF_FFFF_FFFF;
        assert_eq!(next_from_bits(0), (0, 1));
        assert_eq!(next_from_bits(REST), (0, 1));
        assert_eq!(next_from_bits((7 << 60) | REST), (7, 1));
        assert_eq!(next_from_bits((0xB << 60) | (3 << 56)), (27, 2));
        assert_eq!(
            next_from_bits(0xC9A4_F300_0000_0000),
            ((4 << 9) + (1 << 6) + (2 << 3) + 4, 4)
        );
    }

    #[test]
    fn text_bits_iter() {
        fn as_vec(x: u64) -> Vec<usize> {
            BitsIter::new(x).collect()
        }
        assert_eq!(as_vec(USE_BITS), vec![]);
        assert_eq!(as_vec(0x81_31_0000_0000_0000), vec![3]);
        assert_eq!(as_vec(0x87_1A_9300_7F00_0000), vec![1, 139, 0, 0, 7]);
    }

    #[test]
    fn test_make_child() {
        fn test(seq: &[usize], x: u64) {
            let mut id = WidgetId::ROOT;
            for x in seq {
                id = id.make_child(*x);
            }
            let v = u64::from(id);
            if v != x {
                panic!("test({:?}, {:x}): found {:x}", seq, x, v);
            }

            // Every id is its own ancestor:
            assert!(id.is_ancestor_of(id));
        }

        test(&[], USE_BITS);
        test(&[0, 0, 0], USE_BITS | (3 << 56));
        test(&[0, 1, 0], USE_BITS | (3 << 56) | (1 << 48));
        test(&[9, 0, 1, 300], 0x879101cd40000000);
        test(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 0], 0x8e12345679091920);
    }

    #[test]
    fn test_display() {
        fn from_seq(seq: &[usize]) -> String {
            let mut id = WidgetId::ROOT;
            for x in seq {
                id = id.make_child(*x);
            }
            format!("{}", id)
        }

        assert_eq!(from_seq(&[]), "ROOT");
        assert_eq!(from_seq(&[0]), "BITS#0");
        assert_eq!(from_seq(&[1, 2, 3]), "BITS#123");
        assert_eq!(from_seq(&[5, 9, 13]), "BITS#59195");
        assert_eq!(from_seq(&[321]), "BITS#d81");
        assert!(from_seq(&[313553, 13513, 13511631]).starts_with("DB#"));
    }

    #[test]
    fn test_is_ancestor() {
        fn test(seq: &[usize], seq2: &[usize]) {
            let mut id = WidgetId::ROOT;
            for x in seq {
                id = id.make_child(*x);
            }
            println!("id={} val={:x} from {:?}", id, u64::from(id), seq);
            let mut id2 = id;
            for x in seq2 {
                id2 = id2.make_child(*x);
            }
            println!("id2={} val={:x} from {:?}", id2, u64::from(id2), seq2);
            let next = seq2.iter().next().cloned();
            assert_eq!(id.index_of_child(id2), next);
            assert_eq!(id.is_ancestor_of(id2), next.is_some() || id == id2);
        }

        test(&[], &[]);
        test(&[], &[1]);
        test(&[], &[51930, 2, 18]);
        test(&[5, 6, 0, 1, 1], &[]);
        test(&[5, 6, 0, 1, 1], &[1, 1]);
        test(&[8, 26], &[0]);
        test(&[9, 9, 9, 9, 9, 9, 9], &[]);
        test(&[9, 9, 9, 9, 9, 9, 9], &[6]);
        test(&[9, 9, 9, 9, 9, 9, 9, 9], &[3]);
        test(&[0, 2, 2, 0, 17], &[0]);
    }

    #[test]
    fn test_not_ancestor() {
        fn test(seq: &[usize], seq2: &[usize]) {
            let mut id = WidgetId::ROOT;
            for x in seq {
                id = id.make_child(*x);
            }
            println!("id={} val={:x} from {:?}", id, u64::from(id), seq);
            let mut id2 = WidgetId::ROOT;
            for x in seq2 {
                id2 = id2.make_child(*x);
            }
            println!("id2={} val={:x} from {:?}", id2, u64::from(id2), seq2);
            assert_eq!(id.index_of_child(id2), None);
            assert_eq!(id.is_ancestor_of(id2), false);
        }

        test(&[0], &[]);
        test(&[2, 10, 1], &[2, 10]);
    }
}
