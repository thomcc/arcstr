#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
use core::alloc::Layout;
use core::mem::{align_of, size_of};
use core::ptr::NonNull;
use core::sync::atomic::{AtomicUsize, Ordering};

#[cfg(feature = "serde")]
mod impl_serde;

use alloc::borrow::Cow;
use alloc::boxed::Box;
use alloc::string::String;

/// A thin equivalent to `Arc<str>`.
///
/// This offers performance benefits over `Arc<str>` or `Arc<String>` for some
/// use cases, and can be useful when working in the FFI.
///
/// # Usage
///
/// ## As a `str`
///
/// `ArcStr` implements `Deref<Target = str>`, and so all functions and methods
///  from `str` work on it, even though we don't expose them on `ArcStr`
///  directly, for example:
///
/// ```
/// # use stark::ArcStr;
/// let s = ArcStr::from("something");
/// // These go through `Deref`, so they work even though
/// // there is no `ArcStr::len` or `ArcStr::eq_ignore_ascii_case` function
/// assert_eq!(s.len(), 9);
/// assert!(s.eq_ignore_ascii_case("SOMETHING"));
/// ```
///
/// Additionally, `&ArcStr` can be passed to any function which accepts `&str`.
/// For example:
///
/// ```
/// # use stark::ArcStr;
/// fn accepts_str(s: &str) {
///    # let _ = s;
///    // s...
/// }
///
/// let test_str: ArcStr = "test".into();
/// // This works even though `&test_str` is normally an `&ArcStr`
/// accepts_str(&test_str);
///
/// // Another
/// let test_but_loud = ArcStr::from("TEST");
/// assert!(test_str.eq_ignore_ascii_case(&test_but_loud));
/// ```
#[repr(transparent)]
pub struct ArcStr(NonNull<ThinInner>);

unsafe impl Sync for ArcStr {}
unsafe impl Send for ArcStr {}

impl ArcStr {
    /// Construct a new empty string.
    #[inline]
    pub const fn new() -> Self {
        EMPTY
    }

    /// Extract a string slice containing our data.
    ///
    /// Note: This is an equivalent to our `Deref` implementation, but can be
    /// more readable than `&*s` in the cases where a manual invocation of
    /// `Deref` would be required.
    #[inline]
    pub fn as_str(&self) -> &str {
        self
    }

    /// Get the UTF-8 encoded data behind this string.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        let p = self.0.as_ptr();
        unsafe {
            let len = (*p).len;
            let data: *const [u8; 0] = &(*p).data;
            core::slice::from_raw_parts(data as *const u8, len)

            // let offset = (p as *const u8 as usize) - ((*p).data.as_ptr() as usize);
            // debug_assert_eq!((p as *const u8).wrapping_add(offset), (*p).data.as_ptr());
            // let data = (p as *const u8).add(offset);
            // core::slice::from_raw_parts(data, len)
        }
    }
}

impl Clone for ArcStr {
    #[inline]
    fn clone(&self) -> Self {
        let this = self.0.as_ptr();
        unsafe {
            if (*this).nonstatic {
                // From libstd's impl:
                //
                // > Using a relaxed ordering is alright here, as knowledge of the
                // > original reference prevents other threads from erroneously deleting
                // > the object.
                //
                // See: https://doc.rust-lang.org/src/alloc/sync.rs.html#1073
                let n = (*this).strong.fetch_add(1, Ordering::Relaxed);
                // Protect against aggressive leaking of Arcs causing us to overflow `strong`.
                if n > (isize::MAX as usize) {
                    abort();
                }
            }
        }
        Self(self.0)
    }
}

