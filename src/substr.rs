#![allow(
// We follow libstd's lead and prefer to define both.
    clippy::partialeq_ne_impl,
// This is a really annoying clippy lint, since it's required for so many cases...
    clippy::cast_ptr_alignment,
)]
use crate::ArcStr;
use core::ops::{Range, RangeBounds};

#[cfg(feature = "substr-usize-indices")]
type Idx = usize;

#[cfg(not(feature = "substr-usize-indices"))]
type Idx = u32;

/// A low-cost string type representing a view into an [`ArcStr`].
///
/// Conceptually this is `(ArcStr, Range<usize>)` with ergonomic helpers. In
/// implementation, the only difference between it and that is that the index
/// type is `u32` unless the `substr-usize-indices` feature is enabled, which
/// makes them use `usize`.
///
/// # Caveats
/// The main caveat is the bit I mentioned above. The index type is u32 by
/// default. You can turn on `substr-usize-indices` if you desire though. The
/// feature doesn't change the public API at all, just makes it able to handle
/// enormous strings without panicking.
#[derive(Clone)]
#[repr(C)] // We mentioned ArcStr being good at FFI at some point so why not
pub struct Substr(ArcStr, Idx, Idx);

#[inline]
#[cfg(all(target_pointer_width = "64", not(feature = "substr-usize-indices")))]
const fn to_idx_const(i: usize) -> Idx {
    const DUMMY: [(); 1] = [()];
    let _ = DUMMY[i >> 32];
    i as Idx
}
#[inline]
#[cfg(any(not(target_pointer_width = "64"), feature = "substr-usize-indices"))]
const fn to_idx_const(i: usize) -> Idx {
    i as Idx
}

#[inline]
#[cfg(all(target_pointer_width = "64", not(feature = "substr-usize-indices")))]
fn to_idx(i: usize) -> Idx {
    if i > 0xffff_ffff {
        index_overflow(i);
    }
    i as Idx
}

#[inline]
#[cfg(any(not(target_pointer_width = "64"), feature = "substr-usize-indices"))]
fn to_idx(i: usize) -> Idx {
    i as Idx
}

#[cold]
#[inline(never)]
#[cfg(all(target_pointer_width = "64", not(feature = "substr-usize-indices")))]
fn index_overflow(i: usize) -> ! {
    panic!("The index {} is too large for arcstr::Substr (enable the `substr-usize-indices` feature in `arcstr` if you need this)", i);
}
#[cold]
#[inline(never)]
fn bad_substr_idx(s: &ArcStr, i: usize, e: usize) -> ! {
    assert!(i <= e, "Bad substr range: start {} must be <= end {}", i, e);
    let max = if cfg!(all(
        target_pointer_width = "64",
        not(feature = "substr-usize-indices")
    )) {
        u32::MAX as usize
    } else {
        usize::MAX
    };
    let len = s.len().min(max);
    assert!(
        e <= len,
        "Bad substr range: end {} must be <= string length/index max size {}",
        e,
        len
    );
    assert!(
        s.is_char_boundary(i) && s.is_char_boundary(e),
        "Bad substr range: start and end must be on char boundaries"
    );
    unreachable!(
        "[arcstr bug]: should have failed one of the above tests: \
                  please report me. debugging info: b={}, e={}, l={}, max={:#x}",
        i,
        e,
        s.len(),
        max
    );
}

impl Substr {
    /// Construct an empty substr.
    ///
    /// # Examples
    /// ```
    /// # use arcstr::Substr;
    /// let s = Substr::new();
    /// assert_eq!(s, "");
    /// ```
    #[inline]
    pub const fn new() -> Self {
        Substr(ArcStr::new(), 0, 0)
    }

    /// Construct a Substr over the entire ArcStr.
    ///
    /// This is also provided as `Substr::from(some_arcstr)`, and can be
    /// accomplished with `a.substr(..)`, `a.into_substr(..)`, ...
    ///
    /// # Examples
    /// ```
    /// # use arcstr::{Substr, ArcStr};
    /// let s = Substr::full(ArcStr::from("foo"));
    /// assert_eq!(s, "foo");
    /// assert_eq!(s.range(), 0..3);
    /// ```
    #[inline]
    pub fn full(a: ArcStr) -> Self {
        let l = to_idx(a.len());
        Substr(a, 0, l)
    }

