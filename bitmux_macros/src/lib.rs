use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn bitenum(attr: TokenStream, inp: TokenStream) -> TokenStream {
    bitmux_macros_impl::bitenum::do_bitenum(attr.into(), inp.into()).into()
}

#[proc_macro]
pub fn twohot(inp: TokenStream) -> TokenStream {
    bitmux_macros_impl::twohot::do_twohot(inp.into()).into()
}
