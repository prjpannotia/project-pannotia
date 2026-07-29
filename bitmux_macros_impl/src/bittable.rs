//! Turn tables into a list of 2-D coordinates

use unsynn::*;

use crate::error_handling::*;

unsynn! {
    keyword KwX = "x";
    keyword KwY = "y";

    type PoundX = Cons<Pound, KwX>;
    type PoundY = Cons<Pound, KwY>;

    /// Helper enum to look for `#x`/`#y` so we can replace it. Matches any token.
    #[derive(Clone)]
    enum FindPoundXY {
        PoundX(PoundX),
        PoundY(PoundY),
        Group(GroupContaining<Vec<FindPoundXY>>),
        Else(TokenTree),
    }

    type DotOrNum = Either<Dot, LiteralInteger>;
    type NumberRow = Many<DotOrNum>;
    type NumbersTable = CommaDelimitedVec<NumberRow>;

    struct BittableInput {
        expr: LazyVec<FindPoundXY, Comma>,
        numbers: NumbersTable,
    }
}

fn replace_xy(replace_in: &mut [FindPoundXY], x: usize, y: usize) {
    for t in replace_in {
        match t {
            FindPoundXY::PoundX(_) => {
                *t = FindPoundXY::Else(TokenTree::Literal(Literal::usize_unsuffixed(x)))
            }
            FindPoundXY::PoundY(_) => {
                *t = FindPoundXY::Else(TokenTree::Literal(Literal::usize_unsuffixed(y)))
            }
            FindPoundXY::Group(group) => replace_xy(&mut group.content, x, y),
            FindPoundXY::Else(_) => {}
        }
    }
}

pub fn do_bittable(inp: TokenStream) -> TokenStream {
    let inp: BittableInput = match inp.to_token_iter().parse_all() {
        Ok(x) => x,
        Err(e) => return unsynn_err_to_lower_error(&e),
    };

    let mut bit_positions_list = Vec::new();
    for (y, row) in inp.numbers.iter().enumerate() {
        for (x, idx) in row.value.iter().enumerate() {
            if let Either::Second(idx) = &idx.value {
                let idx_span = idx.clone().into_inner().span();
                let idx_val = idx.value() as usize;

                // fill list, if necessary
                if idx_val >= bit_positions_list.len() {
                    bit_positions_list.resize(idx_val + 1, None);
                }

                // check for duplicates
                if let Some((x, y)) = bit_positions_list[idx_val] {
                    return err_with_span(
                        &format!("duplicate bit {idx_val}, previous at ({x}, {y})"),
                        idx_span,
                    );
                }

                bit_positions_list[idx_val] = Some((x, y));
            }
        }
    }

    // make sure everything is filled
    for (i, x) in bit_positions_list.iter().enumerate() {
        if x.is_none() {
            return err_with_span(&format!("missing bit {i}"), Span::call_site());
        }
    }
    let bit_positions_list = bit_positions_list
        .into_iter()
        .map(Option::unwrap)
        .collect::<Vec<_>>();

    // generate array
    let mut bit_array = TokenStream::new();
    for (x, y) in bit_positions_list {
        let mut this_expr = inp.expr.clone();
        replace_xy(&mut this_expr.vec, x, y);
        bit_array.extend(this_expr.to_token_iter());
    }

    quote! {
        [#bit_array]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bittable() {
        let inp = r#"
            Coordinate(#x, #y),
            1   3   2   0,
            7   5   4   6,
            9   11  10  8,
            .   13  12  14
        "#
        .to_token_stream();

        let outp = do_bittable(inp);
        assert_tokens_eq!(
            outp,
            r#"
            [
                Coordinate(3, 0), Coordinate(0, 0), Coordinate(2, 0), Coordinate(1, 0),
                Coordinate(2, 1), Coordinate(1, 1), Coordinate(3, 1), Coordinate(0, 1),
                Coordinate(3, 2), Coordinate(0, 2), Coordinate(2, 2), Coordinate(1, 2),
                Coordinate(2, 3), Coordinate(1, 3), Coordinate(3, 3),
            ]
            "#
        );
    }
}