impl Drop for ArcStr {
    #[inline]
    fn drop(&mut self) {
        let this = self.0.as_ptr();
        unsafe {
            if !(*this).nonstatic {
                return;
            }
            if (*this).strong.fetch_sub(1, Ordering::Release) == 1 {
                // `libstd` uses a full acquire fence here but notes that it's
                // possibly overkill. `triomphe`/`servo_arc` some of firefox ref
                // counting uses a load like this.
                //
                // These are morally equivalent for this case, the fence being a
                // bit more obvious and the load having slightly better perf in
                // some theoretical scenarios... but for our use case both seem
                // unnecessary.
                //
                // The intention behind these is to synchronize with `Release`
                // writes to `strong` that are happening on other threads. That
                // is, after the load (or fence), writes (any write, but
                // specifically writes to any part of `this` are what we care
                // about) from other threads which happened before the latest
                // `Release` write to strong will become visible on this thread.
                //
                // The reason this feels unnecessary is that our data is
                // entirely immutable outside `(*this).strong`. There are no
                // writes we could possibly be interested in.
                //
                // That said, I'll keep (the cheaper variant of) it for now for
                // easier auditing and such... an because I'm not 100% sure that
                // changing the ordering here wouldn't require changing it for
                // the fetch_sub above, or the fetch_add in `clone`...
                let _ = (*this).strong.load(Ordering::Acquire);
                ThinInner::destroy_cold(this)
            }
        }
    }
}
// Caveat on the `nonstatic` field: This indicates if we're located in static
// data (as with empty string) or if we a normal arc-ed string.
//
// While `ArcStr` claims to hold a pointer to a `ThinInner`, for the static case
// we actually are using a pointer to a `ThinInnerStatic`. These are the same
// except for the type of the refernce count field. The issue is: We kind of
// need the static ones to not have any interior mutability, so that `const`s
// can use them, and so that they may be stored in read-only memory.
//
// We do this using the `nonstatic` flag to indicate which case we're in, and
// maintaining the invariant that if we're a `ThinInnerStatic` **we may never
// access `.strong` in any way**.
//
// This is stricter than we need to achieve in order to appease miri at the
// moment, but I'd like to appease it in the future too. That said, even in a
// crazy case, it's very hard to imagine how this could fail to work in
// practice.
#[repr(C)]
struct InnerRepr<RcTy> {
    // kind of a misnomer since there are no weak refs rn.
    strong: RcTy,
    // Note: if `nonstatic` is false, we are never allowed to look at `strong`!
    nonstatic: bool,
    len: usize,
    #[cfg(debug_assertions)]
    orig_layout: Layout,
    data: [u8; 0],
}

type ThinInner = InnerRepr<AtomicUsize>;
type ThinInnerStatic = InnerRepr<usize>;
const _: [(); size_of::<ThinInnerStatic>()] = [(); size_of::<ThinInner>()];
const _: [(); align_of::<ThinInnerStatic>()] = [(); align_of::<ThinInner>()];

const EMPTY_INNER: &'static ThinInnerStatic = &ThinInnerStatic {
    strong: 0usize,
    nonstatic: false,
    len: 0,
    #[cfg(debug_assertions)]
    orig_layout: Layout::new::<ThinInnerStatic>(),
    data: [],
};

const EMPTY: ArcStr =
    ArcStr(unsafe { NonNull::new_unchecked(EMPTY_INNER as *const _ as *mut ThinInner) });

impl ThinInner {
    fn allocate(data: &str) -> NonNull<Self> {
        const ALIGN: usize = align_of::<ThinInner>();

        let num_bytes = data.len();
        debug_assert_ne!(num_bytes, 0);

        let mo = memoffset::offset_of!(ThinInner, data);
        if num_bytes >= (isize::MAX as usize) - (mo + ALIGN) {
            alloc_overflow();
        }

        unsafe {
            debug_assert!(Layout::from_size_align(num_bytes + mo, ALIGN).is_ok());
            let layout = Layout::from_size_align_unchecked(num_bytes + mo, ALIGN);

            let alloced = alloc::alloc::alloc(layout);
            if alloced.is_null() {
                alloc::alloc::handle_alloc_error(layout);
            }

            let ptr = alloced as *mut ThinInner;
            core::ptr::write(&mut (*ptr).strong, AtomicUsize::new(1));
            core::ptr::write(&mut (*ptr).nonstatic, true);
            core::ptr::write(&mut (*ptr).len, num_bytes);
            #[cfg(debug_assertions)]
            {
                core::ptr::write(&mut (*ptr).orig_layout, layout);
            }
            debug_assert_eq!(
                (alloced as *const u8).wrapping_add(mo),
                (*ptr).data.as_ptr(),
            );
            debug_assert_eq!(&(*ptr).data as *const _ as *const u8, (*ptr).data.as_ptr());

            core::ptr::copy_nonoverlapping(data.as_ptr(), alloced.add(mo), num_bytes);

            NonNull::new_unchecked(ptr)
        }
    }

    #[cold]
    unsafe fn destroy_cold(p: *mut ThinInner) {
        debug_assert!((*p).nonstatic);
        let len = (*p).len;
        let layout = {
            let size = len + memoffset::offset_of!(ThinInner, data);
            let align = align_of::<ThinInner>();
            debug_assert_eq!(Layout::from_size_align(size, align), Ok((*p).orig_layout));
            Layout::from_size_align_unchecked(size, align)
        };
        alloc::alloc::dealloc(p as *mut _, layout);
    }
}

#[inline(never)]
#[cold]
fn alloc_overflow() -> ! {
    panic!("overflow during Layout computation")
}

impl From<&str> for ArcStr {
    #[inline]
    fn from(s: &str) -> Self {
        if s.is_empty() {
            Self::new()
        } else {
            Self(ThinInner::allocate(s))
        }
    }
}

