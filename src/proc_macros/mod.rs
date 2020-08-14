extern crate proc_macro;

use self::helpers::compile_error;
use ::proc_macro::{TokenTree as TT, *};

mod helpers;

#[proc_macro]
pub fn byte_lit(mut input: TokenStream) -> TokenStream {
    let (first, snd) = loop {
        let mut iter = input.into_iter();
        let first = iter.next();
        match first {
            Some(TT::Group(g)) if g.delimiter() == Delimiter::None => {
                input = g.stream();
                continue;
            }
            _ => {}
        }
        break (first, iter.next());
    };
    let mut storage = None;
    match (first, snd) {
        (None, _) => compile_error("Missing parameter", Span::call_site()),
        (_, Some(unexpected)) => compile_error("Unexpected token", unexpected.span()),
        (Some(TT::Literal(lit)), _)
            if {
                let s = storage.get_or_insert(lit.to_string());
                s.starts_with('"') && s.ends_with('"') // is a string literal
            } =>
        {
            let ref s = storage.unwrap();
            let value: &str = &s[1..(s.len() - 1)]; // string literal contents
            TT::Literal(Literal::byte_string(value.as_bytes())).into()
        }
        (Some(invalid_tt), _) => {
            compile_error("Expected a string literal", invalid_tt.span())
        }
    }
}
