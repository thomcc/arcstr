use ::proc_macro::{TokenTree as TT, *};

pub(super) fn compile_error(err_msg: &'_ str, span: Span) -> TokenStream {
    macro_rules! spanned {
        ($expr:expr) => {{
            let mut it = $expr;
            it.set_span(span);
            it
        }};
    }
    <TokenStream as ::std::iter::FromIterator<_>>::from_iter(vec![
        TT::Ident(Ident::new("compile_error", span)),
        TT::Punct(spanned!(Punct::new('!', Spacing::Alone))),
        TT::Group(spanned!(Group::new(
            Delimiter::Brace,
            TT::Literal(spanned!(Literal::string(err_msg))).into(),
        ))),
    ])
}
