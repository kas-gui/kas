// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget identifiers

// x << a + b is x << (a + b)
#![allow(clippy::precedence)]

use crate::cast::{Cast, Conv};
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::iter::once;
use std::marker::PhantomData;
use std::mem::size_of;
use std::num::NonZeroU64;
use std::rc::Rc;
use std::{fmt, slice};

/// Invalid (default) identifier
const INVALID: u64 = !0;

/// `x & USE_BITS != 0`: rest is a sequence of 4-bit blocks; len is number of blocks used
const USE_BITS: u64 = 0x01;
/// Combination of all flag bits (must be zero for pointers)
const FLAG_BITS: u64 = 0x03;

const MASK_LEN: u64 = 0xF0;
const SHIFT_LEN: u8 = 4;
const BLOCKS: u8 = 14;
const MASK_BITS: u64 = 0xFFFF_FFFF_FFFF_FF00;

/// Integer or pointer to a reference-counted slice.
///
/// Use `Self::get_ptr` to determine the variant used.
///
/// `self.0 & USE_BITS` is the "flag bit" determining the variant used. This
/// overlaps with the pointer's lowest bit (which must be zero due to alignment).
///
/// `PhantomData<Rc<()>>` is used to impl !Send and !Sync. We need atomic
/// reference counting to support those.
struct IntOrPtr(NonZeroU64, PhantomData<Rc<()>>);

#[derive(Clone, Debug, PartialEq, Eq)]
enum Variant<'a> {
    Invalid,
    Int(u64),
    Slice(&'a [usize]),
}

impl IntOrPtr {
    const ROOT: Self = IntOrPtr(unsafe { NonZeroU64::new_unchecked(USE_BITS) }, PhantomData);
    const INVALID: Self = IntOrPtr(unsafe { NonZeroU64::new_unchecked(INVALID) }, PhantomData);

    #[inline]
    fn get_ptr(&self) -> Option<*mut usize> {
        if self.0.get() & FLAG_BITS == 0 {
            let p = usize::conv(self.0.get());
            Some(p as *mut usize)
        } else {
            None
        }
    }

    /// Construct from an integer
    ///
    /// Note: requires `x & FLAG_BITS != 0`.
    fn new_int(x: u64) -> Self {
        debug_assert!(x & FLAG_BITS != 0);
        let x = NonZeroU64::new(x).unwrap();
        let u = IntOrPtr(x, PhantomData);
        debug_assert!(u.get_ptr().is_none());
        u
    }

    /// Construct as a slice from an iterator
    fn new_iter<I: Clone + Iterator<Item = usize>>(iter: I) -> Self {
        let ref_count = 1;
        let len = iter.clone().count();
        let v: Vec<usize> = once(ref_count).chain(once(len)).chain(iter).collect();
        let b = v.into_boxed_slice();
        let p = Box::leak(b) as *mut [usize] as *mut usize;
        let p = u64::conv(p as usize);
        debug_assert_eq!(p & FLAG_BITS, 0);
        let u = IntOrPtr(NonZeroU64::new(p).unwrap(), PhantomData);
        debug_assert!(u.get_ptr().is_some());
        u
    }

    fn get(&self) -> Variant {
        if let Some(p) = self.get_ptr() {
            unsafe {
                let len = *p.offset(1);
                let p = p.offset(2);
                let slice = slice::from_raw_parts(p, len);
                Variant::Slice(slice)
            }
        } else if self.0.get() == INVALID {
            Variant::Invalid
        } else {
            Variant::Int(self.0.get())
        }
    }

    fn as_u64(&self) -> u64 {
        self.0.get()
    }
}

impl Clone for IntOrPtr {
    fn clone(&self) -> Self {
        if let Some(p) = self.get_ptr() {
            unsafe {
                let ref_count = *p;

                // Copy behaviour of Rc::clone:
                if ref_count == 0 || ref_count == usize::MAX {
                    std::process::abort();
                }

                *p = ref_count + 1;
            }
        }
        IntOrPtr(self.0, PhantomData)
    }
}

impl Drop for IntOrPtr {
    fn drop(&mut self) {
        if let Some(p) = self.get_ptr() {
            unsafe {
                let ref_count = *p;
                if ref_count > 1 {
                    *p = ref_count - 1;
                } else {
                    // len+2 because path len does not include ref_count or len "fields"
                    let len = *p.offset(1) + 2;
                    let slice = slice::from_raw_parts_mut(p, len);
                    let _ = Box::<[usize]>::from_raw(slice);
                }
            }
        }
    }
}

