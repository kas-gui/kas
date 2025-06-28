// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget identifiers

// x << a + b is x << (a + b)
#![allow(clippy::precedence)]

use crate::cast::{Cast, Conv};
use hash_hasher::{HashBuildHasher, HashedMap};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::iter::once;
use std::marker::PhantomData;
use std::mem::size_of;
use std::num::NonZeroU64;
use std::rc::Rc;
use std::{fmt, slice};

thread_local! {
    // Map from hash of path to allocated path
    static DB: RefCell<HashedMap<u64, *mut usize>> = RefCell::new(HashedMap::with_hasher(HashBuildHasher::new()));
}

/// Invalid (default) identifier
const INVALID: u64 = !0;

/// Mark for use; one of the below should be set (or both if INVALID)
const USE_MASK: u64 = 0x3;
/// `x & USE_MASK == USE_BITS`: rest is a sequence of 4-bit blocks; len is number of blocks used
const USE_BITS: u64 = 0x01;
/// `x & USE_MASK == USE_PTR`: high 62 bits are a pointer (or hash: see to_nzu64)
const USE_PTR: u64 = 0x02;

const MASK_LEN: u64 = 0xF0;
const SHIFT_LEN: u8 = 4;
const BLOCKS: u8 = 14;
const MASK_BITS: u64 = 0xFFFF_FFFF_FFFF_FF00;

const MASK_PTR: u64 = 0xFFFF_FFFF_FFFF_FFFC;

/// Integer or pointer to a reference-counted slice.
///
/// Use `Self::get_ptr` to determine the variant used. When reading a pointer,
/// mask with MASK_PTR.
///
/// `self.0 & USE_MASK` are the "flag bits" determining the variant used. This
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

fn hash(path: &[usize]) -> u64 {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}

impl IntOrPtr {
    const ROOT: Self = IntOrPtr(NonZeroU64::new(USE_BITS).unwrap(), PhantomData);
    const INVALID: Self = IntOrPtr(NonZeroU64::new(INVALID).unwrap(), PhantomData);

    #[inline]
    fn get_ptr(&self) -> Option<*mut usize> {
        if self.0.get() & USE_MASK == USE_PTR {
            let p = usize::conv(self.0.get() & MASK_PTR);
            Some(p as *mut usize)
        } else {
            None
        }
    }

    fn get_slice(&self) -> Option<&[usize]> {
        self.get_ptr().map(|p| unsafe {
            let len = *p.offset(1);
            let p = p.offset(2);
            slice::from_raw_parts(p, len)
        })
    }

    /// Construct from an integer
    ///
    /// Note: requires `x & USE_MASK == USE_BITS`.
    fn new_int(x: u64) -> Self {
        assert!(x & USE_MASK == USE_BITS);
        let x = NonZeroU64::new(x).unwrap();
        let u = IntOrPtr(x, PhantomData);
        assert!(u.get_ptr().is_none());
        u
    }

    /// Construct as a slice from an iterator
    fn new_iter<I: Clone + Iterator<Item = usize>>(iter: I) -> Self {
        let ref_count = 1;
        let len = iter.clone().count();
        let v: Vec<usize> = once(ref_count).chain(once(len)).chain(iter).collect();
        let x = hash(&v);

        let mut p: Option<*mut usize> = None;
        let pp = &mut p;
        DB.with_borrow_mut(move |db| match db.entry(x) {
            Entry::Occupied(entry) => {
                let p = *entry.get();
                let slice = unsafe { slice::from_raw_parts(p.offset(2), *p.offset(1)) };
                if slice == &v[2..] {
                    unsafe { increment_rc(p) };
                    *pp = Some(p);
                } else {
                    // Hash collision: do not use the DB
                    let p = Box::leak(v.into_boxed_slice()) as *mut [usize] as *mut usize;
                    *pp = Some(p);
                }
            }
            Entry::Vacant(entry) => {
                let p = Box::leak(v.into_boxed_slice()) as *mut [usize] as *mut usize;
                *pp = Some(p);
                entry.insert(p);
            }
        });

        let p: u64 = (p.expect("failed to access Id DB") as usize).cast();
        debug_assert_eq!(p & 3, 0);
        let p = p | USE_PTR;
        let u = IntOrPtr(NonZeroU64::new(p).unwrap(), PhantomData);
        assert!(u.get_ptr().is_some());
        u
    }

