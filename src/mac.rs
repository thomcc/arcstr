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
