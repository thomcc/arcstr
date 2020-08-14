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

/// Create a const `ArcStr` from a (byte-string) literal. The resulting `ArcStr`
/// require no heap allocation, can be freely cloned and used interchangeably
/// with `ArcStr`s from the heap, and are effectively "free".
///
/// The downside here is that the API for creating them is not perfect :(.
///
/// - First, it's a macro, not a `const fn`.
///
/// - Second, a byte-string literal is required, and not a string literal. So
///   instead of `literal_arcstr!("foo")` you must do `literal_arcstr!(b"foo")`.
///
/// - Third, this macro is `unsafe`, as not accepting a `&str` literal means we
///   can't guarantee UTF-8 any longer.
///
/// These drawbacks suck, but this functionality is insanely useful.
///
/// This is also why it's `arcstr::literal_arcstr!(...)` instead of a nicer
/// `arcstr::literal!(...)`: I'm saving the better name for when the `unsafe` is
/// no longer needed.
///
/// (P.S. please file an issue, PR, or contact me somehow if you know a way
/// around this. Note that it's trickier than it might seem, though).
///
/// # Usage
///
/// ```
/// # use arcstr::{ArcStr, literal_arcstr};
/// // The argument must be a byte-string literal. E.g. `b"foo"`.
/// // Not plain `"foo"`! Additionally, `unsafe` is required as we
/// // cannot ensure the input is valid UTF-8.
/// const MY_ARCSTR: ArcStr = literal_arcstr!("testing testing");
/// assert_eq!(MY_ARCSTR, "testing testing");
///
/// // Or, just in normal expressions.
/// assert_eq!("Wow!", literal_arcstr!("Wow!"));
/// ```
///
/// # Safety
///
/// The argument to this macro must be valid UTF-8.
///
/// Because this only accepts a byte-string literal, it is possible to provide
/// invalid UTF-8 (e.g. `b"f\xff"`), which would be a violation of the safety
/// contract. Dont' do it!
#[macro_export]
macro_rules! literal_arcstr {
    ($text:expr) => {{
        const S: &str = $text;
        const LEN: usize = S.len();
        const INNER: &$crate::private_::StaticArcStrInner<[u8; LEN]> = {
            let mut data = [0u8; LEN];
            let mut i = 0;
            let s = S.as_bytes();
            loop {
                if i >= LEN { break; }
                data[i] = s[i];
                i += 1;
            }
            &$crate::private_::StaticArcStrInner {
                len_flags: LEN << 1,
                count: 0,
                data,
            }
        };
        unsafe { $crate::ArcStr::new_static(INNER) }
    }};
}

// Not public API, exists for macros
#[doc(hidden)]
pub mod private_ {
    pub use crate::arc_str::StaticArcStrInner;
}