    fn get(&self) -> Variant<'_> {
        if let Some(slice) = self.get_slice() {
            Variant::Slice(slice)
        } else if self.0.get() == INVALID {
            Variant::Invalid
        } else {
            Variant::Int(self.0.get())
        }
    }

    fn to_nzu64(&self) -> NonZeroU64 {
        if let Some(slice) = self.get_slice() {
            let x = hash(slice) & MASK_PTR;
            NonZeroU64::new(x | USE_PTR).unwrap()
        } else {
            self.0
        }
    }

    fn try_from_u64(n: u64) -> Option<IntOrPtr> {
        match n & USE_MASK {
            USE_BITS => {
                // TODO: sanity checks
                Some(IntOrPtr(NonZeroU64::new(n).unwrap(), PhantomData))
            }
            USE_PTR => {
                let mut result = None;
                let r = &mut result;
                DB.with_borrow(|db| {
                    if let Some(p) = db.get(&n) {
                        unsafe { increment_rc(*p) };
                        let p: u64 = (*p as usize).cast();
                        *r = Some(IntOrPtr(NonZeroU64::new(p).unwrap(), PhantomData))
                    }
                });
                result
            }
            _ if n == !0 => Some(IntOrPtr::INVALID),
            _ => None,
        }
    }
}

/// Increment a IntOrPtr pointer ref-count
///
/// Safety: the input must be a valid IntOrPtr pointer value.
unsafe fn increment_rc(p: *mut usize) {
    unsafe {
        let ref_count = *p;

        // Copy behaviour of Rc::clone:
        if ref_count == 0 || ref_count == usize::MAX {
            std::process::abort();
        }

        *p = ref_count + 1;
    }
}

impl Clone for IntOrPtr {
    fn clone(&self) -> Self {
        if let Some(p) = self.get_ptr() {
            unsafe { increment_rc(p) };
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

                    let x = hash(slice);
                    DB.with_borrow_mut(|db| db.remove(&x));

                    let _ = Box::<[usize]>::from_raw(slice);
                }
            }
        }
    }
}

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

/// Iterator over [`Id`] path components
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

/// Widget identifier
///
/// All widgets are assigned an identifier which is unique within the window.
/// This type may be tested for equality and order and may be iterated over as
/// a "path" of "key" values.
///
/// Formatting an `Id` via [`Display`] prints the the path, for example
/// `#1290a4`. Here, `#` represents the root; each following hexadecimal digit
/// represents a path component except that digits `8-f` are combined with the
/// following digit(s). Hence, the above path has components `1`, `2`, `90`,
/// `a4`. To interpret these values, first subtract 8 from each digit but the
/// last digit, then read as base-8: `[1, 2, 8, 20]`.
///
/// # Representation
///
/// This type is small (64-bit) and non-zero: `Option<Id>` has the same
/// size as `Id`. It is also very cheap to `Clone`: usually only one `if`
/// check, and in the worst case a pointer dereference and ref-count increment.
/// Paths up to 14 digits long (as printed) are represented internally;
/// beyond this limit a reference-counted stack allocation is used.
///
/// `Id` is neither `Send` nor `Sync`.
///
/// # Invalid state
///
/// When `Id` is [`Default`] constructed, it uses a special **invalid** state.
/// Any attempt to use or compare such an invalid state will cause a panic.
///
/// Widgets are initially constructed with an invalid `Id`, then assigned an
/// `Id` when configured (see [`Events::configure`]).
/// In most cases values are persistent but this is not guaranteed (e.g.
/// inserting or removing a child from a `List` widget will affect the
/// identifiers of all following children). View-widgets assign path components
/// based on the data key, thus *possibly* making identifiers persistent.
///
/// [`Display`]: std::fmt::Display
/// [`ConfigCx::configure`]: crate::event::ConfigCx::configure
/// [`Events::configure`]: crate::Events::configure
#[allow(clippy::assigning_clones)]
#[derive(Clone)]
pub struct Id(IntOrPtr);

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
        assert!((bits & USE_MASK) == USE_BITS);
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

impl Id {
    /// Identifier of the window
    pub(crate) const ROOT: Self = Id(IntOrPtr::ROOT);

    const INVALID: Self = Id(IntOrPtr::INVALID);

