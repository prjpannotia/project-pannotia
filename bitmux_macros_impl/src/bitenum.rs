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
    dbg!(&inp);

    let enum_attr = &inp.attr;
    let enum_vis = &inp.vis;
    let enum_ident = &inp.ident;

    // This will go inside the generated output enum
    let mut enum_variants = TokenStream::new();
    let mut default_code = TokenStream::new();

    for var in inp.inner.content {
        let var = &var.value;
        match var {
            Either::First(def) => {
                if !default_code.is_empty() {
                    return err_with_span(
                        "cannot have multiple default blocks",
                        tokeniter_to_span(def.to_token_iter()),
                    );
                }
                default_code = def.stuff.to_token_stream();
            }
            Either::Second(var) => {
                let var_attr = &var.attr;
                let var_ident = &var.ident;
                let var_value = var.value.as_str();

                enum_variants.extend(quote! {
                    #var_attr #var_ident ,
                });
            }
            _ => unreachable!(),
        }
    }

    quote! {
        #enum_attr
        #enum_vis enum #enum_ident {
            #enum_variants
        }

        // FIXME crate path
        impl #crate_path::BitEnum for #enum_ident {
            fn get(g: impl #crate_path::BitGetter) -> Self {
                todo!()
            }
            fn set(&self, mut s: impl #crate_path::BitSetter) {
                todo!()
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
                err = foo,
            }
        "#
        .to_token_stream();

        let outp = do_bitenum(TokenStream::new(), inp);
        dbg!(&outp);
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

            impl ::bitmux::BitEnum for TestEnum {
                fn get(g: impl ::bitmux::BitGetter) -> Self {
                    todo!()
                }
                fn set(&self, mut s: impl ::bitmux::BitSetter) {
                    todo!()
                }
            }
            "#
        );
    }
}
