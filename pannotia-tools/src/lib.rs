pub trait DumpTile {
    fn dump<W: std::fmt::Write>(&self, w: W) -> std::fmt::Result;
}

pub trait PackerParse {
    fn try_parse(s: &str) -> Result<Self, ()>
    where
        Self: Sized;
}

pub mod parsing;
pub mod prettyprint;
pub mod tile_fields;