    /// Is the identifier valid?
    ///
    /// Default-constructed identifiers are invalid. Comparing invalid ids is
    /// considered a logic error and thus will panic in debug builds.
    /// This method may be used to check an identifier's validity.
    pub fn is_valid(&self) -> bool {
        self.0.get() != Variant::Invalid
    }

    /// Iterate over path components
    pub fn iter(&self) -> WidgetPathIter<'_> {
        match self.0.get() {
            Variant::Invalid => panic!("Id::iter: invalid"),
            Variant::Int(x) => WidgetPathIter(PathIter::Bits(BitsIter::new(x))),
            Variant::Slice(path) => WidgetPathIter(PathIter::Slice(path.iter().cloned())),
        }
    }

    /// Get the path len (number of indices, not storage length)
    pub fn path_len(&self) -> usize {
        match self.0.get() {
            Variant::Invalid => panic!("Id::path_len: invalid"),
            Variant::Int(x) => BitsIter::new(x).count(),
            Variant::Slice(path) => path.len(),
        }
    }

    /// Returns true if `self` equals `id` or if `id` is a descendant of `self`
    pub fn is_ancestor_of(&self, id: &Self) -> bool {
        match (self.0.get(), id.0.get()) {
            (Variant::Invalid, _) | (_, Variant::Invalid) => {
                panic!("Id::is_ancestor_of: invalid")
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

    /// Find the most-derived common ancestor
    pub fn common_ancestor(&self, id: &Self) -> Self {
        let mut r = Id::ROOT;
        for (a, b) in self.iter().zip(id.iter()) {
            if a != b {
                break;
            }
            r = r.make_child(a);
        }
        r
    }

    pub fn iter_keys_after(&self, id: &Self) -> WidgetPathIter<'_> {
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
                panic!("Id::next_key_after: invalid")
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
    pub fn parent(&self) -> Option<Id> {
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
                    return Some(Id(IntOrPtr::new_int(id)));
                }
                None
            }
            Variant::Slice(path) => {
                if path.is_empty() {
                    return None;
                }

                let len = path.len();
                let path = &path[0..len - 1];

                if len > BLOCKS as usize {
                    Some(Id(IntOrPtr::new_iter(path.iter().cloned())))
                } else {
                    Some(make_id(path))
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
            Variant::Invalid => panic!("Id::make_child: invalid"),
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
                    debug_assert_eq!(used_blocks, req_bits.div_ceil(3));
                    let len = (block_len as u64 + used_blocks as u64) << SHIFT_LEN;
                    let rest = bits << 4 * avail_blocks - bit_len + 8;
                    let id = (self_x & MASK_BITS) | rest | len | USE_BITS;
                    Id(IntOrPtr::new_int(id))
                } else {
                    Id(IntOrPtr::new_iter(BitsIter::new(self_x).chain(once(key))))
                }
            }
            Variant::Slice(path) => Id(IntOrPtr::new_iter(path.iter().cloned().chain(once(key)))),
        }
    }

    /// Convert to a [`NonZeroU64`] representation
    ///
    /// This conversion is infallible but not free.
    ///
    /// The result may be used in equality checks (with a *very* small risk of
    /// false positive due to hash collision) and may be passed to
    /// [`Self::try_from_u64`].
    pub fn to_nzu64(&self) -> NonZeroU64 {
        self.0.to_nzu64()
    }

    /// Try converting a `u64` to an `Id`
    ///
    /// This method is the inverse of [`Self::to_nzu64`]. Assuming that the
    /// input `n` is the result of calling `id.to_nzu64().get()` for some `id`,
    /// then this method will return:
    ///
    /// -   `Some(Id::INVALID)` where `!id.is_valid()`
    /// -   `Some(id)` where `id` uses the internal (non-allocated) representation
    /// -   Either `None` or `Some(id2)` where `id == id2` where `id` uses an
    ///     allocated representation, excepting the unlikely case of hash
    ///     collision (in this case the resulting `id2` represents another path)
    ///
    /// This method will return `None` where:
    ///
    /// -   `n == 0`
    /// -   The originating `id` used an allocating representation and has
    ///     been deallocated
    /// -   The originating `id` used an allocating representation and was
    ///     constructed on another thread (excepting the case where an `Id` with
    ///     the same hash was constructed on this thread)
    ///
    /// Bad / random inputs may yield either `None` or `Some(bad_id)` and in the
    /// latter case operations on `bad_id` may panic, but is memory safe.
    pub fn try_from_u64(n: u64) -> Option<Id> {
        IntOrPtr::try_from_u64(n).map(Id)
    }
}

