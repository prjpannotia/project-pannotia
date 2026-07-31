//! General-purpose interconnect
//!
//! These muxes are found in logic, BRAM, and routing-only tiles.
//! The exact connectivity may vary slightly, but the routing is
//! generic enough that it makes sense to abstract it via a common interface.

use std::{
    borrow::{Borrow, BorrowMut},
    fmt::Display,
};

use super::*;

use bitmux::{BitGetter, BitSetter};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct GenericRoutingRef<D: DebugTracer, Ref: Borrow<Bitstream<D>>> {
    pub(super) r: Ref,
    pub(super) p: TilePos,
    pub(super) _d: PhantomData<D>,
}
impl<D: DebugTracer, Ref: Borrow<Bitstream<D>>> TileRefTrait<D, Ref> for GenericRoutingRef<D, Ref> {
    fn tile_type(&self) -> TileType {
        let family = self.r.borrow().family();
        family.get_tile_type(self.p)
    }
    fn pos(&self) -> TilePos {
        self.p
    }
    fn as_base_tile(self) -> TileRef<D, Ref> {
        TileRef {
            r: self.r,
            p: self.p,
            _d: PhantomData,
        }
    }
}

pub(crate) struct RMUXRef {
    pub(crate) is_bram: bool,
    pub(crate) i: u8,
}
impl FieldPositionCalculator for RMUXRef {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.i < 96, "RMUX index out of range");
        // There are 96 RMUXes per tile, which we _logically_ group as 16x 6,
        // but which are _physically_ grouped more like 4x 24.
        let group_of_24 = (self.i / 24) as u32;
        let group_within_24 = self.i % 24;

        // Each sub-group of 24 is 3 columns of 8 in this strange order.
        // The middle column also has its bits flipped.
        // Each mux is a 5 wide by 2 high block, so we _can_ start computing the
        // y coordinate now
        let (mut y_offs, col_of_three) = bitmux::bittable!(
            ((#y + group_of_24 * 8) * 2, #x),
            0       2       4,
            6       8       10,
            17      13      15,
            23      19      21,
            1       3       5,
            7       9       11,
            16      12      14,
            22      18      20,
        )[group_within_24 as usize];

        // These tiles all have a 4-row gap in the middle for global routing bits
        if self.i >= 48 {
            y_offs += 4;
        }

        // We can perform a basic lookup of the 5x2 bit block now
        let (y, xbase) = bitmux::bittable!(
            (y_offs + #y, #x),
            7   6   4   2   0,
            8   9   5   3   1,
        )[biti];

        // Finally deal with inverting the middle column
        let mut x = match col_of_three {
            0 => xbase,
            1 => 9 - xbase,
            2 => 10 + xbase,
            _ => unreachable!(),
        };

        // For a BRAM tile, this is scooted right by 1
        if self.is_bram {
            x += 1;
        }

        TileRelativeBitPos { y, x }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum RMUX {
    None,
    I(u8),
}
impl Default for RMUX {
    fn default() -> Self {
        Self::None
    }
}
impl Display for RMUX {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "<unset>"),
            Self::I(i) => write!(f, "#{i}"),
        }
    }
}
impl RMUX {
    fn from_bits(bits: u32) -> Self {
        bitmux::twohot!(3, 7, match bits {
            #bits => RMUX::I(#val),
            0 => RMUX::None,
            _ => panic!("invalid RMUX {bits:010b}"),
        })
    }

    fn to_bits(self) -> u32 {
        bitmux::twohot!(3, 7, match self {
            RMUX::I(#val) => #bits,
            RMUX::None => 0,
            _ => panic!("invalid RMUX {}", self),
        })
    }
}
impl bitmux::BitstreamField for RMUX {
    fn get(b: impl bitmux::BitGetter) -> Self {
        Self::from_bits(b.get_bits::<10>())
    }
    fn set(&self, mut b: impl bitmux::BitSetter) {
        b.set_bits::<10>(self.to_bits());
    }
}

pub trait GenericRoutingRefTrait {
    fn rmux(&self, rmux_idx: u8) -> RMUX;
}
pub trait GenericRoutingRefMutTrait: GenericRoutingRefTrait {
    fn set_rmux(&mut self, rmux_idx: u8, val: RMUX);
}

impl<D: DebugTracer, Ref: Borrow<Bitstream<D>>> GenericRoutingRefTrait
    for GenericRoutingRef<D, Ref>
{
    fn rmux(&self, rmux_idx: u8) -> RMUX {
        let is_bram = self.tile_type() == TileType::BRAM;
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: RMUXRef {
                is_bram,
                i: rmux_idx,
            },
            _d: PhantomData,
        };
        RMUX::from_bits(ref_.get_bits::<10>())
    }
}
impl<D: DebugTracer, Ref: BorrowMut<Bitstream<D>>> GenericRoutingRefMutTrait
    for GenericRoutingRef<D, Ref>
{
    fn set_rmux(&mut self, rmux_idx: u8, val: RMUX) {
        let is_bram = self.tile_type() == TileType::BRAM;
        let mut ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: RMUXRef {
                is_bram,
                i: rmux_idx,
            },
            _d: PhantomData,
        };
        ref_.set_bits::<10>(val.to_bits());
    }
}
