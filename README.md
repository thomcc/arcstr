
A better reference-counted string type.

Or, really the intent is for it to have a couple of those. It just has one at the moment: `ArcStr`, which has the following benefits over `Arc<str>`, and such:

- Only a single pointer. Great for cases where you want to keep the data structure lightweight or need to do some FFI stuff with it or who knows.

- It's possible to create a const `ArcStr` from a literal string constant.

  These are zero cost, take no heap allocation, and don't even need to perform atomic reads/writes when being cloned or dropped (or at any other time). They even get stored in the read-only memory of your executable, which can be beneficial for performance and memory usage.

  That said, I won't lie to you: the API for this is... a bit of a janky macro: `unsafe { literal_arcstr!(b"stuff") };`. Thing is, I can't verify UTF-8 validity and stay `const`, and various details mean I need a bytestring literal like `b"..."` which unfortunately means it could be non-utf8.

- That said, `ArcStr::new()` is a `const` function, which isn't true of e.g. `Arc<str>`, which actually has to heap allocate for each default-initialized string. This shouldn't be surprising given the macro I mentioned. Naturally, this means that `ArcStr::default()` is free too. That said, this doesn't make us that special, as most types in libstd get it right, it's just `Arc` that can't.

- `ArcStr` is totally immutable. No more need to lose sleep over code that thinks it has a right to mutate your `Arc` just because it holds the only reference. This is deliberate and IMO a feature... but I can see why some might want to frame it as a negative.

- More implementations of various traits like `PartialEq<Other>` and friends than `Arc<str>` has AFAIK. That is, sometimes `Arc<str>`'s ergonomics feel a bit off, but I'm hoping that doesnt happen here.

- We don't support `Weak` references, which means the overhead of atomic operations is lower. This is also a "Well, it's a feature to *me*" situation...

## Planned funtionality

So right, yeah, I did mention that "really the intent is for the crate to have a couple of those". What did I mean by that? Well, there are a few things you can build on `ArcStr` in not much code that are pretty nice:

### `Substr` Type

Essentially an ergonomic `(ArcStr, Range<usize>)`, which can be used to
avoid allocation when creating a lot of ranges over the same string. A use
case for this is parsers and lexers (Note that in practice I might use
`Range<u32>` and not `Range<usize>`).

### `Key` type

Essentially this will be an 8-byte wrapper around `ArcStr` that allows
storing small 7b-or-fewer strings inline, without allocation. It will be 8
bytes on 32-bit and 64-bit platforms, since 3b-or-fewer is not compelling.

Actually, I need to do some invesigation that 7b isn't too small too. The
idea is for use as map keys or other small frequently repeated identifiers.


