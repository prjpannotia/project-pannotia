//! Handle `#[bitenum]` macro

use unsynn::*;

use crate::error_handling::*;
use crate::grammar_common::*;

unsynn! {
    keyword KwErr = "err";

    /// An enum default, of the following syntax:
    ///
    /// ```ignore
    /// err = {
    ///     // stuff
    /// }
    /// ```
    struct EnumDefault {
        _err: KwErr,
        _eq: Assign,
        stuff: AnyUntilComma,
    }

    /// One enum value, of the following syntax:
    ///
    /// ```ignore
    /// #[attributes_such_as_doc]
    /// VariantOne = "0011xxXX"
    /// ```
    struct EnumItem {
        attr: Vec<AnyOuterAttribute>,
        ident: Ident,
        _eq: Assign,
        value: LiteralString,
    }

    /// One bitstream enum field, of the following syntax:
    ///
    /// ```ignore
    /// #[attributes_such_as_doc]
    /// #[bitmux::bitenum]
    /// pub enum ExampleSetting {
    ///     OptionA = "00",
    ///     OptionB = "01",
    ///     OptionC = "1x",
    /// }
    /// ```
    struct BitEnum {
        attr: Vec<AnyOuterAttribute>,
        vis: Visibility,
        _kw_enum: KwEnum,
        ident: Ident,
        inner: BraceGroupContaining<CommaDelimitedVec<Either<EnumDefault, EnumItem>>>,
    }

    // The following is used for settings only

    /// `crate = ::path`
    struct SettingCratePath {
        _kw_crate: KwCrate,
        _eq: Assign,
        path: AnyUntilComma,
    }

    /// any `foo = bar` setting
    enum Setting {
        CratePath(SettingCratePath),
    }

    type SettingsList = CommaDelimitedVec<Setting>;
}

/// Configuration settings (arguments to the macro)
#[derive(Debug)]
struct Settings {
    crate_path: TokenStream,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            crate_path: quote! { ::bitmux },
        }
    }
}

impl TryFrom<TokenStream> for Settings {
    type Error = TokenStream;

    fn try_from(value: TokenStream) -> std::result::Result<Self, TokenStream> {
        let settings_list: SettingsList = match value.to_token_iter().parse_all() {
            Ok(x) => x,
            Err(e) => return Err(unsynn_err_to_lower_error(&e)),
        };

        let mut ret = Self::default();
        for setting in settings_list {
            let setting = setting.value;
            match setting {
                Setting::CratePath(setting_crate_path) => {
                    ret.crate_path = setting_crate_path.path.vec.to_token_stream();
                }
            }
        }

        Ok(ret)
    }
}