    #[inline]
    pub(crate) fn from_parts(a: &ArcStr, range: impl RangeBounds<usize>) -> Self {
        use core::ops::Bound;
        let begin = match range.start_bound() {
            Bound::Included(&n) => n,
            Bound::Excluded(&n) => n + 1,
            Bound::Unbounded => 0,
        };

        let end = match range.end_bound() {
            Bound::Included(&n) => n + 1,
            Bound::Excluded(&n) => n,
            Bound::Unbounded => a.len(),
        };
        let _ = &a.as_str()[begin..end];
        if end == begin {
            Self::new()
        } else {
            Self(ArcStr::clone(a), to_idx(begin), to_idx(end))
        }
    }

    /// Extract a substr of this substr.
    ///
    /// If the result would be empty, a new strong reference to our parent is
    /// not created.
    ///
    /// # Examples
    /// ```
    /// # use arcstr::Substr;
    /// let s: Substr = arcstr::literal!("foobarbaz").substr(3..);
    /// assert_eq!(s.as_str(), "barbaz");
    ///
    /// let s2 = s.substr(1..5);
    /// assert_eq!(s2, "arba");
    /// ```
    /// # Panics
    /// If any of the following are untrue, we panic
    /// - `range.start() <= range.end()`
    /// - `range.end() <= self.len()`
    /// - `self.is_char_boundary(start) && self.is_char_boundary(end)`
    /// - These can be conveniently verified in advance using
    ///   `self.get(start..end).is_some()` if needed.
    #[inline]
    pub fn substr(&self, range: impl RangeBounds<usize>) -> Self {
        use core::ops::Bound;
        let my_end = self.2 as usize;

        let begin = match range.start_bound() {
            Bound::Included(&n) => n,
            Bound::Excluded(&n) => n + 1,
            Bound::Unbounded => 0,
        };

        let end = match range.end_bound() {
            Bound::Included(&n) => n + 1,
            Bound::Excluded(&n) => n,
            Bound::Unbounded => self.len(),
        };
        let new_begin = self.1 as usize + begin;
        let new_end = self.1 as usize + end;
        // let _ = &self.0.as_str()[new_begin..new_end];
        if begin > end
            || end > my_end
            || !self.0.is_char_boundary(new_begin)
            || !self.0.is_char_boundary(new_end)
        {
            bad_substr_idx(&self.0, new_begin, new_end);
        }
        debug_assert!(self.0.get(new_begin..new_end).is_some());

        if new_end == new_begin {
            Self::new()
        } else {
            debug_assert!(new_begin <= (Idx::MAX as usize) && new_end <= (Idx::MAX as usize));
            Self(ArcStr::clone(&self.0), new_begin as Idx, new_end as Idx)
        }
    }

    /// Extract a string slice containing our data.
    ///
    /// Note: This is an equivalent to our `Deref` implementation, but can be
    /// more readable than `&*s` in the cases where a manual invocation of
    /// `Deref` would be required.
    ///
    /// # Examples
    /// ```
    /// # use arcstr::Substr;
    /// let s: Substr = arcstr::literal!("foobar").substr(3..);
    /// assert_eq!(s.as_str(), "bar");
    /// ```
    #[inline]
    pub fn as_str(&self) -> &str {
        self
    }