impl core::ops::Deref for ArcStr {
    type Target = str;
    #[inline]
    fn deref(&self) -> &str {
        unsafe { core::str::from_utf8_unchecked(self.as_bytes()) }
    }
}

impl Default for ArcStr {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl From<String> for ArcStr {
    #[inline]
    fn from(v: String) -> Self {
        v.as_str().into()
    }
}

impl core::fmt::Debug for ArcStr {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(self.as_str(), f)
    }
}

impl core::fmt::Display for ArcStr {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(self.as_str(), f)
    }
}

impl PartialEq for ArcStr {
    #[inline]
    fn eq(&self, o: &Self) -> bool {
        core::ptr::eq(self.0.as_ptr(), o.0.as_ptr()) || PartialEq::eq(self.as_str(), o.as_str())
    }
    #[inline]
    fn ne(&self, o: &Self) -> bool {
        !core::ptr::eq(self.0.as_ptr(), o.0.as_ptr()) && PartialEq::ne(self.as_str(), o.as_str())
    }
}

impl Eq for ArcStr {}

macro_rules! impl_peq {
    (@one $a:ty, $b:ty) => {
        impl<'a> PartialEq<$b> for $a {
            #[inline]
            fn eq(&self, s: &$b) -> bool {
                PartialEq::eq(&self[..], &s[..])
            }
            #[inline]
            fn ne(&self, s: &$b) -> bool {
                PartialEq::ne(&self[..], &s[..])
            }
        }
    };
    ($(($a:ty, $b:ty),)+) => {$(
        impl_peq!(@one $a, $b);
        impl_peq!(@one $b, $a);
    )+};
}

impl_peq! {
    (ArcStr, str),
    (ArcStr, &'a str),
    (ArcStr, String),
    (ArcStr, Cow<'a, str>),
    (ArcStr, Box<str>),
}

impl PartialOrd for ArcStr {
    #[inline]
    fn partial_cmp(&self, s: &Self) -> Option<core::cmp::Ordering> {
        Some(self.as_str().cmp(s.as_str()))
    }
}

impl Ord for ArcStr {
    #[inline]
    fn cmp(&self, s: &Self) -> core::cmp::Ordering {
        self.as_str().cmp(s.as_str())
    }
}

impl core::hash::Hash for ArcStr {
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, h: &mut H) {
        self.as_str().hash(h)
    }
}

macro_rules! impl_index {
    ($($IdxT:ty,)*) => {$(
        impl core::ops::Index<$IdxT> for ArcStr {
            type Output = str;
            #[inline]
            fn index(&self, i: $IdxT) -> &Self::Output {
                &self.as_str()[i]
            }
        }
    )*};
}

impl_index! {
    core::ops::RangeFull,
    core::ops::Range<usize>,
    core::ops::RangeFrom<usize>,
    core::ops::RangeTo<usize>,
    core::ops::RangeInclusive<usize>,
    core::ops::RangeToInclusive<usize>,
}

impl AsRef<str> for ArcStr {
    #[inline]
    fn as_ref(&self) -> &str {
        self
    }
}

impl AsRef<[u8]> for ArcStr {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl core::borrow::Borrow<str> for ArcStr {
    #[inline]
    fn borrow(&self) -> &str {
        self
    }
}

impl core::str::FromStr for ArcStr {
    type Err = core::convert::Infallible;
    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(s))
    }
}

#[cold]
#[inline(never)]
#[cfg(not(feature = "std"))]
fn abort() -> ! {
    struct PanicOnDrop;
    impl Drop for PanicOnDrop {
        fn drop(&mut self) {
            panic!("fatal error: second panic")
        }
    }
    let _double_panicer = PanicOnDrop;
    panic!("fatal error: aborting via double panic");
}

#[cfg(feature = "std")]
use std::process::abort;

#[cfg(test)]
#[test]
fn verify_type_pun_offsets() {
    assert_eq!(
        memoffset::offset_of!(ThinInner, strong),
        memoffset::offset_of!(ThinInnerStatic, strong),
    );
    assert_eq!(
        memoffset::offset_of!(ThinInner, nonstatic),
        memoffset::offset_of!(ThinInnerStatic, nonstatic),
    );
    assert_eq!(
        memoffset::offset_of!(ThinInner, len),
        memoffset::offset_of!(ThinInnerStatic, len),
    );
    assert_eq!(
        memoffset::offset_of!(ThinInner, data),
        memoffset::offset_of!(ThinInnerStatic, data),
    );
    #[cfg(debug_assertions)]
    {
        assert_eq!(
            memoffset::offset_of!(ThinInner, orig_layout),
            memoffset::offset_of!(ThinInnerStatic, orig_layout),
        );
    }
}