impl PartialEq for Id {
    fn eq(&self, rhs: &Self) -> bool {
        match (self.0.get(), rhs.0.get()) {
            (Variant::Invalid, _) | (_, Variant::Invalid) => panic!("Id::eq: invalid id"),
            (Variant::Int(x), Variant::Int(y)) => x == y,
            _ => self.iter().eq(rhs.iter()),
        }
    }
}
impl Eq for Id {}

impl PartialEq<str> for Id {
    fn eq(&self, rhs: &str) -> bool {
        // TODO(opt): we don't have to format to test this
        let formatted = format!("{self}");
        formatted == rhs
    }
}
impl PartialEq<&str> for Id {
    fn eq(&self, rhs: &&str) -> bool {
        self == *rhs
    }
}

impl PartialOrd for Id {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        Some(self.cmp(rhs))
    }
}

impl Ord for Id {
    fn cmp(&self, rhs: &Self) -> Ordering {
        match (self.0.get(), rhs.0.get()) {
            (Variant::Invalid, _) | (_, Variant::Invalid) => panic!("Id::cmp: invalid id"),
            (Variant::Int(x), Variant::Int(y)) => x.cmp(&y),
            _ => self.iter().cmp(rhs.iter()),
        }
    }
}

impl Hash for Id {
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

impl PartialEq<Option<Id>> for Id {
    #[inline]
    fn eq(&self, rhs: &Option<Id>) -> bool {
        rhs.as_ref().map(|id| id == self).unwrap_or(false)
    }
}

impl<'a> PartialEq<Option<&'a Id>> for Id {
    #[inline]
    fn eq(&self, rhs: &Option<&'a Id>) -> bool {
        rhs.map(|id| id == self).unwrap_or(false)
    }
}

impl<'a> PartialEq<&'a Id> for Id {
    #[inline]
    fn eq(&self, rhs: &&Id) -> bool {
        self == *rhs
    }
}

impl<'a> PartialEq<&'a Option<Id>> for Id {
    #[inline]
    fn eq(&self, rhs: &&Option<Id>) -> bool {
        self == *rhs
    }
}

impl Default for Id {
    fn default() -> Self {
        Id::INVALID
    }
}

impl fmt::Debug for Id {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "Id({self})")
    }
}