    /// Returns the length of this `Substr` in bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use arcstr::{ArcStr, Substr};
    /// let a: Substr = ArcStr::from("foo").substr(1..);
    /// assert_eq!(a.len(), 2);
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        debug_assert!(self.2 >= self.1);
        (self.2 - self.1) as usize
    }

    /// Returns true if this `Substr` is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// # use arcstr::Substr;
    /// assert!(arcstr::literal!("abc").substr(3..).is_empty());
    /// assert!(!arcstr::literal!("abc").substr(2..).is_empty());
    /// assert!(Substr::new().is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.2 == self.1
    }

    /// Convert us to a `std::string::String`.
    ///
    /// This is provided as an inherent method to avoid needing to route through
    /// the `Display` machinery, but is equivalent to `ToString::to_string`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use arcstr::Substr;
    /// let s: Substr = arcstr::literal!("12345").substr(1..4);
    /// assert_eq!(s.to_string(), "234");
    /// ```
    #[inline]
    #[allow(clippy::inherent_to_string_shadow_display)]
    pub fn to_string(&self) -> alloc::string::String {
        #[cfg(not(feature = "std"))]
        use alloc::borrow::ToOwned;
        self.as_str().to_owned()
    }

    /// Unchecked function to cunstruct a [`Substr`] from an [`ArcStr`] and a
    /// byte range. This function is largely discouraged in favor of
    /// [`ArcStr::substr`][crate::ArcStr::substr].
    ///
    /// This is unsafe because currently `ArcStr` cannot provide a `&str` in a
    /// `const fn`. If that changes then we will likely deprecate this function,
    /// and provide a `pub const fn from_parts` with equivalent functionality.
    ///
    /// In the distant future, it would be nice if this accepted other kinds of
    /// ranges too.
    ///
    /// # Examples
    ///
    /// ```
    /// use arcstr::{ArcStr, Substr};
    /// const FOOBAR: ArcStr = arcstr::literal!("foobar");
    /// const OBA: Substr = unsafe { Substr::from_parts_unchecked(FOOBAR, 2..5) };
    /// assert_eq!(OBA, "oba");
    /// ```
    // TODO: can I do a compile_fail test that only is a failure under a certain feature?
    ///
    /// # Safety
    /// You promise that `range` is in bounds for `s`, and that the start and
    /// end are both on character boundaries. Note that we do check that the
    /// `usize` indices fit into `u32` if thats our configured index type, so
    /// `_unchecked` is not *entirely* a lie.
    ///
    /// # Panics
    /// If the `substr-usize-indices` is not enabled, and the target arch is
    /// 64-bit, and the usizes do not fit in 32 bits, then we panic with a
    /// (possibly strange-looking) index-out-of-bounds error in order to force
    /// compilation failure.
    #[inline]
    pub const unsafe fn from_parts_unchecked(s: ArcStr, range: Range<usize>) -> Self {
        Self(s, to_idx_const(range.start), to_idx_const(range.end))
    }

    /// Returns `true` if the two `Substr`s have identical parents, and are
    /// covering the same range.
    ///
    /// Note that the "identical"ness of parents is determined by
    /// [`ArcStr::ptr_eq`], which can have surprising/nondeterministic results
    /// when used on `const` `ArcStr`s. It is guaranteed that `Substr::clone()`s
    /// will be `shallow_eq` eachother, however.
    ///
    /// This should generally only be used as an optimization, or a debugging
    /// aide. Additionally, it is already used in the implementation of
    /// `PartialEq`, so optimizing a comparison by performing it first is
    /// generally unnecessary.
    ///
    /// # Examples
    /// ```
    /// # use arcstr::{ArcStr, Substr};
    /// let parent = ArcStr::from("foooo");
    /// let sub1 = parent.substr(1..3);
    /// let sub2 = parent.substr(1..3);
    /// assert!(Substr::shallow_eq(&sub1, &sub2));
    /// // Same parent *and* contents, but over a different range: not `shallow_eq`.
    /// let not_same = parent.substr(3..);
    /// assert!(!Substr::shallow_eq(&sub1, &not_same));
    /// ```
    #[inline]
    pub fn shallow_eq(this: &Self, o: &Self) -> bool {
        ArcStr::ptr_eq(&this.0, &o.0) && (this.1 == o.1) && (this.2 == o.2)
    }

    /// Returns the ArcStr this is a substring of.
    ///
    /// Note that the exact pointer value of this can be somewhat
    /// nondeterministic when used with `const` `ArcStr`s. For example
    ///
    /// ```rust,ignore
    /// const FOO: ArcStr = arcstr::literal!("foo");
    /// // This is non-deterministic, as all references to a given
    /// // const are not required to point to the same value.
    /// ArcStr::ptr_eq(FOO.substr(..).parent(), &FOO);
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// # use arcstr::ArcStr;
    /// let parent = ArcStr::from("abc def");
    /// let child = parent.substr(2..5);
    /// assert!(ArcStr::ptr_eq(&parent, child.parent()));
    ///
    /// let child = parent.substr(..);
    /// assert_eq!(child.range(), 0..7);
    /// ```
    #[inline]
    pub fn parent(&self) -> &ArcStr {
        &self.0
    }

    /// Returns the range of bytes we occupy inside our parent.
    ///
    /// This range is always guaranteed to:
    ///
    /// - Have an end >= start.
    /// - Have both start and end be less than or equal to `self.parent().len()`
    /// - Have both start and end be on meet `self.parent().is_char_boundary(b)`
    ///
    /// To put another way, it's always sound to do
    /// `s.parent().get_unchecked(s.range())`.
    ///
    /// ```
    /// # use arcstr::ArcStr;
    /// let parent = ArcStr::from("abc def");
    /// let child = parent.substr(2..5);
    /// assert_eq!(child.range(), 2..5);
    ///
    /// let child = parent.substr(..);
    /// assert_eq!(child.range(), 0..7);
    /// ```
    #[inline]
    pub fn range(&self) -> Range<usize> {
        (self.1 as usize)..(self.2 as usize)
    }

    /// Returns a [`Substr`] of self over the given `&str`, or panics.
    ///
    /// It is not rare to end up with a `&str` which holds a view into a
    /// `Substr`'s backing data. A common case is when using functionality that
    /// takes and returns `&str` and are entirely unaware of `arcstr`, for
    /// example: `str::trim()`.
    ///
    /// This function allows you to reconstruct a [`Substr`] from a `&str` which
    /// is a view into this `Substr`'s backing string.
    ///
    /// See [`Substr::try_substr_from`] for a version that returns an option
    /// rather than panicking.
    ///
    /// # Examples
    ///
    /// ```
    /// use arcstr::Substr;
    /// let text = Substr::from("   abc");
    /// let trimmed = text.trim();
    /// let substr: Substr = text.substr_from(trimmed);
    /// assert_eq!(substr, "abc");
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if `substr` isn't a view into our memory.
    ///
    /// Also panics if `substr` is a view into our memory but is >= `u32::MAX`
    /// bytes away from our start, if we're a 64-bit machine and
    /// `substr-usize-indices` is not enabled.
    pub fn substr_from(&self, substr: &str) -> Substr {
        // TODO: should outline `expect` call to avoid fmt bloat and let us
        // provide better error message like we do for ArcStr
        self.try_substr_from(substr)
            .expect("non-substring passed to Substr::substr_from")
    }

    /// If possible, returns a [`Substr`] of self over the given `&str`.
    ///
    /// This is a fallible version of [`Substr::substr_from`].
    ///
    /// It is not rare to end up with a `&str` which holds a view into a
    /// `ArcStr`'s backing data. A common case is when using functionality that
    /// takes and returns `&str` and are entirely unaware of `arcstr`, for
    /// example: `str::trim()`.
    ///
    /// This function allows you to reconstruct a [`Substr`] from a `&str` which
    /// is a view into this [`Substr`]'s backing string. Note that we accept the
    /// empty string as input, in which case we return the same value as
    /// [`Substr::new`] (For clarity, this no longer holds a reference to
    /// `self.parent()`).
    ///
    /// # Examples
    ///
    /// ```
    /// use arcstr::Substr;
    /// let text = Substr::from("   abc");
    /// let trimmed = text.trim();
    /// let substr: Option<Substr> = text.try_substr_from(trimmed);
    /// assert_eq!(substr.unwrap(), "abc");
    /// // `&str`s not derived from `self` will return None.
    /// let not_substr = text.try_substr_from("abc");
    /// assert!(not_substr.is_none());
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if `substr` is a view into our memory but is >= `u32::MAX` bytes
    /// away from our start, on a 64-bit machine, when `substr-usize-indices` is
    /// not enabled.
    pub fn try_substr_from(&self, substr: &str) -> Option<Substr> {
        if substr.is_empty() {
            return Some(Substr::new());
        }
        let parent_ptr = self.0.as_ptr() as usize;
        let self_start = parent_ptr + (self.1 as usize);
        let self_end = parent_ptr + (self.2 as usize);

        let substr_start = substr.as_ptr() as usize;
        let substr_end = substr_start + substr.len();
        if substr_start < self_start || substr_end > self_end {
            return None;
        }

        let index = substr_start - self_start;
        let end = index + substr.len();
        Some(self.substr(index..end))
    }
    /// Compute a derived `&str` a function of `&str` => `&str`, and produce a
    /// Substr of the result if possible.
    ///
    /// The function may return either a derived string, or any empty string.
    ///
    /// This function is mainly a wrapper around [`Substr::try_substr_from`]. If
    /// you're coming to `arcstr` from the `shared_string` crate, this is the
    /// moral equivalent of the `slice_with` function.
    ///
    /// # Examples
    ///
    /// ```
    /// use arcstr::Substr;
    /// let text = Substr::from("   abc");
    /// let trimmed: Option<Substr> = text.try_substr_using(str::trim);
    /// assert_eq!(trimmed.unwrap(), "abc");
    /// let other = text.try_substr_using(|_s| "different string!");
    /// assert_eq!(other, None);
    /// // As a special case, this is allowed.
    /// let empty = text.try_substr_using(|_s| "");
    /// assert_eq!(empty.unwrap(), "");
    /// ```
    pub fn try_substr_using(&self, f: impl FnOnce(&str) -> &str) -> Option<Self> {
        self.try_substr_from(f(self.as_str()))
    }
    /// Compute a derived `&str` a function of `&str` => `&str`, and produce a
    /// Substr of the result.
    ///
    /// The function may return either a derived string, or any empty string.
    /// Returning anything else will result in a panic.
    ///
    /// This function is mainly a wrapper around [`Substr::try_substr_from`]. If
    /// you're coming to `arcstr` from the `shared_string` crate, this is the
    /// likely closest to the `slice_with_unchecked` function, but this panics
    /// instead of UB on dodginess.
    ///
    /// # Examples
    ///
    /// ```
    /// use arcstr::Substr;
    /// let text = Substr::from("   abc");
    /// let trimmed: Substr = text.substr_using(str::trim);
    /// assert_eq!(trimmed, "abc");
    /// // As a special case, this is allowed.
    /// let empty = text.substr_using(|_s| "");
    /// assert_eq!(empty, "");
    /// ```
    pub fn substr_using(&self, f: impl FnOnce(&str) -> &str) -> Self {
        self.substr_from(f(self.as_str()))
    }
}

