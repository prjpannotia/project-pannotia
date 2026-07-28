//! Common elements of Rust grammar used throughout

use unsynn::*;

unsynn! {
    pub keyword KwEnum = "enum";
    pub keyword KwPub = "pub";
    pub keyword KwCrate = "crate";

    /// An outer attribute (`#[thing]`)
    pub type AnyOuterAttribute = Cons<Pound, BracketGroupContaining<TokenStream>>;

    /// A visibility specifier which must begin with `pub`
    pub type PubVisibility = Cons<KwPub, Option<ParenthesisGroupContaining<TokenStream>>>;

    /// A visibility specifier
    pub type Visibility = Option<PubVisibility>;

    /// Consume all tokens until a comma or end-of-stream
    pub type AnyUntilComma = LazyVecUntil<TokenTree, Either<Comma, EndOfStream>>;
}
