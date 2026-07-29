//! Logic tiles

use std::borrow::{Borrow, BorrowMut};

use super::generic_routing::{GenericRoutingRefMutTrait, GenericRoutingRefTrait, RMUX, RMUXRef};
use super::*;

use bitmux::{BitGetter, BitSetter};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct LogicTileRef<D: DebugTracer, Ref: Borrow<Bitstream<D>>> {
    pub(super) r: Ref,
    pub(super) p: TilePos,
    pub(super) _d: PhantomData<D>,
}
impl<D: DebugTracer, Ref: Borrow<Bitstream<D>>> TileRefTrait<D, Ref> for LogicTileRef<D, Ref> {
    fn tile_type(&self) -> TileType {
        TileType::Logic
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

struct LogicLUT(u8);
impl FieldPositionCalculator for LogicLUT {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 16, "LUT index out of range");
        bitmux::bittable!(
            TileRelativeBitPos {
                x: 27 + #x,
                y: #y + self.0 as u32 * 4 + if self.0 >= 8 { 4 } else { 0 }
            },
            1   3   2   0,
            7   5   4   6,
            9   11  10  8,
            15   13  12  14
        )[biti]
    }
}
impl<D: DebugTracer, Ref: Borrow<Bitstream<D>>> LogicTileRef<D, Ref> {
    pub fn lut(&self, lut_idx: u8) -> u16 {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: LogicLUT(lut_idx),
            _d: PhantomData,
        };
        ref_.get_bits::<16>() as u16
    }
}
impl<D: DebugTracer, Ref: BorrowMut<Bitstream<D>>> LogicTileRef<D, Ref> {
    pub fn set_lut(&mut self, lut_idx: u8, val: u16) {
        let mut ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: LogicLUT(lut_idx),
            _d: PhantomData,
        };
        ref_.set_bits::<16>(val as u32)
    }
}

impl<D: DebugTracer, Ref: Borrow<Bitstream<D>>> GenericRoutingRefTrait for LogicTileRef<D, Ref> {
    fn rmux(&self, rmux_idx: u8) -> RMUX {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: RMUXRef {
                is_bram: false,
                i: rmux_idx,
            },
            _d: PhantomData,
        };
        RMUX::from_bits(ref_.get_bits::<10>())
    }
}
impl<D: DebugTracer, Ref: BorrowMut<Bitstream<D>>> GenericRoutingRefMutTrait
    for LogicTileRef<D, Ref>
{
    fn set_rmux(&mut self, rmux_idx: u8, val: RMUX) {
        let mut ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: RMUXRef {
                is_bram: false,
                i: rmux_idx,
            },
            _d: PhantomData,
        };
        ref_.set_bits::<10>(val.to_bits());
    }
}
