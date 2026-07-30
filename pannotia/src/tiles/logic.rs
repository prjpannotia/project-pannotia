//! Logic tiles

use std::borrow::{Borrow, BorrowMut};

use super::generic_routing::{GenericRoutingRefMutTrait, GenericRoutingRefTrait, RMUX, RMUXRef};
use super::local_lines::{IMUX, IMUXRef};
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

struct LogicAsyncMux(u8);
impl FieldPositionCalculator for LogicAsyncMux {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 16, "LUT index out of range");
        TileRelativeBitPos {
            x: 32,
            y: self.0 as u32 * 4 + if self.0 >= 8 { 4 } else { 0 },
        }
    }
}
struct LogicClkMux(u8);
impl FieldPositionCalculator for LogicClkMux {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 16, "LUT index out of range");
        TileRelativeBitPos {
            x: 32,
            y: 1 + self.0 as u32 * 4 + if self.0 >= 8 { 4 } else { 0 },
        }
    }
}
struct LogicShiftMode(u8);
impl FieldPositionCalculator for LogicShiftMode {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 16, "LUT index out of range");
        TileRelativeBitPos {
            x: 32,
            y: 2 + self.0 as u32 * 4 + if self.0 >= 8 { 4 } else { 0 },
        }
    }
}
struct LogicBypassMode(u8);
impl FieldPositionCalculator for LogicBypassMode {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 16, "LUT index out of range");
        TileRelativeBitPos {
            x: 32,
            y: 3 + self.0 as u32 * 4 + if self.0 >= 8 { 4 } else { 0 },
        }
    }
}

impl<D: DebugTracer, Ref: Borrow<Bitstream<D>>> LogicTileRef<D, Ref> {
    pub fn lut(&self, lc_idx: u8) -> u16 {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: LogicLUT(lc_idx),
            _d: PhantomData,
        };
        ref_.get_bits::<16>() as u16
    }

    pub fn lut_input(&self, lc_idx: u8, inp_idx: u8) -> IMUX {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: IMUXRef {
                is_bram: false,
                i: lc_idx * 4 + inp_idx,
            },
            _d: PhantomData,
        };
        IMUX::from_bits(ref_.get_bits::<12>())
    }

    pub fn lc_async_choice(&self, lc_idx: u8) -> Mux2 {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: LogicAsyncMux(lc_idx),
            _d: PhantomData,
        };
        Mux2::from(ref_.get_bit(0))
    }
    pub fn lc_clk_choice(&self, lc_idx: u8) -> Mux2 {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: LogicClkMux(lc_idx),
            _d: PhantomData,
        };
        Mux2::from(ref_.get_bit(0))
    }
    pub fn lc_shift_reg_mode(&self, lc_idx: u8) -> bool {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: LogicShiftMode(lc_idx),
            _d: PhantomData,
        };
        ref_.get_bit(0)
    }
    pub fn lc_input_c_bypass_mode(&self, lc_idx: u8) -> bool {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: LogicBypassMode(lc_idx),
            _d: PhantomData,
        };
        ref_.get_bit(0)
    }
}
impl<D: DebugTracer, Ref: BorrowMut<Bitstream<D>>> LogicTileRef<D, Ref> {
    pub fn set_lut(&mut self, lc_idx: u8, val: u16) {
        let mut ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: LogicLUT(lc_idx),
            _d: PhantomData,
        };
        ref_.set_bits::<16>(val as u32)
    }

    pub fn set_lut_input(&mut self, lc_idx: u8, inp_idx: u8, val: IMUX) {
        let mut ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: IMUXRef {
                is_bram: false,
                i: lc_idx * 4 + inp_idx,
            },
            _d: PhantomData,
        };
        ref_.set_bits::<16>(val.to_bits());
    }

    pub fn set_lc_async_choice(&mut self, lc_idx: u8, val: Mux2) {
        let mut ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: LogicAsyncMux(lc_idx),
            _d: PhantomData,
        };
        ref_.set_bit(0, val.into());
    }
    pub fn set_lc_clk_choice(&mut self, lc_idx: u8, val: Mux2) {
        let mut ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: LogicClkMux(lc_idx),
            _d: PhantomData,
        };
        ref_.set_bit(0, val.into());
    }
    pub fn set_lc_shift_reg_mode(&mut self, lc_idx: u8, val: bool) {
        let mut ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: LogicShiftMode(lc_idx),
            _d: PhantomData,
        };
        ref_.set_bit(0, val);
    }
    pub fn set_lc_input_c_bypass_mode(&mut self, lc_idx: u8, val: bool) {
        let mut ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: LogicBypassMode(lc_idx),
            _d: PhantomData,
        };
        ref_.set_bit(0, val);
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