impl fmt::Display for Id {
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

/// Convert to a `NodeId` using [`Id::to_nzu64`]
///
/// Use [`Id::try_from_u64`] for the inverse conversion.
#[cfg(feature = "accesskit")]
impl From<&Id> for accesskit::NodeId {
    #[inline]
    fn from(id: &Id) -> Self {
        accesskit::NodeId(id.to_nzu64().get())
    }
}

/// Types supporting conversion to [`Id`]
///
/// A method taking an `id: impl HasId` parameter supports an [`Id`],
/// a reference to an [`Id`] (which is thus cloned),
/// or a (mutable) reference to a widget.
///
/// Note: in some cases attempting to pass a widget reference does not pass
/// borrow checks. In this case pass `widget.id()` explicitly.
pub trait HasId {
    fn has_id(self) -> Id;
}

impl HasId for Id {
    #[inline]
    fn has_id(self) -> Id {
        self
    }
}

impl HasId for &Id {
    #[inline]
    fn has_id(self) -> Id {
        self.clone()
    }
}

impl HasId for &mut Id {
    #[inline]
    fn has_id(self) -> Id {
        self.clone()
    }
}

fn make_id(seq: &[usize]) -> Id {
    let mut id = Id::ROOT;
    for x in seq {
        id = id.make_child(*x);
    }
    id
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn size_of_option_widget_id() {
        use std::mem::size_of;
        assert_eq!(size_of::<Id>(), 8);
        assert_eq!(size_of::<Id>(), size_of::<Option<Id>>());
    }

    #[test]
    fn test_partial_eq() {
        assert_eq!(Id::ROOT, Id::ROOT);
        let c1 = Id::ROOT.make_child(0).make_child(15);
        let c2 = Id::ROOT.make_child(1).make_child(15);
        let c3 = Id::ROOT.make_child(0).make_child(14);
        let c4 = Id::ROOT.make_child(0).make_child(15);
        assert!(c1 != c2);
        assert!(c1 != c3);
        assert!(c2 != c3);
        assert_eq!(c1, c4);
        assert!(c1 != Id::ROOT);

        let d1 = Id(IntOrPtr::new_iter([0, 15].iter().cloned()));
        let d2 = Id(IntOrPtr::new_iter([1, 15].iter().cloned()));
        assert_eq!(c1, d1);
        assert_eq!(c2, d2);
        assert!(d1 != d2);
        assert!(d1 != Id::ROOT);
    }

    #[test]
    #[should_panic]
    fn test_partial_eq_invalid_1() {
        let _ = Id::INVALID == Id::INVALID;
    }

    #[test]
    #[should_panic]
    fn test_partial_eq_invalid_2() {
        let _ = Id::ROOT == Id::INVALID;
    }

    #[test]
    #[should_panic]
    fn test_partial_eq_invalid_3() {
        let _ = Id::INVALID == Id::ROOT;
    }

    #[test]
    fn test_partial_eq_str() {
        assert_eq!(Id::ROOT, "#");
        assert!(Id::ROOT != "#0");
        assert_eq!(Id::ROOT.make_child(0).make_child(15), "#097");
        assert_eq!(Id::ROOT.make_child(1).make_child(15), "#197");
        let c3 = Id::ROOT.make_child(0).make_child(14);
        assert_eq!(c3, "#096");
        assert!(c3 != "#097");
    }

    #[test]
    fn test_ord() {
        let root = Id::ROOT;
        let c_0 = root.make_child(0);
        let c_0_0 = c_0.make_child(0);
        assert!(root < c_0);
        assert!(c_0 < c_0_0);

        let c_1 = root.make_child(1);
        assert!(c_0_0 < c_1);
        assert!(c_1 < root.make_child(8));

        let d_0 = Id(IntOrPtr::new_iter([0].iter().cloned()));
        let d_0_0 = Id(IntOrPtr::new_iter([0, 0].iter().cloned()));
        let d_1 = Id(IntOrPtr::new_iter([1].iter().cloned()));
        assert_eq!(d_0.cmp(&c_0), Ordering::Equal);
        assert_eq!(d_0_0.cmp(&c_0_0), Ordering::Equal);
        assert_eq!(d_1.cmp(&c_1), Ordering::Equal);
        assert!(d_0 < d_0_0);
        assert!(d_0_0 < d_1);
    }

    #[test]
    #[should_panic]
    fn test_ord_invalid() {
        let _ = Id::INVALID < Id::ROOT;
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
            let len = seq.len();
            let id = make_id(&seq[..len - 1]);

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
            let id = make_id(seq);
            let v = id.to_nzu64().get();
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
            format!("{}", make_id(seq))
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
            let id = make_id(seq);
            let mut id2 = id.clone();
            for x in seq2 {
                id2 = id2.make_child(*x);
            }
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
            let id = make_id(seq);
            let id2 = make_id(seq2);
            assert_eq!(id.is_ancestor_of(&id2), false);
            assert_eq!(id2.next_key_after(&id), None);
        }

        test(&[0], &[]);
        test(&[0], &[1]);
        test(&[2, 10, 1], &[2, 10]);
        test(&[0, 5, 2], &[0, 1, 5]);
    }

    #[test]
    fn common_ancestor() {
        fn test(seq: &[usize], seq2: &[usize], seq3: &[usize]) {
            let id = make_id(seq);
            let id2 = make_id(seq2);
            assert_eq!(id.common_ancestor(&id2), make_id(seq3));
        }

        test(&[0], &[], &[]);
        test(&[0], &[1, 2], &[]);
        test(&[0], &[0, 2], &[0]);
        test(&[2, 10, 1], &[2, 10], &[2, 10]);
        test(&[0, 5, 2], &[0, 5, 1], &[0, 5]);
    }

    #[test]
    fn deduplicate_allocations() {
        let id1 = make_id(&[2, 6, 1, 1, 0, 13, 0, 100, 1000]);
        let id2 = make_id(&[2, 6, 1, 1, 0, 13, 0, 100, 1000]);
        assert_eq!(id1.0 .0, id2.0 .0);
    }
}
