pub trait DumpTile {
    fn dump<W: std::fmt::Write>(&self, w: W) -> std::fmt::Result;
}

pub mod prettyprint;
pub mod tile_fields;
