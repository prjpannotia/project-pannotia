//! Error handling helpers

use unsynn::*;

pub fn tokeniter_to_span(mut tok_iter: TokenIter) -> Span {
    let span0 = match tok_iter.next() {
        Some(t) => t.span(),
        None => Span::call_site(),
    };
    tok_iter.fold(span0, |s, t| s.join(t.span()).unwrap_or(s))
}

pub fn err_with_span(err: &str, span: Span) -> TokenStream {
    let err_lit = Literal::string(err);
    let mut ret = quote! {
        compile_error!(#err_lit);
    }
    .to_token_iter()
    .collect::<Vec<_>>();
    for x in &mut ret {
        x.set_span(span);
    }
    ret.to_token_stream()
}

pub fn unsynn_err_to_lower_error(e: &unsynn::Error) -> TokenStream {
    let msg = format!("{}", e);
    let span = e.failed_at().map_or(Span::call_site(), |x| x.span());
    err_with_span(&msg, span)
}
