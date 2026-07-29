//! Code for accessing every kind of FPGA tile
//!
//! In order to allow API users to refer to a specific tile
//! as a first-class object (rather than passing in coordinates every time),
//! every kind of tile has its own `SomeKindOfTileRef` struct
//! containing a reference to the bitstream itself plus the tile's coordinate.
//! This is abstracted over mutability by using the [Borrow]/[BorrowMut](std::borrow::BorrowMut) traits.
//!
//! The "generic" tile reference is [TileRef], and it is constructed by calling
//! [Bitstream::tile{_mut}](Bitstream::tile)

use std::borrow::Borrow;
use std::marker::PhantomData;

use crate::container::{Bitstream, DebugTracer};
use crate::coordinates::*;

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

/// Functions common to all tile references
pub trait TileRefTrait<D: DebugTracer, Ref: Borrow<Bitstream<D>>> {
    /// Get the type of the current tile
    fn tile_type(&self) -> TileType;
    /// Get the position of the current tile
    fn pos(&self) -> TilePos;
    /// Downcast this back to a generic tile reference
    fn as_base_tile(self) -> TileRef<D, Ref>;
}

/// Generic reference to a tile
///
/// This can be coerced to a more-specific reference type
/// using the `as_*` functions. These functions all panic if the
/// tile type is not as expected. The tile type can be validated
/// by first calling [tile_type](Self::tile_type).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct TileRef<D: DebugTracer, Ref: Borrow<Bitstream<D>>> {
    r: Ref,
    p: TilePos,
    _d: PhantomData<D>,
}
impl<D: DebugTracer, Ref: Borrow<Bitstream<D>>> TileRefTrait<D, Ref> for TileRef<D, Ref> {
    fn tile_type(&self) -> TileType {
        let family = self.r.borrow().family();
        family.get_tile_type(self.p)
    }
    fn pos(&self) -> TilePos {
        self.p
    }
    fn as_base_tile(self) -> TileRef<D, Ref> {
        self
    }
}

impl<D: DebugTracer, Ref: Borrow<Bitstream<D>>> TileRef<D, Ref> {
    pub(crate) fn new(r: Ref, p: TilePos) -> Self {
        Self {
            r,
            p,
            _d: PhantomData,
        }
    }

    /// Coerce to a reference to a generic routing tile
    #[inline]
    pub fn as_generic_routing_tile(self) -> generic_routing::GenericRoutingRef<D, Ref> {
        let tile_type = self.tile_type();
        assert!(
            tile_type == TileType::Logic
                || tile_type == TileType::RoutingOnly
                || tile_type == TileType::BRAM
        );
        generic_routing::GenericRoutingRef {
            r: self.r,
            p: self.p,
            _d: PhantomData,
        }
    }

    /// Coerce to a reference to a logic tile
    #[inline]
    pub fn as_logic_tile(self) -> logic::LogicTileRef<D, Ref> {
        assert!(self.tile_type() == TileType::Logic);
        logic::LogicTileRef {
            r: self.r,
            p: self.p,
            _d: PhantomData,
        }
    }
}

pub mod generic_routing;
pub mod logic;