impl PartialEq for IntOrPtr {
    fn eq(&self, rhs: &Self) -> bool {
        let (x, y) = (self.0.get(), rhs.0.get());
        if x == INVALID || y == INVALID {
            panic!("OwnedId::eq: invalid id");
        } else if x == y {
            // No need to unpack variants when values are bit-equal
            return true;
        }
        match (self.get(), rhs.get()) {
            (Variant::Int(_), Variant::Int(_)) => false,
            (Variant::Slice(l), Variant::Slice(r)) => l == r,
            (Variant::Int(x), Variant::Slice(path)) | (Variant::Slice(path), Variant::Int(x)) => {
                let p = WidgetPathIter(PathIter::Bits(BitsIter::new(x)));
                let q = WidgetPathIter(PathIter::Slice(path.iter().cloned()));
                p.eq(q)
            }
            _ => unreachable!(),
        }
    }
}
impl Eq for IntOrPtr {}

enum PathIter<'a> {
    Bits(BitsIter),
    Slice(std::iter::Cloned<std::slice::Iter<'a, usize>>),
}

impl<'a> Iterator for PathIter<'a> {
    type Item = usize;

    #[inline]
    fn next(&mut self) -> Option<usize> {
        match self {
            PathIter::Bits(bits) => bits.next(),
            PathIter::Slice(slice) => slice.next(),
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            PathIter::Bits(bits) => bits.size_hint(),
            PathIter::Slice(slice) => slice.size_hint(),
        }
    }
}

/// Iterator over [`OwnedId`] path components
pub struct WidgetPathIter<'a>(PathIter<'a>);
impl<'a> Iterator for WidgetPathIter<'a> {
    type Item = usize;