impl From<ArcStr> for Substr {
    #[inline]
    fn from(a: ArcStr) -> Self {
        Self::full(a)
    }
}

impl From<&ArcStr> for Substr {
    #[inline]
    fn from(a: &ArcStr) -> Self {
        Self::full(a.clone())
    }
}

impl core::ops::Deref for Substr {
    type Target = str;
    #[inline]
    fn deref(&self) -> &str {
        debug_assert!(self.0.get((self.1 as usize)..(self.2 as usize)).is_some());
        unsafe { self.0.get_unchecked((self.1 as usize)..(self.2 as usize)) }
    }
}

impl PartialEq for Substr {
    #[inline]
    fn eq(&self, o: &Self) -> bool {
        Substr::shallow_eq(self, o) || PartialEq::eq(self.as_str(), o.as_str())
    }
    #[inline]
    fn ne(&self, o: &Self) -> bool {
        !Substr::shallow_eq(self, o) && PartialEq::ne(self.as_str(), o.as_str())
    }
}

impl PartialEq<ArcStr> for Substr {
    #[inline]
    fn eq(&self, o: &ArcStr) -> bool {
        (ArcStr::ptr_eq(&self.0, o) && (self.1 == 0) && (self.2 as usize == o.len()))
            || PartialEq::eq(self.as_str(), o.as_str())
    }
    #[inline]
    fn ne(&self, o: &ArcStr) -> bool {
        (!ArcStr::ptr_eq(&self.0, o) || (self.1 != 0) || (self.2 as usize != o.len()))
            && PartialEq::ne(self.as_str(), o.as_str())
    }
}
impl PartialEq<Substr> for ArcStr {
    #[inline]
    fn eq(&self, o: &Substr) -> bool {
        PartialEq::eq(o, self)
    }
    #[inline]
    fn ne(&self, o: &Substr) -> bool {
        PartialEq::ne(o, self)
    }
}

