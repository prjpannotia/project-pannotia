//! Logic tiles

use std::borrow::{Borrow, BorrowMut};

use super::generic_routing::{GenericRoutingRefMutTrait, GenericRoutingRefTrait, RMUX, RMUXRef};
use super::local_lines::{CtrlMux, CtrlMuxRef, IMUX, IMUXRef};
use super::*;

use bitmux::{BitGetter, BitSetter, BitstreamField};

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

struct TileClk(u8);
impl FieldPositionCalculator for TileClk {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 2, "clock index out of range");
        let x = 30 - biti as u32;
        let y = [32, 35][self.0 as usize];
        TileRelativeBitPos { y, x }
    }
}
struct TileClkEn(u8);
impl FieldPositionCalculator for TileClkEn {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 2, "clock enable index out of range");
        let x = 33 - biti as u32;
        let y = [32, 35][self.0 as usize];
        TileRelativeBitPos { y, x }
    }
}
struct TileAsync(u8);
impl FieldPositionCalculator for TileAsync {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 2, "async index out of range");
        let x = 30 - biti as u32;
        let y = [33, 34][self.0 as usize];
        TileRelativeBitPos { y, x }
    }
}
struct TileSLoad {}
impl FieldPositionCalculator for TileSLoad {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        let x = 33 - biti as u32;
        let y = 33;
        TileRelativeBitPos { y, x }
    }
}
struct TileSClr {}
impl FieldPositionCalculator for TileSClr {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        let x = 33 - biti as u32;
        let y = 34;
        TileRelativeBitPos { y, x }
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

// FIXME: This needs to be RE'd, cannot figure out how to get vendor tools to generate it
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[bitmux::bitenum]
pub enum InputCMode {
    _00 = "00",
    _01 = "01",
    _10 = "10",
    _11 = "11",
}
impl Default for InputCMode {
    fn default() -> Self {
        Self::_00
    }
}

struct LogicInputC(u8);
impl FieldPositionCalculator for LogicInputC {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 16, "LUT index out of range");
        TileRelativeBitPos {
            x: 31,
            y: [3, 0][biti] + self.0 as u32 * 4 + if self.0 >= 8 { 4 } else { 0 },
        }
    }
}
struct LogicCarryEn(u8);
impl FieldPositionCalculator for LogicCarryEn {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 16, "LUT index out of range");
        TileRelativeBitPos {
            x: 31,
            y: 2 + self.0 as u32 * 4 + if self.0 >= 8 { 4 } else { 0 },
        }
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

/// A choice between the LUT's output and the flip-flop's output
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum OMUX {
    LUT,
    FlipFlop,
}
impl Display for OMUX {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LUT => write!(f, "LUT"),
            Self::FlipFlop => write!(f, "FF"),
        }
    }
}
impl Default for OMUX {
    fn default() -> Self {
        Self::LUT
    }
}
impl From<bool> for OMUX {
    fn from(value: bool) -> Self {
        match value {
            false => Self::LUT,
            true => Self::FlipFlop,
        }
    }
}
impl From<OMUX> for bool {
    fn from(value: OMUX) -> Self {
        match value {
            OMUX::LUT => false,
            OMUX::FlipFlop => true,
        }
    }
}

struct LogicOut {
    lc: u8,
    i: u8,
}
impl FieldPositionCalculator for LogicOut {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.lc < 16, "LUT index out of range");
        assert!(self.i < 3, "output index out of range");
        TileRelativeBitPos {
            x: 33,
            y: [0, 2, 3][self.i as usize] + self.lc as u32 * 4 + if self.lc >= 8 { 4 } else { 0 },
        }
    }
}