    #[inline]
    fn next(&mut self) -> Option<usize> {
        self.0.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

// Encode lowest 48 bits of key into the low bits of a u64, returning also the encoded bit-length
fn encode(key: usize) -> (u64, u8) {
    debug_assert!(8 * size_of::<usize>() as u32 - key.leading_zeros() <= 64);
    let mut x = key as u64 & 0x0000_FFFF_FFFF_FFFF;
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

// Returns usize read from highest bits of x plus blocks used
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

#[derive(Clone, Debug)]
struct BitsIter(u8, u64);
impl BitsIter {
    fn new(bits: u64) -> Self {
        assert!((bits & USE_BITS) != 0);
        let len = block_len(bits);
        BitsIter(len, bits & MASK_BITS)
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

/// An owning widget identifier
///
/// All widgets are assigned a unique identifier, using a path. This type is
/// small (64-bit and non-zero), sometimes allocated (longer paths only),
/// and cheap to copy (using reference counting).
/// It is neither [`Send`] nor [`Sync`]. See also [`WidgetId`].
///
/// Formatting an `OwnedId` via [`Display`] prints the the path, for example
/// `#1290a4`. Here, `#` represents the root; each following hexadecimal digit
/// represents a path component except that digits `8-f` are combined with the
/// following digit(s). Hence, the above path has components `1`, `2`, `90`,
/// `a4`. To interpret these values, first subtract 8 from each digit but the
/// last digit, then read as base-8: `[1, 2, 8, 20]`.
///
/// [`Display`]: std::fmt::Display
#[derive(Clone, PartialEq, Eq)]
pub struct OwnedId(IntOrPtr);

impl OwnedId {
    /// Identifier of the window
    pub(crate) const ROOT: Self = OwnedId(IntOrPtr::ROOT);

    const INVALID: Self = OwnedId(IntOrPtr::INVALID);

    /// Is the identifier valid?
    ///
    /// Default-constructed identifiers are invalid. Comparing invalid ids is
    /// considered a logic error and thus will panic in debug builds.
    /// This method may be used to check an identifier's validity.
    pub fn is_valid(&self) -> bool {
        self.0.get() != Variant::Invalid
    }

    /// Iterate over path components
    pub fn iter(&self) -> WidgetPathIter {
        match self.0.get() {
            Variant::Invalid => panic!("OwnedId::iter: invalid"),
            Variant::Int(x) => WidgetPathIter(PathIter::Bits(BitsIter::new(x))),
            Variant::Slice(path) => WidgetPathIter(PathIter::Slice(path.iter().cloned())),
        }
    }

    /// Get the path len (number of indices, not storage length)
    pub fn path_len(&self) -> usize {
        match self.0.get() {
            Variant::Invalid => panic!("OwnedId::path_len: invalid"),
            Variant::Int(x) => BitsIter::new(x).count(),
            Variant::Slice(path) => path.len(),
        }
    }

    /// Returns true if `self` equals `id` or if `id` is a descendant of `self`
    pub fn is_ancestor_of(&self, id: &Self) -> bool {
        match (self.0.get(), id.0.get()) {
            (Variant::Invalid, _) | (_, Variant::Invalid) => {
                panic!("OwnedId::is_ancestor_of: invalid")
            }
            (Variant::Slice(_), Variant::Int(_)) => {
                // This combo will never be created where id is a child.
                false
            }
            (Variant::Int(self_x), Variant::Int(child_x)) => {
                let self_blocks = block_len(self_x);
                let child_blocks = block_len(child_x);
                if self_blocks > child_blocks {
                    return false;
                }

                // self_blocks == 0 for ROOT, otherwise > 0
                let shift = 4 * (BLOCKS - self_blocks) + 8;
                shift == 64 || self_x >> shift == child_x >> shift
            }
            (Variant::Int(self_x), Variant::Slice(child)) => {
                let iter = BitsIter::new(self_x);
                iter.zip(child.iter()).all(|(a, b)| a == *b)
            }
            (Variant::Slice(self_path), Variant::Slice(child)) => child.starts_with(self_path),
        }
    }

    pub fn iter_keys_after(&self, id: &Self) -> WidgetPathIter {
        let mut self_iter = self.iter();
        for v in id.iter() {
            if self_iter.next() != Some(v) {
                return WidgetPathIter(PathIter::Bits(BitsIter(0, 0)));
            }
        }
        self_iter
    }

    /// Get first key in path of `self` path after `id`
    ///
    /// If the path of `self` starts with the path of `id`
    /// (`id.is_ancestor_of(self)`) then this returns the *next* key in
    /// `self`'s path (if any). Otherwise, this returns `None`.
    pub fn next_key_after(&self, id: &Self) -> Option<usize> {
        match (id.0.get(), self.0.get()) {
            (Variant::Invalid, _) | (_, Variant::Invalid) => {
                panic!("OwnedId::next_key_after: invalid")
            }
            (Variant::Slice(_), Variant::Int(_)) => None,
            (Variant::Int(parent_x), Variant::Int(child_x)) => {
                let parent_blocks = block_len(parent_x);
                let child_blocks = block_len(child_x);
                if parent_blocks >= child_blocks {
                    return None;
                }

                // parent_blocks == 0 for ROOT, otherwise > 0
                let shift = 4 * (BLOCKS - parent_blocks) + 8;
                if shift != 64 && parent_x >> shift != child_x >> shift {
                    return None;
                }

                debug_assert!(child_blocks > 0);
                let next_bits = (child_x & MASK_BITS) << (4 * parent_blocks);
                Some(next_from_bits(next_bits).0)
            }
            (Variant::Int(parent_x), Variant::Slice(child_path)) => {
                let iter = BitsIter::new(parent_x);
                let mut child_iter = child_path.iter();
                if iter.zip(&mut child_iter).all(|(a, b)| a == *b) {
                    child_iter.next().cloned()
                } else {
                    None
                }
            }
            (Variant::Slice(parent_path), Variant::Slice(child_path)) => {
                if child_path.starts_with(parent_path) {
                    child_path[parent_path.len()..].iter().next().cloned()
                } else {
                    None
                }
            }
        }
    }

    /// Get the parent widget's identifier, if not root
    ///
    /// Note: there is no guarantee that [`Self::as_u64`] on the result will
    /// match that of the original parent identifier.
    pub fn parent(&self) -> Option<OwnedId> {
        match self.0.get() {
            Variant::Invalid => None,
            Variant::Int(x) => {
                let mut bit_len = 4 * block_len(x);
                while bit_len > 0 {
                    bit_len -= 4;
                    if bit_len > 0 && (x >> (64 - bit_len)) & 8 != 0 {
                        continue;
                    }

                    let len = ((bit_len / 4) as u64) << SHIFT_LEN;
                    let mask = MASK_BITS << (56 - bit_len);
                    let id = (x & mask) | len | USE_BITS;
                    return Some(OwnedId(IntOrPtr::new_int(id)));
                }
                None
            }
            Variant::Slice(path) => {
                let len = path.len();
                if len > 1 {
                    // TODO(opt): in some cases we could make Variant::Int
                    Some(OwnedId(IntOrPtr::new_iter(
                        path[0..len - 1].iter().cloned(),
                    )))
                } else {
                    None
                }
            }
        }
    }

    /// Make an identifier for the child with the given `key`
    ///
    /// Note: this is not a getter method. Calling multiple times with the same
    /// `key` may or may not return the same value!
    #[must_use]
    pub fn make_child(&self, key: usize) -> Self {
        match self.0.get() {
            Variant::Invalid => panic!("OwnedId::make_child: invalid"),
            Variant::Int(self_x) => {
                // TODO(opt): this bit-packing approach is designed for space-optimisation, but it may
                // be better to use a simpler, less-compressed approach, possibly with u128 type.
                let block_len = block_len(self_x);
                let avail_blocks = BLOCKS - block_len;
                // Note: zero is encoded with 1 block to force bump to len
                let req_bits = (8 * size_of::<usize>() as u8 - key.leading_zeros() as u8).max(1);
                if req_bits <= 3 * avail_blocks {
                    let (bits, bit_len) = encode(key);
                    let used_blocks = bit_len / 4;
                    debug_assert_eq!(used_blocks, (req_bits + 2) / 3);
                    let len = (block_len as u64 + used_blocks as u64) << SHIFT_LEN;
                    let rest = bits << 4 * avail_blocks - bit_len + 8;
                    let id = (self_x & MASK_BITS) | rest | len | USE_BITS;
                    OwnedId(IntOrPtr::new_int(id))
                } else {
                    OwnedId(IntOrPtr::new_iter(BitsIter::new(self_x).chain(once(key))))
                }
            }
            Variant::Slice(path) => {
                OwnedId(IntOrPtr::new_iter(path.iter().cloned().chain(once(key))))
            }
        }
    }

    /// Convert to a `u64`
    ///
    /// This `u64` is non-zero and may be used as an equality check.
    /// Caveat: if two separate [`OwnedId`]s are allocated for the same
    /// (longer) path, it is possible that the values derived from them via this
    /// method will compare unequal. This may be a problem for view widgets
    /// of e.g. `ListView` since widget identifiers are re-assigned during
    /// scrolling.
    pub fn as_u64(&self) -> u64 {
        self.0.as_u64()
    }
}

/// A non-owning widget identifier
///
/// Like [`OwnedId`], this uniquely identifies a widget, but making some
/// compromises in order to support [`Copy`], [`Send`] and [`Sync`]:
///
/// -   There is a chance that equality tests will result in a false positive.
///     This should be rare and only occur when the identifier is *not* used in
///     [`EventState`]. (This requires that an [`OwnedId`] `a` is
///     constructed for some longer path `p`, a [`WidgetId`] `b` is consrtucted
///     from `a`, `a` is freed, then a new [`OwnedId`] `c` is constructed from
///     the same path `p`.)
/// -   The path may not be available. As above, this should be rare and may
///     only occur when the identifier is not used by [`EventState`].
///
/// [`EventState`]: crate::event::EventState
pub type WidgetId = OwnedId;

impl PartialEq<str> for OwnedId {
    fn eq(&self, rhs: &str) -> bool {
        // TODO(opt): we don't have to format to test this
        let formatted = format!("{self}");
        formatted == rhs
    }
}
impl PartialEq<&str> for OwnedId {
    fn eq(&self, rhs: &&str) -> bool {
        self == *rhs
    }
}

impl PartialOrd for OwnedId {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        Some(self.cmp(rhs))
    }
}

impl Ord for OwnedId {
    fn cmp(&self, rhs: &Self) -> Ordering {
        match (self.0.get(), rhs.0.get()) {
            (Variant::Invalid, _) | (_, Variant::Invalid) => panic!("OwnedId::cmp: invalid id"),
            (Variant::Int(x), Variant::Int(y)) => x.cmp(&y),
            _ => self.iter().cmp(rhs.iter()),
        }
    }
}

impl Hash for OwnedId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self.0.get() {
            Variant::Invalid => (),
            Variant::Int(x) => {
                x.hash(state);
            }
            Variant::Slice(path) => {
                path.hash(state);
            }
        }
    }
}

impl PartialEq<Option<OwnedId>> for OwnedId {
    #[inline]
    fn eq(&self, rhs: &Option<OwnedId>) -> bool {
        rhs.as_ref().map(|id| id == self).unwrap_or(false)
    }
}

impl<'a> PartialEq<Option<&'a OwnedId>> for OwnedId {
    #[inline]
    fn eq(&self, rhs: &Option<&'a OwnedId>) -> bool {
        rhs.map(|id| id == self).unwrap_or(false)
    }
}

impl<'a> PartialEq<&'a OwnedId> for OwnedId {
    #[inline]
    fn eq(&self, rhs: &&OwnedId) -> bool {
        self == *rhs
    }
}

impl<'a> PartialEq<&'a Option<OwnedId>> for OwnedId {
    #[inline]
    fn eq(&self, rhs: &&Option<OwnedId>) -> bool {
        self == *rhs
    }
}

impl Default for OwnedId {
    fn default() -> Self {
        OwnedId::INVALID
    }
}

impl fmt::Debug for OwnedId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "OwnedId({self})")
    }
}

