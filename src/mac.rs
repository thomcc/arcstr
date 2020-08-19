/// Create a const [`ArcStr`](crate::ArcStr) from a string literal. The
/// resulting `ArcStr` require no heap allocation, can be freely cloned and used
/// interchangeably with `ArcStr`s from the heap, and are effectively "free".
///
/// The main downside is that it's a macro. Eventually it may be doable as a
/// `const fn`, which would be cleaner, but for now the drawbacks to this are
/// not overwhelming, and the functionality it provides is very useful.
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
            const S: $crate::ArcStr = unsafe { $crate::ArcStr::_private_new_from_static_data(SI) };
            S
        }
    }};
}

/// Conceptually equivalent to `ArcStr::from(format!("...", args...))`.
///
/// Currently, the only difference here is that when used with no formatting
/// args, this behaves equivalently to [`arcstr::literal!`](crate::literal).
///
/// In the future, this will be implemented in such a way to avoid an additional
/// string copy which is required by the `from` operation.
///
/// # Example
///
/// ```
/// let arcstr = arcstr::format!("testing {}", 123);
/// assert_eq!(arcstr, "testing 123");
/// ```
#[macro_export]
macro_rules! format {
    ($text:expr $(,)?) => {
        // prevent use as const, but keep cheap.
        $crate::ArcStr::from($crate::literal!($text))
    };
    ($text:expr, $($toks:tt)+) => {
        $crate::ArcStr::from($crate::alloc::fmt::format(::core::format_args!($text, $($toks)+)))
    };
}

/// `feature = "substr"`: Create a `const` [`Substr`][crate::Substr].
///
/// This is a wrapper that initializes a `Substr` over the entire contents of a
/// `const` [`ArcStr`](crate::ArcStr) made using [arcstr::literal!](crate::literal).
///
/// As with `arcstr::literal`, these require no heap allocation, can be freely
/// cloned and used interchangeably with `ArcStr`s from the heap, and are
/// effectively "free".
///
/// The main use case here is in applications where `Substr` is a much more
/// common string type than `ArcStr`.
///
/// # Examples
///
/// ```
/// use arcstr::{Substr, literal_substr};
/// // Works in const:
/// const EXAMPLE_SUBSTR: Substr = literal_substr!("testing testing");
/// assert_eq!(EXAMPLE_SUBSTR, "testing testing");
///
/// // Or, just in normal expressions.
/// assert_eq!("Wow!", literal_substr!("Wow!"));
/// ```
#[macro_export]
#[cfg(feature = "substr")]
macro_rules! literal_substr {
    ($text:expr) => {{
        const __S: &str = $text;
        {
            const PARENT: $crate::ArcStr = $crate::literal!(__S);
            const SUBSTR: $crate::Substr =
                unsafe { $crate::Substr::from_parts_unchecked(PARENT, 0..__S.len()) };
            SUBSTR
        }
    }};
}

#[cfg(test)]
mod test {
    #[test]
    fn ensure_no_import() {
        let v = literal!("foo");
        assert_eq!(v, "foo");
        #[cfg(feature = "substr")]
        {
            let substr = literal_substr!("bar");
            assert_eq!(substr, "bar");
        }
        // Loom doesn't like it if you do things outside `loom::model`, AFAICT.
        // These calls produce error messages from inside `libstd` about
        // accessing thread_locals that haven't been initialized.
        #[cfg(not(loom))]
        {
            let test = crate::format!("foo");
            assert_eq!(test, "foo");
            let test2 = crate::format!("foo {}", 123);
            assert_eq!(test2, "foo 123");
        }
    }
}