impl Eq for Substr {}

impl PartialOrd for Substr {
    #[inline]
    fn partial_cmp(&self, s: &Self) -> Option<core::cmp::Ordering> {
        Some(self.as_str().cmp(s.as_str()))
    }
}

impl Ord for Substr {
    #[inline]
    fn cmp(&self, s: &Self) -> core::cmp::Ordering {
        self.as_str().cmp(s.as_str())
    }
}

impl core::hash::Hash for Substr {
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, h: &mut H) {
        self.as_str().hash(h)
    }
}

impl core::fmt::Debug for Substr {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(self.as_str(), f)
    }
}

impl core::fmt::Display for Substr {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(self.as_str(), f)
    }
}

impl Default for Substr {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

macro_rules! impl_from_via_arcstr {
    ($($SrcTy:ty),+) => {$(
        impl From<$SrcTy> for Substr {
            #[inline]
            fn from(v: $SrcTy) -> Self {
                Self::full(ArcStr::from(v))
            }
        }
    )+};
}
impl_from_via_arcstr![
    &str,
    &mut str,
    alloc::string::String,
    &alloc::string::String,
    alloc::boxed::Box<str>,
    alloc::rc::Rc<str>,
    alloc::sync::Arc<str>,
    alloc::borrow::Cow<'_, str>
];

impl<'a> From<&'a Substr> for alloc::borrow::Cow<'a, str> {
    #[inline]
    fn from(s: &'a Substr) -> Self {
        alloc::borrow::Cow::Borrowed(s)
    }
}