pub fn do_bitenum(attr: TokenStream, inp: TokenStream) -> TokenStream {
    let settings = match Settings::try_from(attr) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let crate_path = &settings.crate_path;

    let inp: BitEnum = match inp.to_token_iter().parse_all() {
        Ok(x) => x,
        Err(e) => return unsynn_err_to_lower_error(&e),
    };

    let enum_attr = &inp.attr;
    let enum_vis = &inp.vis;
    let enum_ident = &inp.ident;

    // This will go inside the generated output enum
    let mut enum_variants = TokenStream::new();
    let mut enum_get_match_arms = TokenStream::new();
    let mut enum_set_match_arms = TokenStream::new();
    let mut default_code = TokenStream::new();
    let mut nbits = None;

    for var in inp.inner.content {
        let var = &var.value;
        match var {
            Either::First(def) => {
                if !default_code.is_empty() {
                    return err_with_span(
                        "cannot have multiple err blocks",
                        tokeniter_to_span(def.to_token_iter()),
                    );
                }
                default_code = def.stuff.to_token_stream();
            }
            Either::Second(var) => {
                let var_attr = &var.attr;
                let var_ident = &var.ident;
                let var_value = var.value.as_str();
                let var_value_span = var.value.clone().into_inner().span();

                // Validate the bit length
                match nbits {
                    Some(nbits) => {
                        if nbits != var_value.len() {
                            return err_with_span(
                                &format!("wrong number of bits (expected {})", nbits),
                                var_value_span,
                            );
                        }
                    }
                    None => nbits = Some(var_value.len()),
                }

                // Store the ident
                enum_variants.extend(quote! {
                    #var_attr #var_ident ,
                });

                // process the bit pattern
                let mut match_mask = 0u32;
                let mut match_val = 0u32;
                let mut write_val = 0u32;
                let mut has_dont_care = false;
                for b in var_value.chars() {
                    match_mask <<= 1;
                    match_val <<= 1;
                    write_val <<= 1;
                    match b {
                        '0' => {
                            match_mask |= 1;
                        }
                        '1' => {
                            match_mask |= 1;
                            match_val |= 1;
                            write_val |= 1;
                        }
                        'x' => {
                            // Write as 0, read as don't care
                            has_dont_care = true;
                        }
                        'X' => {
                            // Write as 1, read as don't care
                            has_dont_care = true;
                            write_val |= 1;
                        }
                        _ => return err_with_span(&format!("invalid bit '{}'", b), var_value_span),
                    }
                }

                // generate "get" code
                if !has_dont_care {
                    enum_get_match_arms.extend(quote! {
                        #match_val => Self::#var_ident,
                    });
                } else {
                    enum_get_match_arms.extend(quote! {
                        _ if bits & #match_mask == #match_val => Self::#var_ident,
                    });
                }

                // generate "set" code
                enum_set_match_arms.extend(quote! {
                    Self::#var_ident => #crate_path::BitSetter::set_bits::<#nbits>(&mut s, #write_val),
                });
            }
            _ => unreachable!(),
        }
    }

    if nbits.is_none() {
        return err_with_span("cannot have zero variants", enum_ident.span());
    }
    let nbits = nbits.unwrap();

    let default_handling = if default_code.is_empty() {
        quote! { _ => panic!("invalid bit pattern {bits:0width$b}", width = #nbits) }
    } else {
        quote! { _ => #default_code }
    };

    quote! {
        #enum_attr
        #enum_vis enum #enum_ident {
            #enum_variants
        }

        impl #crate_path::BitstreamField for #enum_ident {
            fn get(g: impl #crate_path::BitGetter) -> Self {
                let bits = #crate_path::BitGetter::get_bits::<#nbits>(&g);
                match bits {
                    #enum_get_match_arms
                    #default_handling
                }
            }
            fn set(&self, mut s: impl #crate_path::BitSetter) {
                match self {
                    #enum_set_match_arms
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_bitenum() {
        let inp = r#"
            /// Example doc comment for the whole enum
            pub enum TestEnum {
                /// Docs for A
                VarA = "00",
                VarB = "01",
                VarC = "1x",
            }
        "#
        .to_token_stream();

        let outp = do_bitenum(TokenStream::new(), inp);
        assert_tokens_eq!(
            outp,
            r#"
            /// Example doc comment for the whole enum
            pub enum TestEnum {
                /// Docs for A
                VarA,
                VarB,
                VarC,
            }

            impl ::bitmux::BitstreamField for TestEnum {
                fn get(g: impl ::bitmux::BitGetter) -> Self {
                    let bits = ::bitmux::BitGetter::get_bits:: <2>(&g);
                    match bits {
                        0 => Self::VarA,
                        1 => Self::VarB,
                        _ if bits & 2 == 2 => Self::VarC,
                        _ => panic!("invalid bit pattern {bits:0width$b}", width = 2)
                    }
                }
                fn set(&self, mut s: impl ::bitmux::BitSetter) {
                    match self {
                        Self::VarA => ::bitmux::BitSetter::set_bits:: <2>(&mut s, 0),
                        Self::VarB => ::bitmux::BitSetter::set_bits:: <2>(&mut s, 1),
                        Self::VarC => ::bitmux::BitSetter::set_bits:: <2>(&mut s, 2),
                    }
                }
            }
            "#
        );
    }
}