impl fmt::Display for OwnedId {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self.0.get() {
            Variant::Invalid => write!(f, "#INVALID"),
            Variant::Int(x) => {
                let len = block_len(x);
                if len == 0 {
                    write!(f, "#")
                } else {
                    let bits = x >> (4 * (BLOCKS - len) + 8);
                    write!(f, "#{1:0>0$x}", len as usize, bits)
                }
            }
            Variant::Slice(path) => {
                write!(f, "#")?;
                for key in path {
                    let (bits, bit_len) = encode(*key);
                    write!(f, "{1:0>0$x}", bit_len as usize / 4, bits)?;
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn size_of_option_widget_id() {
        use std::mem::size_of;
        assert_eq!(size_of::<OwnedId>(), 8);
        assert_eq!(size_of::<OwnedId>(), size_of::<Option<OwnedId>>());
    }

    #[test]
    fn test_partial_eq() {
        assert_eq!(OwnedId::ROOT, OwnedId::ROOT);
        let c1 = OwnedId::ROOT.make_child(0).make_child(15);
        let c2 = OwnedId::ROOT.make_child(1).make_child(15);
        let c3 = OwnedId::ROOT.make_child(0).make_child(14);
        let c4 = OwnedId::ROOT.make_child(0).make_child(15);
        assert!(c1 != c2);
        assert!(c1 != c3);
        assert!(c2 != c3);
        assert_eq!(c1, c4);
        assert!(c1 != OwnedId::ROOT);

        let d1 = OwnedId(IntOrPtr::new_iter([0, 15].iter().cloned()));
        let d2 = OwnedId(IntOrPtr::new_iter([1, 15].iter().cloned()));
        assert_eq!(c1, d1);
        assert_eq!(c2, d2);
        assert!(d1 != d2);
        assert!(d1 != OwnedId::ROOT);
    }

    #[test]
    #[should_panic]
    fn test_partial_eq_invalid_1() {
        let _ = OwnedId::INVALID == OwnedId::INVALID;
    }

    #[test]
    #[should_panic]
    fn test_partial_eq_invalid_2() {
        let _ = OwnedId::ROOT == OwnedId::INVALID;
    }

    #[test]
    #[should_panic]
    fn test_partial_eq_invalid_3() {
        let _ = OwnedId::INVALID == OwnedId::ROOT;
    }

    #[test]
    fn test_partial_eq_str() {
        assert_eq!(OwnedId::ROOT, "#");
        assert!(OwnedId::ROOT != "#0");
        assert_eq!(OwnedId::ROOT.make_child(0).make_child(15), "#097");
        assert_eq!(OwnedId::ROOT.make_child(1).make_child(15), "#197");
        let c3 = OwnedId::ROOT.make_child(0).make_child(14);
        assert_eq!(c3, "#096");
        assert!(c3 != "#097");
    }

    #[test]
    fn test_ord() {
        let root = OwnedId::ROOT;
        let c_0 = root.make_child(0);
        let c_0_0 = c_0.make_child(0);
        assert!(root < c_0);
        assert!(c_0 < c_0_0);

        let c_1 = root.make_child(1);
        assert!(c_0_0 < c_1);
        assert!(c_1 < root.make_child(8));

        let d_0 = OwnedId(IntOrPtr::new_iter([0].iter().cloned()));
        let d_0_0 = OwnedId(IntOrPtr::new_iter([0, 0].iter().cloned()));
        let d_1 = OwnedId(IntOrPtr::new_iter([1].iter().cloned()));
        assert_eq!(d_0.cmp(&c_0), Ordering::Equal);
        assert_eq!(d_0_0.cmp(&c_0_0), Ordering::Equal);
        assert_eq!(d_1.cmp(&c_1), Ordering::Equal);
        assert!(d_0 < d_0_0);
        assert!(d_0_0 < d_1);
    }

    #[test]
    #[should_panic]
    fn test_ord_invalid() {
        let _ = OwnedId::INVALID < OwnedId::ROOT;
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
        assert_eq!(as_vec(0x3100_0000_0000_0011), vec![3]);
        assert_eq!(as_vec(0x1A93_007F_0000_0071), vec![1, 139, 0, 0, 7]);
    }

    #[test]
    fn test_parent() {
        fn test(seq: &[usize]) {
            println!("seq: {seq:?}");
            let mut id = OwnedId::ROOT;
            let len = seq.len();
            for key in &seq[..len - 1] {
                id = id.make_child(*key);
            }

            if len == 0 {
                assert_eq!(id.parent(), None);
            } else {
                let id2 = id.make_child(seq[len - 1]);
                assert_eq!(id2.parent(), Some(id));
            }
        }

        test(&[4]);
        test(&[4, 0]);
        test(&[0, 0, 0]);
        test(&[0, 1, 0]);
        test(&[9, 0, 1, 300]);
        test(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 0]);
        test(&[313553, 13513, 13511631]);
    }

    #[test]
    fn test_make_child() {
        fn test(seq: &[usize], x: u64) {
            let mut id = OwnedId::ROOT;
            for key in seq {
                id = id.make_child(*key);
            }
            let v = id.as_u64();
            if v != x {
                panic!("test({seq:?}, {x:x}): found {v:x}");
            }

            // Every id is its own ancestor:
            assert!(id.is_ancestor_of(&id));
        }

        test(&[], USE_BITS);
        test(&[0, 0, 0], (3 << 4) | USE_BITS);
        test(&[0, 1, 0], (3 << 4) | (1 << 56) | USE_BITS);
        test(&[9, 0, 1, 300], 0x9101cd4000000071);
        test(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 0], 0x12345679091920e1);
    }

