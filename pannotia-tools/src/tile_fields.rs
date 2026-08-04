//! Traits, macros, etc for going through everything in a tile

use pannotia::prelude::*;

// *MUST* import all the traits!
use crate::prettyprint::*;

/// Replace a "bad" `self` token with a "good" `self` token
macro_rules! _replace_self_tok {
    ($desired_self:ident self) => {
        $desired_self
    };
    ($desired_self:ident $tok:tt) => {
        $tok
    };
}

/// Replace _all_ "bad" `self` tokens with a "good" `self` token
///
/// This is needed to work around some weird hygiene rules
macro_rules! _replace_self {
    ($desired_self:ident $($toks:tt)*) => {
        $(_replace_self_tok!($desired_self $toks))*
    };
}

macro_rules! _dump_one_tile_field {
    ($self:ident, $w:ident, $fn_str:literal, $fn_name:ident, $i:expr) => {
        let setting = $self.$fn_name($i);
        if setting != Default::default() {
            write!($w, "tile[{}].{}[{}] = ", $self.pos(), $fn_str, $i)?;
            (&&PrettyPrintWrap(setting)).pretty_print(
                &mut $w,
                $self.family(),
                $self.pos(),
                $self.tile_type(),
                $i,
            )?;
            writeln!($w)?;
        }
    };
}

macro_rules! make_tile_fields {
    ($self:ident: $($tile_ref:ident)::+ { $($func:ident $human_name:literal = $($count:literal)? $({ $($count_complex:tt)* })? ;)* }) => {
        impl<
            D: debug::DebugTracer,
            Ref: std::borrow::Borrow<Bitstream<D>>,
        > crate::DumpTile for $($tile_ref)::+<D, Ref>
        {
            fn dump<W: std::fmt::Write>(&$self, mut w: W) -> std::fmt::Result {
                $({
                    let count = $($count)? $(_replace_self!{ $self { $($count_complex)* } })?;
                    for i in 0..count {
                        _dump_one_tile_field!($self, w, $human_name, $func, i);
                    }
                })*

                Ok(())
            }
        }
    };
}

make_tile_fields! {
    self: tile::GenericRoutingRef {
        rmux "rmux" = 96;
    }
}

make_tile_fields! {
    self: tile::LogicTileRef {
        rmux "rmux" = 96;
    }
}
