//! Generate two-hot muxes

use unsynn::*;

use crate::error_handling::*;

unsynn! {
    keyword KwMatch = "match";
    keyword KwBits = "bits";
    keyword KwVal = "val";

    type PoundBits = Cons<Pound, KwBits>;
    type PoundVal = Cons<Pound, KwVal>;

    /// Helper enum to look for `#val` so we can replace it. Matches any token.
    #[derive(Clone)]
    enum FindPoundVal {
        HashVal(PoundVal),
        Group(GroupContaining<Vec<FindPoundVal>>),
        Else(TokenTree),
    }

    /// `#bits => /* some expr */ ,`
    struct BitsToValue {
        _bits: PoundBits,
        _arrow: FatArrow,
        val: LazyVec<FindPoundVal, Comma>,
    }
    /// `/* some expr */ => #bits ,`
    struct ValueToBits {
        val: LazyVec<FindPoundVal, FatArrow>,
        _bits: PoundBits,
        _comma: Comma,
    }

    /// Matches the inside of a two-hot mux specification:
    ///
    /// ```ignore
    /// twohot!(7, 3, match expr {
    ///     #bits => some_expr_involving(#val),
    ///     /* or */
    ///     #val if some_blah_extra => #bits,
    ///
    ///     // Other arms
    ///     _ => asdf
    /// })
    /// ```
    struct TwohotMux {
        sel1: LiteralInteger,
        _comma1: Comma,
        sel2: LiteralInteger,
        _comma2: Comma,
        _kw_match: KwMatch,
        expr: LazyVecUntil<TokenTree, BraceGroup>,
        inside: BraceGroupContaining<Cons<Either<BitsToValue, ValueToBits>, TokenStream>>,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dir {
    BitsToValue,
    ValueToBits,
}

fn replace_pound_val(replace_in: &mut [FindPoundVal], replace_with: u128) {
    for x in replace_in {
        match x {
            FindPoundVal::HashVal(_) => {
                *x = FindPoundVal::Else(TokenTree::Literal(format_literal!("{replace_with}")));
            }
            FindPoundVal::Group(group) => {
                replace_pound_val(&mut group.content, replace_with);
            }
            FindPoundVal::Else(_) => {}
        }
    }
}

pub fn do_twohot(inp: TokenStream) -> TokenStream {
    let inp: TwohotMux = match inp.to_token_iter().parse_all() {
        Ok(x) => x,
        Err(e) => return unsynn_err_to_lower_error(&e),
    };

    // Pull apart the parsed data
    let (dir, val_stuff) = match inp.inside.content.first {
        Either::First(b2v) => (Dir::BitsToValue, b2v.val.vec),
        Either::Second(v2b) => (Dir::ValueToBits, v2b.val.vec),
        _ => unreachable!(),
    };

    let outer_sel_num = inp.sel1.value();
    let inner_sel_num = inp.sel2.value();

    let match_expr = &inp.expr;
    let other_match_arms = inp.inside.content.second;
    let mut twohot_match_arms = TokenStream::new();

    for outer_sel_idx in 0..outer_sel_num {
        for inner_sel_idx in 0..inner_sel_num {
            // "outer" sel = higher bits
            // "inner" sel = lower bits
            let sel_bit_pattern = (1 << inner_sel_idx) | (1 << (outer_sel_idx + inner_sel_num));
            let sel_bit_pattern = format_literal!(
                "0b{sel_bit_pattern:0width$b}",
                width = (outer_sel_num + inner_sel_num) as usize
            );

            let sel_numeric_value = outer_sel_idx * inner_sel_num + inner_sel_idx;
            let mut val_stuff = val_stuff.clone();
            replace_pound_val(&mut val_stuff, sel_numeric_value);

            if dir == Dir::BitsToValue {
                twohot_match_arms.extend(quote! {
                    #sel_bit_pattern => #val_stuff,
                });
            } else {
                twohot_match_arms.extend(quote! {
                    #val_stuff => #sel_bit_pattern,
                });
            }
        }
    }

    quote! {
        match #match_expr {
            #twohot_match_arms
            #other_match_arms
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_twohot_dec() {
        let inp = r#"
            2, 3, match value {
                #bits => asdf(#val + 1),
            }
        "#
        .to_token_stream();

        let outp = do_twohot(inp);
        assert_tokens_eq!(
            outp,
            r#"
            match value {
                0b01001 => asdf(0 + 1),
                0b01010 => asdf(1 + 1),
                0b01100 => asdf(2 + 1),
                0b10001 => asdf(3 + 1),
                0b10010 => asdf(4 + 1),
                0b10100 => asdf(5 + 1),
            }
            "#
        );
    }

    #[test]
    fn test_basic_twohot_enc() {
        let inp = r#"
            3, 2, match value {
                #val if 123 != 456 => #bits,
            }
        "#
        .to_token_stream();

        let outp = do_twohot(inp);
        assert_tokens_eq!(
            outp,
            r#"
            match value {
                0 if 123 != 456 => 0b00101,
                1 if 123 != 456 => 0b00110,
                2 if 123 != 456 => 0b01001,
                3 if 123 != 456 => 0b01010,
                4 if 123 != 456 => 0b10001,
                5 if 123 != 456 => 0b10010,
            }
            "#
        );
    }
}