    #[test]
    fn test_display() {
        fn from_seq(seq: &[usize]) -> String {
            let mut id = OwnedId::ROOT;
            for x in seq {
                id = id.make_child(*x);
            }
            format!("{id}")
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
            let mut id = OwnedId::ROOT;
            for x in seq {
                id = id.make_child(*x);
            }
            println!("id={} val={:x} from {:?}", id, id.as_u64(), seq);
            let mut id2 = id.clone();
            for x in seq2 {
                id2 = id2.make_child(*x);
            }
            println!("id2={} val={:x} from {:?}", id2, id2.as_u64(), seq2);
            let next = seq2.iter().next().cloned();
            assert_eq!(id.is_ancestor_of(&id2), next.is_some() || id == id2);
            assert_eq!(id2.next_key_after(&id), next);
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
            let mut id = OwnedId::ROOT;
            for x in seq {
                id = id.make_child(*x);
            }
            println!("id={} val={:x} from {:?}", id, id.as_u64(), seq);
            let mut id2 = OwnedId::ROOT;
            for x in seq2 {
                id2 = id2.make_child(*x);
            }
            println!("id2={} val={:x} from {:?}", id2, id2.as_u64(), seq2);
            assert_eq!(id.is_ancestor_of(&id2), false);
            assert_eq!(id2.next_key_after(&id), None);
        }

        test(&[0], &[]);
        test(&[0], &[1]);
        test(&[2, 10, 1], &[2, 10]);
        test(&[0, 5, 2], &[0, 1, 5]);
    }
}
