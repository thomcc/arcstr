//! # Better reference counted strings
//!
//! This crate defines [`ArcStr`], a type similar to `Arc<str>`, but with a
//! number of new features and functionality. Theres a list of
//! [benefits][benefits] in the `ArcStr` documentation comment which covers some
//! of the reasons you might want to use it over other alternatives
//!
//! (Aside: eventually, my hope is to provide a couple utility types built on
//! top of `ArcStr`, but for now, just the main one).
//!
//! [benefits]: struct.ArcStr.html#benefits-of-arcstr-over-arcstr
//!
//! ## Feature overview
//!
//! A quick tour of the distinguishing features:
//!
//! ```
//! use arcstr::ArcStr;
//!
//! // Works in const:
//! const MY_ARCSTR: ArcStr = arcstr::literal!("amazing constant");
//! assert_eq!(MY_ARCSTR, "amazing constant");
//!
//! // `arcstr::literal!` input can come from `include_str!` too:
//! # // We have to fake it here, but this has test coverage and such.
//! # const _: &str = stringify!{
//! const MY_ARCSTR: ArcStr = arcstr::literal!(include_str!("my-best-files.txt"));
//! # };
//! ```
//!
//! Or, you can define the literals in normal expressions. Note that these
//! literals are essentially ["Zero Cost"][zero-cost]. Specifically, below we
//! not only don't allocate any heap memory to instantiate `wow` or any of the
//! clones, we also don't have to perform any atomic reads or writes.
//!
//! [zero-cost]: struct.ArcStr.html#what-does-zero-cost-literals-mean
//!
//! ```
//! use arcstr::ArcStr;
//!
//! let wow: ArcStr = arcstr::literal!("Wow!");
//! assert_eq!("Wow!", wow);
//! // This line is probably not something you want to do regularly,
//! // but causes no extra allocations, nor performs any atomic reads
//! // nor writes.
//! let wowzers = wow.clone().clone().clone().clone();
//!
//! // At some point in the future, we can get a `&'static str` out of one
//! // of the literal `ArcStr`s too. Note that this returns `None` for
//! // dynamically allocated `ArcStr`:
//! let static_str: Option<&'static str> = ArcStr::as_static(&wowzers);
//! assert_eq!(static_str, Some("Wow!"));
//! ```
//!
//! Of course, this is in addition to the typical functionality you might find a
//! non-borrowed string type (with the caveat that no way to mutate `ArcStr` is
//! provided intentionally).
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[macro_use]
mod mac;
mod arc_str;
#[cfg(feature = "serde")]
mod impl_serde;
pub use arc_str::ArcStr;

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