impl<D: DebugTracer, Ref: Borrow<Bitstream<D>>> LogicTileRef<D, Ref> {
    pub fn clock_mux(&self, clk_idx: u8) -> Mux3Inv {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: TileClk(clk_idx),
            _d: PhantomData,
        };
        Mux3Inv::get(ref_)
    }
    pub fn clock_en_mux(&self, clk_idx: u8) -> Mux2Inv {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: TileClkEn(clk_idx),
            _d: PhantomData,
        };
        Mux2Inv::get(ref_)
    }
    pub fn async_mux(&self, clk_idx: u8) -> Mux3Inv {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: TileAsync(clk_idx),
            _d: PhantomData,
        };
        Mux3Inv::get(ref_)
    }
    pub fn sync_load_mux(&self) -> Mux2Inv {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: TileSLoad {},
            _d: PhantomData,
        };
        Mux2Inv::get(ref_)
    }
    pub fn sync_clr_mux(&self) -> Mux2Inv {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: TileSClr {},
            _d: PhantomData,
        };
        Mux2Inv::get(ref_)
    }

    pub fn global_to_local(&self, inp_idx: u8) -> GlobalToLocalMux {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: GlobalToLocalMuxRef {
                is_bram: false,
                i: inp_idx,
            },
            _d: PhantomData,
        };
        GlobalToLocalMux::get(ref_)
    }

    pub fn control_signal_preselect(&self, inp_idx: u8) -> CtrlMux {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: CtrlMuxRef {
                is_bram: false,
                i: inp_idx,
            },
            _d: PhantomData,
        };
        CtrlMux::get(ref_)
    }

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
        IMUX::get(ref_)
    }

    pub fn lc_output(&self, lc_idx: u8, out_idx: u8) -> OMUX {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: LogicOut {
                lc: lc_idx,
                i: out_idx,
            },
            _d: PhantomData,
        };
        ref_.get_bit(0).into()
    }

    pub fn lc_input_c_mode(&self, lc_idx: u8) -> InputCMode {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: LogicInputC(lc_idx),
            _d: PhantomData,
        };
        InputCMode::get(ref_)
    }
    pub fn lc_carry_en(&self, lc_idx: u8) -> bool {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: LogicCarryEn(lc_idx),
            _d: PhantomData,
        };
        !ref_.get_bit(0)
    }

    pub fn lc_async_choice(&self, lc_idx: u8) -> Mux2 {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: LogicAsyncMux(lc_idx),
            _d: PhantomData,
        };
        ref_.get_bit(0).into()
    }
    pub fn lc_clk_choice(&self, lc_idx: u8) -> Mux2 {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: LogicClkMux(lc_idx),
            _d: PhantomData,
        };
        ref_.get_bit(0).into()
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
    pub fn set_clock_mux(&mut self, clk_idx: u8, val: Mux3Inv) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: TileClk(clk_idx),
            _d: PhantomData,
        };
        val.set(ref_);
    }
    pub fn set_clock_en_mux(&mut self, clk_idx: u8, val: Mux2Inv) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: TileClkEn(clk_idx),
            _d: PhantomData,
        };
        val.set(ref_);
    }
    pub fn set_async_mux(&mut self, clk_idx: u8, val: Mux3Inv) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: TileAsync(clk_idx),
            _d: PhantomData,
        };
        val.set(ref_);
    }
    pub fn set_sync_load_mux(&mut self, val: Mux2Inv) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: TileSLoad {},
            _d: PhantomData,
        };
        val.set(ref_);
    }
    pub fn set_sync_clr_mux(&mut self, val: Mux2Inv) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: TileSClr {},
            _d: PhantomData,
        };
        val.set(ref_);
    }

    pub fn set_global_to_local(&mut self, inp_idx: u8, val: GlobalToLocalMux) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: GlobalToLocalMuxRef {
                is_bram: false,
                i: inp_idx,
            },
            _d: PhantomData,
        };
        val.set(ref_);
    }

    pub fn set_control_signal_preselect(&mut self, inp_idx: u8, val: CtrlMux) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: CtrlMuxRef {
                is_bram: false,
                i: inp_idx,
            },
            _d: PhantomData,
        };
        val.set(ref_);
    }

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
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: IMUXRef {
                is_bram: false,
                i: lc_idx * 4 + inp_idx,
            },
            _d: PhantomData,
        };
        val.set(ref_);
    }

    pub fn set_lc_output(&mut self, lc_idx: u8, out_idx: u8, val: OMUX) {
        let mut ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: LogicOut {
                lc: lc_idx,
                i: out_idx,
            },
            _d: PhantomData,
        };
        ref_.set_bit(0, val.into());
    }

    pub fn set_lc_input_c_mode(&mut self, lc_idx: u8, val: InputCMode) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: LogicInputC(lc_idx),
            _d: PhantomData,
        };
        val.set(ref_)
    }
    pub fn set_lc_carry_en(&mut self, lc_idx: u8, val: bool) {
        let mut ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: LogicCarryEn(lc_idx),
            _d: PhantomData,
        };
        ref_.set_bit(0, !val);
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
        RMUX::get(ref_)
    }
}
impl<D: DebugTracer, Ref: BorrowMut<Bitstream<D>>> GenericRoutingRefMutTrait
    for LogicTileRef<D, Ref>
{
    fn set_rmux(&mut self, rmux_idx: u8, val: RMUX) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: RMUXRef {
                is_bram: false,
                i: rmux_idx,
            },
            _d: PhantomData,
        };
        val.set(ref_);
    }
}
