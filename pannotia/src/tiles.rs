use std::borrow::{Borrow, BorrowMut};

use crate::coordinates::TilePos;

/// The kind of tile that exists at a given position
///
///
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum TileType {
    /// There is no tile here
    ///
    /// Places where there might not be a tile include the corners
    /// and holes left by embedded microcontroller hard blocks.
    ///
    /// This also includes "logical" tiles used by the vendor software
    /// which do not have any actual configuration bits,
    /// such as a "clock distribution" tile.
    None,

    /// A logic tile, containing LUTs
    Logic,

    /// A tile which only contains routing
    ///
    /// This is found on the right-hand side of the chip.
    RoutingOnly,

    /// A block RAM tile
    BRAM,

    /// IO, on the top and bottom sides
    TopBottomIO,
    /// IO, on the left and right sides
    LeftRightIO,

    /// Special function interface, on the top and bottom sides
    ///
    /// Note that "top" and "bottom" are relative to "the rest of the logic fabric"
    /// and are not the top and bottom of the entire tile grid.
    /// In fact, the bottom side of the MCU is interfaced with `TopBottomIP` tiles,
    /// and these tiles are found in a row in the middle of the tile grid.
    TopBottomIP,
    /// Special function interface, on the left and right sides
    ///
    /// See the note for [TopBottomIP](Self::TopBottomIP).
    /// These tiles can be found both in the middle (for interfacing with the MCU)
    /// and on the actual tile grid boundary (for interfacing with analog IP).
    LeftRightIP,

    /// Tile containing a PLL
    PLL,

    /// Tile controlling global clock distribution
    GCLKSW,
}

pub trait TileRefTrait {
    fn tile_type(&self) -> TileType;
}

pub struct TileRef<Ref: Borrow<crate::container::Bitstream>> {
    r: Ref,
    p: TilePos,
}
impl<Ref: Borrow<crate::container::Bitstream>> TileRefTrait for TileRef<Ref> {
    fn tile_type(&self) -> TileType {
        let family = self.r.borrow().family();
        family.get_tile_type(self.p)
    }
}
impl<Ref: Borrow<crate::container::Bitstream>> TileRef<Ref> {
    pub(crate) fn new(r: Ref, p: TilePos) -> Self {
        Self { r, p }
    }

    pub fn as_logic_tile(self) -> LogicTileRef<Ref> {
        LogicTileRef {
            r: self.r,
            p: self.p,
        }
    }
}

pub struct LogicTileRef<Ref: Borrow<crate::container::Bitstream>> {
    r: Ref,
    p: TilePos,
}
impl<Ref: Borrow<crate::container::Bitstream>> LogicTileRef<Ref> {
    pub fn lut(&self) -> u16 {
        let bits = self.r.borrow();
        // bits.get_logic_array_bit(123, 456);
        12345
    }
}
impl<Ref: BorrowMut<crate::container::Bitstream>> LogicTileRef<Ref> {
    pub fn set_lut(&mut self, val: u16) {
        let bits = self.r.borrow_mut();
        // bits.set_logic_array_bit(123, 456, val == 12345);
    }
}
