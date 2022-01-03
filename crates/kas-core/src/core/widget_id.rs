// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget identifiers

// x << a + b is x << (a + b)
#![allow(clippy::precedence)]

use crate::cast::{Cast, Conv};
use std::cmp::PartialEq;
use std::fmt;
use std::hash::{Hash, Hasher};
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
#[derive(Clone, Copy, Eq)]
pub struct WidgetId(NonZeroU64);

/// Invalid (default) identifier
const INVALID: u64 = !0;

/// `x & USE_BITS != 0`: rest is a sequence of 4-bit blocks; len is number of blocks used
const USE_BITS: u64 = 0x8000_0000_0000_0000;

const MASK_LEN: u64 = 0x0F00_0000_0000_0000;
const SHIFT_LEN: u8 = 56;
const BLOCKS: u8 = 14;
const MASK_BITS: u64 = 0x00FF_FFFF_FFFF_FFFF;

const MASK_PTR: u64 = 0x7FFF_FFFF_FFFF_FFFF;

fn encode(index: usize) -> (u64, u8) {
    debug_assert!(8 * size_of::<usize>() as u32 - index.leading_zeros() <= 64);
    let mut x = index as u64 & 0x0000_FFFF_FFFF_FFFF;
    let mut y = x & 7;
    x >>= 3;
    let mut shift = 4;
    while x != 0 {
        y |= (8 | (x & 7)) << shift;
        x >>= 3;
        shift += 4;
    }
    (y, shift)
}

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
    (y.cast(), c)
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

    const INVALID: WidgetId = WidgetId(unsafe { NonZeroU64::new_unchecked(INVALID) });

    /// Is the identifier valid?
    ///
    /// Default-constructed identifiers are invalid. Comparing invalid ids is
    /// considered a logic error and thus will panic in debug builds.
    /// This method may be used to check an identifier's validity.
    pub fn is_valid(self) -> bool {
        self.0.get() != !0
    }

    /// Returns true if `self` equals `id` or if `id` is a descendant of `self`
    pub fn is_ancestor_of(self, id: Self) -> bool {
        let self_id = self.0.get();
        let child_id = id.0.get();
        if (child_id & USE_BITS) != 0 {
            let self_blocks = block_len(self_id);
            let child_blocks = block_len(child_id);
            if self_id == !0 || child_id == !0 {
                return false; // invalid
            }
            if (self_id & USE_BITS) == 0 || self_blocks > child_blocks {
                return false; // assumption: parent uses bits when child does
            }

            let shift = 4 * (BLOCKS - self_blocks);
            return (self_id & MASK_BITS) >> shift == (child_id & MASK_BITS) >> shift;
        }

        let db = DB.lock().unwrap();
        let child_i = usize::conv(child_id & MASK_PTR);

        if (self_id & USE_BITS) != 0 {
            let iter = BitsIter::new(self_id);
            iter.zip(db[child_i].iter()).all(|(a, b)| a == *b)
        } else {
            let self_i = usize::conv(self_id & MASK_PTR);
            db[child_i].starts_with(&db[self_i])
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
            if self_id == !0 || child_id == !0 {
                return None; // invalid
            }
            if (self_id & USE_BITS) == 0 || self_blocks >= child_blocks {
                return None;
            }

            let shift = 4 * (BLOCKS - self_blocks);
            let child_rest = child_id & MASK_BITS;
            if (self_id & MASK_BITS) >> shift != child_rest >> shift {
                return None;
            }

            return Some(next_from_bits(child_rest << 8 + 4 * self_blocks).0);
        }

        let db = DB.lock().unwrap();
        let child_slice = &db[usize::conv(child_id & MASK_PTR)];

        if (self_id & USE_BITS) != 0 {
            let iter = BitsIter::new(self_id);
            let mut child_iter = child_slice.iter();
            if iter.zip(&mut child_iter).all(|(a, b)| a == *b) {
                child_iter.next().cloned()
            } else {
                None
            }
        } else {
            let self_slice = &db[usize::conv(self_id & MASK_PTR)];
            if child_slice.starts_with(self_slice) {
                child_slice[self_slice.len()..].iter().next().cloned()
            } else {
                None
            }
        }
    }

    /// Make an identifier for the child with the given `index`
    ///
    /// Note: this is not a getter method. Calling multiple times with the same
    /// `index` may or may not return the same value!
    #[must_use]
    pub fn make_child(self, index: usize) -> Self {
        let self_id = self.0.get();
        let mut path = None;
        if (self_id & USE_BITS) != 0 {
            if self_id == !0 {
                panic!("WidgetId::make_child: invalid id");
            }

            // TODO(opt): this bit-packing approach is designed for space-optimisation, but it may
            // be better to use a simpler, less-compressed approach, possibly with u128 type.
            let block_len = block_len(self_id);
            let avail_blocks = BLOCKS - block_len;
            let req_bits = 8 * size_of::<usize>() as u8 - index.leading_zeros() as u8;
            if req_bits <= 3 * avail_blocks {
                let (bits, bit_len) = encode(index);
                // Note: zero is encoded with 1 block to force bump to len
                let used_blocks = bit_len / 4;
                debug_assert_eq!(used_blocks, ((req_bits + 2) / 3).max(1));
                let len = (block_len as u64 + used_blocks as u64) << SHIFT_LEN;
                let rest = bits << 4 * avail_blocks - bit_len;
                let id = USE_BITS | len | (self_id & MASK_BITS) | rest;
                return WidgetId(NonZeroU64::new(id).unwrap());
            } else {
                path = Some(BitsIter::new(self_id).chain(once(index)).collect());
            }
        }

        let mut db = DB.lock().unwrap();

        let path = path.unwrap_or_else(|| {
            let i = usize::conv(self_id & MASK_PTR);
            db[i].iter().cloned().chain(once(index)).collect()
        });

        let id = u64::conv(db.len());
        // We can quite safely assume this:
        debug_assert_eq!(id & USE_BITS, 0);
        let id = id & MASK_PTR;

        db.push(path);

        WidgetId(NonZeroU64::new(id).unwrap())
    }

    /// Convert to a `u64`
    ///
    /// This value should not be interpreted, except as follows:
    ///
    /// -   it is guaranteed non-zero
    /// -   it may be passed to [`Self::opt_from_u64`]
    pub fn as_u64(&self) -> u64 {
        self.0.get()
    }

    /// Convert `Option<WidgetId>` to `u64`
    ///
    /// This value should not be interpreted, except as follows:
    ///
    /// -   it is zero if and only if `id == None`
    /// -   it may be passed to [`Self::opt_from_u64`]
    pub fn opt_to_u64(id: Option<&WidgetId>) -> u64 {
        match id {
            None => 0,
            Some(id) => id.as_u64(),
        }
    }

    /// Convert `u64` to `Option<WidgetId>`
    ///
    /// This always "succeeds", though the result may not identify any widget.
    pub fn opt_from_u64(n: u64) -> Option<WidgetId> {
        NonZeroU64::new(n).map(WidgetId)
    }
}

