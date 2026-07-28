use unsynn::*;

use crate::grammar_common::*;

unsynn! {
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
        inner: BraceGroupContaining<CommaDelimitedVec<EnumItem>>,
    }
}

pub fn do_bitenum(_attr: TokenStream, inp: TokenStream) -> TokenStream {
    let inp: BitEnum = inp.to_token_iter().parse_all().unwrap();
    dbg!(&inp);

    let enum_attr = &inp.attr;
    let enum_vis = &inp.vis;
    let enum_ident = &inp.ident;

    // This will go inside the generated output enum
    let mut enum_variants = TokenStream::new();

    for var in inp.inner.content {
        let var = &var.value;
        let var_attr = &var.attr;
        let var_ident = &var.ident;
        let var_value = var.value.as_str();

        enum_variants.extend(quote! {
            #var_attr #var_ident ,
        });
    }

    quote! {
        #enum_attr
        #enum_vis enum #enum_ident {
            #enum_variants
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
            "#
        );
    }
}
