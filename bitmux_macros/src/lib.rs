use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn bitenum(_attr: TokenStream, inp: TokenStream) -> TokenStream {
    bitmux_macros_impl::bitenum::do_bitenum(_attr.into(), inp.into()).into()
}