impl PartialEq for WidgetId {
    fn eq(&self, rhs: &Self) -> bool {
        let lhs = self.0.get();
        let rhs = rhs.0.get();

        if lhs == !0 || rhs == !0 {
            panic!("WidgetId::eq: invalid id");
        }

        if lhs == rhs {
            return true;
        }

        let use_bits = (lhs & USE_BITS, rhs & USE_BITS);
        if use_bits.0 != 0 && use_bits.1 != 0 {
            return false;
        }

        let db = DB.lock().unwrap();

        let (mut lbi, mut rbi);
        let (mut lvi, mut rvi);
        let lpath: &mut dyn Iterator<Item = usize> = if use_bits.0 != 0 {
            lbi = BitsIter::new(lhs);
            &mut lbi
        } else {
            lvi = db[usize::conv(lhs & MASK_PTR)].iter().cloned();
            &mut lvi
        };
        let rpath: &mut dyn Iterator<Item = usize> = if use_bits.1 != 0 {
            rbi = BitsIter::new(rhs);
            &mut rbi
        } else {
            rvi = db[usize::conv(rhs & MASK_PTR)].iter().cloned();
            &mut rvi
        };

        lpath.eq(rpath)
    }
}

impl PartialEq<Option<WidgetId>> for WidgetId {
    #[inline]
    fn eq(&self, rhs: &Option<WidgetId>) -> bool {
        rhs.map(|id| id == *self).unwrap_or(false)
    }
}

impl<'a> PartialEq<Option<&'a WidgetId>> for WidgetId {
    #[inline]
    fn eq(&self, rhs: &Option<&'a WidgetId>) -> bool {
        rhs.map(|id| id == self).unwrap_or(false)
    }
}

