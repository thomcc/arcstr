# `arcstr`: A better reference-counted string type.

[![Build Status](https://github.com/thomcc/arcstr/workflows/CI/badge.svg)](https://github.com/thomcc/arcstr/actions)
[![codecov](https://codecov.io/gh/thomcc/arcstr/branch/main/graph/badge.svg)](https://codecov.io/gh/thomcc/arcstr)
[![Docs](https://docs.rs/arcstr/badge.svg)](https://docs.rs/arcstr)
[![Latest Version](https://img.shields.io/crates/v/arcstr.svg)](https://crates.io/crates/arcstr)

This crate defines `ArcStr`, a reference counted string type. It's essentially trying to be a better `Arc<str>` or `Arc<String>`, at least for most use cases.

ArcStr intentionally gives up some of the features of `Arc` which are rarely-used for `Arc<str>` (`Weak`, `Arc::make_mut`, ...). And in exchange, it gets a number of features that are very useful, especially for strings. Notably robust support for cheap/zero-cost `ArcStr`s holding static data (for example, string literals).

(Aside from this, it's also a single pointer, which can be good for performance and FFI)

Eventually, my hope is to provide a couple utility types built on top of `ArcStr` too (see github issues), but for now, just ArcStr is implemented.

## Feature overview

A quick tour of the distinguishing features. Note that it offers essentially the full set of functionality you'd expect in addition — these are just the unique selling points (well, the literal support is the main distinguishing feature at the moment):

```rust
use arcstr::ArcStr;
// Works in const:
const AMAZING: ArcStr = arcstr::literal!("amazing constant");
assert_eq!(AMAZING, "amazing constant");

// `arcstr::literal!` input can come from `include_str!` too:
const MY_BEST_FILES: ArcStr = arcstr::literal!(include_str!("my-best-files.txt"));
```
Or, you can define the literals in normal expressions. Note that these literals are essentially ["Zero Cost"][zero-cost]. Specifically, below we not only don't allocate any heap memory to instantiate `wow` or any of the clones, we also don't have to perform any atomic reads or writes when cloning, or dropping them (or during any other operations on them).

[zero-cost]: https://docs.rs/arcstr/%2a/arcstr/struct.ArcStr.html#what-does-zero-cost-literals-mean

```rust
let wow: ArcStr = arcstr::literal!("Wow!");
assert_eq!("Wow!", wow);
// This line is probably not something you want to do regularly,
// but as mentioned, causes no extra allocations, nor performs any
// atomic loads, stores, rmws, etc.
let wowzers = wow.clone().clone().clone().clone();

// At some point in the future, we can get a `&'static str` out of one
// of the literal `ArcStr`s too.
let static_str: Option<&'static str> = ArcStr::as_static(&wowzers);
assert_eq!(static_str, Some("Wow!"));

// Note that this returns `None` for dynamically allocated `ArcStr`:
let dynamic_arc = ArcStr::from(format!("cool {}", 123));
assert_eq!(ArcStr::as_static(&dynamic_arc), None);
```

Note that there's a better list of [benefits](https://docs.rs/arcstr/%2a/arcstr/struct.ArcStr.html#benefits-of-arcstr-over-arcstr) in the `ArcStr` documentation which covers some of the reasons you might want to use it over other alternatives.

## Usage

It's a normal rust crate, drop it in your `Cargo.toml` dep section. In the wildly unlikely case that you're here and don't kown how:

```toml
arcstr = { version = "...", features = ["any", "features", "you", "want"] }
```

The following cargo features are available. None are on by default currently.

- `std` (off by default). Turn on to use `std::process`'s aborting, instead of triggering an abort using the "double-panic trick".

    Essentially, there's one case we need to abort, and that's during a catastrophic error where you leak the same (dynamic) `ArcStr` 2^31 on 32-bit systems, or 2^63 in 64-bit systems. If this happens, we follow `libstd`'s lead and just abort because we're hosed anyway. If `std` is enabled, we use the real `std::process::abort`. If `std` is not enabled, we trigger an `abort` by triggering a panic while another panic is unwinding, which is either defined to cause an abort, or causes one in practice.

    In pratice you will never hit this edge case, and it still works in no_std, so no_std is the default. If you have to turn this on, because you hit this ridiculous case and found our handling bad, let me know

- `serde` enable serde serialization of `ArcStr`, off by default. Note that this doesn't do any fancy deduping or whatever.