impl<'a> From<Substr> for alloc::borrow::Cow<'a, str> {
    #[inline]
    fn from(s: Substr) -> Self {
        if let Some(st) = ArcStr::as_static(&s.0) {
            debug_assert!(st.get(s.range()).is_some());
            alloc::borrow::Cow::Borrowed(unsafe { st.get_unchecked(s.range()) })
        } else {
            alloc::borrow::Cow::Owned(s.to_string())
        }
    }
}

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
    (Substr, str),
    (Substr, &'a str),
    (Substr, alloc::string::String),
    (Substr, alloc::borrow::Cow<'a, str>),
    (Substr, alloc::boxed::Box<str>),
    (Substr, alloc::sync::Arc<str>),
    (Substr, alloc::rc::Rc<str>),
}

macro_rules! impl_index {
    ($($IdxT:ty,)*) => {$(
        impl core::ops::Index<$IdxT> for Substr {
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

impl AsRef<str> for Substr {
    #[inline]
    fn as_ref(&self) -> &str {
        self
    }
}

impl AsRef<[u8]> for Substr {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl core::borrow::Borrow<str> for Substr {
    #[inline]
    fn borrow(&self) -> &str {
        self
    }
}

impl core::str::FromStr for Substr {
    type Err = core::convert::Infallible;
    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(ArcStr::from(s)))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    #[should_panic]
    #[cfg(not(miri))] // XXX does miri still hate unwinding?
    #[cfg(all(target_pointer_width = "64", not(feature = "substr-usize-indices")))]
    fn test_from_parts_unchecked_err() {
        let s = crate::literal!("foo");
        // Note: this is actually a violation of the safety requirement of
        // from_parts_unchecked (the indices are illegal), but I can't get an
        // ArcStr that's big enough, and I'm the author so I know it's fine
        // because we hit the panic case.
        let _u = unsafe { Substr::from_parts_unchecked(s, 0x1_0000_0000usize..0x1_0000_0001) };
    }
    #[test]
    fn test_from_parts_unchecked_valid() {
        let s = crate::literal!("foobar");
        let u = unsafe { Substr::from_parts_unchecked(s, 2..5) };
        assert_eq!(&*u, "oba");
    }
}