impl Hash for WidgetId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let x = self.0.get();
        if x & USE_BITS != 0 {
            // Assuming the USE_BITS representation is used whenever possible
            // (true outside of tests), we can simply hash the bit value.
            // (Otherwise we must use BitsIter, handling INVALID and ROOT as special cases.)
            x.hash(state);
        } else {
            let db = DB.lock().unwrap();
            for index in db[usize::conv(x & MASK_PTR)].iter() {
                index.hash(state);
            }
        }
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
        if self_id == !0 {
            write!(f, "#INVALID")
        } else if self_id & USE_BITS != 0 {
            let len = block_len(self_id);
            if len == 0 {
                write!(f, "#")
            } else {
                let bits = (self_id & MASK_BITS) >> (4 * (BLOCKS - len));
                write!(f, "#{1:0>0$x}", len as usize, bits)
            }
        } else {
            write!(f, "#")?;
            let db = DB.lock().unwrap();
            let seq = &db[usize::conv(self_id & MASK_PTR)];
            for index in seq {
                let (bits, bit_len) = encode(*index);
                write!(f, "{1:0>0$x}", bit_len as usize / 4, bits)?;
            }
            Ok(())
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
    fn test_partial_eq() {
        assert_eq!(WidgetId::ROOT, WidgetId::ROOT);
        let c1 = WidgetId::ROOT.make_child(0).make_child(15);
        let c2 = WidgetId::ROOT.make_child(1).make_child(15);
        let c3 = WidgetId::ROOT.make_child(0).make_child(14);
        let c4 = WidgetId::ROOT.make_child(0).make_child(15);
        println!("c1: {}", c1);
        assert!(c1 != c2);
        assert!(c1 != c3);
        assert!(c2 != c3);
        assert_eq!(c1, c4);
        assert!(c1 != WidgetId::ROOT);

        fn make_db(v: Vec<usize>) -> WidgetId {
            let mut db = DB.lock().unwrap();
            let id = u64::conv(db.len());
            let id = id & MASK_PTR;
            db.push(v);
            WidgetId(NonZeroU64::new(id).unwrap())
        }
        let d1 = make_db(vec![0, 15]);
        let d2 = make_db(vec![1, 15]);
        assert_eq!(c1, d1);
        assert_eq!(c2, d2);
        assert!(d1 != d2);
        assert!(d1 != WidgetId::ROOT);
    }

    #[test]
    #[should_panic]
    fn test_partial_eq_invalid_1() {
        assert_eq!(WidgetId::INVALID, WidgetId::INVALID);
    }

    #[test]
    #[should_panic]
    fn test_partial_eq_invalid_2() {
        assert_eq!(WidgetId::ROOT, WidgetId::INVALID);
    }

    #[test]
    #[should_panic]
    fn test_partial_eq_invalid_3() {
        assert_eq!(WidgetId::INVALID, WidgetId::ROOT);
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
        assert_eq!(as_vec(USE_BITS), Vec::<usize>::new());
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
            let v = id.as_u64();
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

        assert_eq!(from_seq(&[]), "#");
        assert_eq!(from_seq(&[0]), "#0");
        assert_eq!(from_seq(&[1, 2, 3]), "#123");
        assert_eq!(from_seq(&[5, 9, 13]), "#59195");
        assert_eq!(from_seq(&[321]), "#d81");
        assert_eq!(
            from_seq(&[313553, 13513, 13511631]),
            "#99ccba1bab91ebcadf97"
        )
    }

    #[test]
    fn test_is_ancestor() {
        fn test(seq: &[usize], seq2: &[usize]) {
            let mut id = WidgetId::ROOT;
            for x in seq {
                id = id.make_child(*x);
            }
            println!("id={} val={:x} from {:?}", id, id.as_u64(), seq);
            let mut id2 = id;
            for x in seq2 {
                id2 = id2.make_child(*x);
            }
            println!("id2={} val={:x} from {:?}", id2, id2.as_u64(), seq2);
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
            println!("id={} val={:x} from {:?}", id, id.as_u64(), seq);
            let mut id2 = WidgetId::ROOT;
            for x in seq2 {
                id2 = id2.make_child(*x);
            }
            println!("id2={} val={:x} from {:?}", id2, id2.as_u64(), seq2);
            assert_eq!(id.index_of_child(id2), None);
            assert_eq!(id.is_ancestor_of(id2), false);
        }

        test(&[0], &[]);
        test(&[2, 10, 1], &[2, 10]);
    }
}
