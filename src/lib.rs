//! Various implementations of `Arc<str>`-like types.
//!
//! Well, just the one at the moment: [`ArcStr`], which has the following
//! benefits over `Arc<str>`:
//!
//! - Only a single pointer. Great for cases where you want to keep the data
//!   structure lightweight or need to do some FFI stuff with it.
//!
//! - It's possible to create a const `arcstr` from a literal via the
//!   [`literal_arcstr!`][crate::literal_arcstr] macro.
//!
//!   These are zero cost, take no heap allocation, and don't even need to
//!   perform atomic reads/writes when being cloned or dropped (or at any other
//!   time). They even get stored in the read-only memory of your executable,
//!   which can be beneficial for performance and memory usage. (The downside is
//!   that the API is a bit janky, see it's docs for why).
//!
//! - [`ArcStr::new()`](ArcStr.html#method.new) is a `const` function. This
//!   shouldn't be surprising given point 2 though. Naturally, this means that
//!   `ArcStr::default()` is free (unlike `std::sync::Arc<str>::default()`).
//!
//! - `ArcStr` is totally immutable. No need to lose sleep over code that thinks
//!   it has a right to mutate your `Arc` just because it holds the only
//!   reference.
//!
//! - More implementations of various traits `PartialEq<Other>` and other traits
//!   that hopefully will help improve ergonomics.
//!
//! - We don't support `Weak` references, which means the overhead of atomic
//!   operations is lower.
//!
//! ### Planned or incomplete funtionality
//!
//! #### `Substr` Type
//!
//! Essentially an ergonomic `(ArcStr, Range<usize>)`, which can be used to
//! avoid allocation when creating a lot of ranges over the same string. A use
//! case for this is parsers and lexers (Note that in practice I might use
//! `Range<u32>` and not `Range<usize>`).
//!
//! #### `Key` type.
//!
//! Essentially this will be an 8-byte wrapper around `ArcStr` that allows
//! storing small 7b-or-fewer strings inline, without allocation. It will be 8
//! bytes on 32-bit and 64-bit platforms, since 3b-or-fewer is not compelling.
//!
//! Actually, I need to do some invesigation that 7b isn't too small too. The
//! idea is for use as map keys or other small frequently repeated identifiers.
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
mod arc_str;
#[cfg(feature = "serde")]
mod impl_serde;
pub use arc_str::ArcStr;

/// Create a const `ArcStr` from a string literal. The resulting `ArcStr`
/// require no heap allocation, can be freely cloned and used interchangeably
/// with `ArcStr`s from the heap, and are effectively "free".
///
/// The main downside is that it's a macro. Eventually it may be doable as a
/// `const fn`, but for now the drawbacks to this are not overwhelming.
///
/// # Usage
///
/// ```
/// # use arcstr::ArcStr;
/// // Works in const:
/// const MY_ARCSTR: ArcStr = arcstr::literal!("testing testing");
/// assert_eq!(MY_ARCSTR, "testing testing");
///
/// // Or, just in normal expressions.
/// assert_eq!("Wow!", arcstr::literal!("Wow!"));
/// ```
///
/// Another motivating use case is bundled files (eventually this will improve
/// when `arcstr::Substr` is implemented):
///
/// ```rust,ignore
/// use arcstr::ArcStr;
/// const VERY_IMPORTANT_FILE: ArcStr =
///     arcstr::literal!(include_str!("./very-important.txt"));
/// ```
#[macro_export]
macro_rules! literal {
    ($text:expr) => {{
        // Note: extra scope to reduce the size of what's in `$text`'s scope
        // (note that consts in macros dont have hygene the way let does).
        const __TEXT: &str = $text;
        {
            const SI: &$crate::_private::StaticArcStrInner<[u8; __TEXT.len()]> = unsafe {
                &$crate::_private::StaticArcStrInner {
                    len_flags: __TEXT.len() << 1,
                    count: 0,
                    // See comment for `_private::ConstPtrDeref` for what the hell's
                    // going on here.
                    data: *$crate::_private::ConstPtrDeref::<[u8; __TEXT.len()]> {
                        p: __TEXT.as_ptr(),
                    }
                    .a,
                }
            };
            const S: ArcStr = unsafe { $crate::ArcStr::_private_new_from_static_data(SI) };
            S
        }
    }};
}

// Not public API, exists for macros
#[doc(hidden)]
pub mod _private {
    // Not part of public API. transmutes a `*const u8` to a `&[u8; N]`.
    //
    // As of writing this, it's unstable to directly deref a raw pointer in
    // const code, we can get around this by transmuting (using the
    // const-transmute union trick) to tranmute from `*const u8` to `&[u8; N]`,
    // and the dereferencing that.
    //
    // ... I'm a little surprised that this is allowed, but in general I do
    // remember a motivation behind stabilizing transmute in `const fn` was that
    // the union trick existed as a workaround.
    //
    // Anyway, this trick is courtesy of rodrimati1992 (that means you have to
    // blame them if it blows up :p).
    pub union ConstPtrDeref<Arr: Copy + 'static> {
        pub p: *const u8,
        pub a: &'static Arr,
    }
    pub use crate::arc_str::StaticArcStrInner;
}
