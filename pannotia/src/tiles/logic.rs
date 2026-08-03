//! Logic tiles
//!
//! A logic tile contains 16 individual logic elements (LEs), which
//! each consist of a LUT4 and a register (flip-flop).
//! The tile also contains input selection muxes for each LE
//! and a number of control signal muxes shared by all LEs.
//!
//! ## Routing
//!
//! Each LE's input has an [IMUX] to select a signal.
//! Inputs `A` and `C` have a similar signal mix available,
//! as do inputs `B` and `D`. The input mix is also very similar
//! between each LE in a tile. In general, this allows for
//! input swapping and LE swapping. However, the sites are not _exactly_
//! identical, so care must be taken.
//!
//! `IMUX` can select from amongst the following choices:
//! - `RMUX` self-wires (from where it likely came from the interconnect)
//! - neighbor wires from the tile on the right
//!   (typically an output from another `RMUX` and thus also from the interconnect)
//! - LE outputs from within this tile (but via an `OMUX`, so only
//!   _either_ the LUT or the flip-flop's output is available, but not both _via this path_)
//!
//! Both the combinatorial output and the registered output of
//! each LE can be used simultaneously, and each LE has _three_
//! [OMUX]es independently selecting from the two. These muxes then drive:
//! 1. right-going neighbor wires
//!    (where the signal typically enters an `RMUX` to drive it further)
//! 2. LUT inputs within this tile (via `IMUX`)
//! 3. `RMUX` within this tile
//!
//! Visually, the routing _within_ a logic tile looks like this:
//!
//! ```text
//!                                     +------------+
//! output wires other than T1_E <------|            |--+
//! general-purpose routing wires ----->| 6× 16 RMUX |  | RMUX-to-RMUX self-wires
//!                                     |            |<-+
//!                                     | 4× CtrlMUX |----------------------+
//!                                     |            |<-------------+       | 2× 16 local lines (RMUX-to-IMUX)
//!                                     +------------+              |       v
//!                                         ^ |                     |   +------------+
//!                 +--------------------+  | | 4× control signal   |   | 4× 16 IMUX |<-- T1_W wires
//! global wires -> | 4× global-to-local | -+ |    preselections    |   +------------+
//!                 +--------------------+  | |                     |       | ^
//!                                         v v            16× OMUX |       | | 16× OMUX
//!                                        _____                    |       v |
//!                                        \___/                    |   +---------+
//!                                          |                      +---|         |
//!                tile-wide control signals |                          | 16× LEs |----> T1_E wires (via OMUX)
//!        (clock+enable, async reset, etc.) +------------------------->|         |
//!                                                                     +---------+
//! ```
//!
//! TODO: This following bit of the documentation should be improved
//!
//! A logic tile also has a number of "control signal" wires which are shared by all LEs.
//! The input to these wires can either come from a global net or from the general routing.
//! If it comes from a global net, a global net must first be selected via a "global to local" mux.
//! If it comes from general routing, a signal must be selected with a [CtrlMux],
//! which is similar to an `IMUX` but with more choices.
//!
//! ## Logic elements
//!
//! TODO: This documentation should be stolen from Altera and fixed accordingly

use std::borrow::{Borrow, BorrowMut};

use super::generic_routing::{GenericRoutingRefMutTrait, GenericRoutingRefTrait, RMUX, RMUXRef};
use super::local_lines::{CtrlMux, CtrlMuxRef, IMUX, IMUXRef};
use super::*;

use bitmux::{BitGetter, BitSetter};

make_tile_ref! {
    /// Access to a logic tile
    LogicTileRef = TileType::Logic
}

/// (Helper) access to clock mux
#[derive(Debug)]
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
/// (Helper) access to clock enable mux
#[derive(Debug)]
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
/// (Helper) access to async control signal mux
#[derive(Debug)]
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
/// (Helper) access to sync load mux
#[derive(Debug)]
struct TileSLoad {}
impl FieldPositionCalculator for TileSLoad {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        let x = 33 - biti as u32;
        let y = 33;
        TileRelativeBitPos { y, x }
    }
}
/// (Helper) access to sync clear mux
#[derive(Debug)]
struct TileSClr {}
impl FieldPositionCalculator for TileSClr {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        let x = 33 - biti as u32;
        let y = 34;
        TileRelativeBitPos { y, x }
    }
}

/// (Helper) access to LUT bits
#[derive(Debug)]
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

/// (Helper) access to LE input-C setting
#[derive(Debug)]
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
/// (Helper) access to LE carry setting
#[derive(Debug)]
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

/// (Helper) access to LE async control mux
#[derive(Debug)]
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
/// (Helper) access to LE clock+enable mux
#[derive(Debug)]
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
/// (Helper) access to LE shift-register setting
#[derive(Debug)]
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
/// (Helper) access to LE LUT-bypass setting
#[derive(Debug)]
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

/// A choice between an unregistered output and the flip-flop's output
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[bitmux::bitenum]
pub enum OMUX {
    Combinatorial = "0",
    FlipFlop = "1",
}
impl Display for OMUX {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Combinatorial => write!(f, "comb"),
            Self::FlipFlop => write!(f, "ff"),
        }
    }
}
impl Default for OMUX {
    fn default() -> Self {
        Self::Combinatorial
    }
}

/// (Helper) access to LE OMUX setting
#[derive(Debug)]
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
    pub fn lut(&self, lc_idx: u8) -> u16 {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: LogicLUT(lc_idx),
            _d: PhantomData,
        };
        ref_.get_bits::<16>() as u16
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
}

magic_tile_impl_gen! {
    impl LogicTileRef {
        /// Selects which input (a global line or a preselected control signal) drives this clock line
        pub fn clock_mux(&self, clk_idx: u8) -> Mux3Inv {
            TileClk(clk_idx)
        }
        pub fn clock_en_mux(&self, clk_idx: u8) -> Mux2Inv {
            TileClkEn(clk_idx)
        }
        pub fn async_mux(&self, asy_idx: u8) -> Mux3Inv {
            TileAsync(asy_idx)
        }
        pub fn sync_load_mux(&self) -> Mux2Inv {
            TileSLoad {}
        }
        pub fn sync_clr_mux(&self) -> Mux2Inv {
            TileSClr {}
        }

        pub fn global_to_local(&self, inp_idx: u8) -> GlobalToLocalMux {
            GlobalToLocalMuxRef {
                is_bram: false,
                i: inp_idx,
            }
        }

        pub fn control_signal_preselect(&self, inp_idx: u8) -> CtrlMux {
            CtrlMuxRef {
                is_bram: false,
                i: inp_idx,
            }
        }

        pub fn lut_input(&self, lc_idx: u8, inp_idx: u8) -> IMUX {
            IMUXRef {
                is_bram: false,
                i: lc_idx * 4 + inp_idx,
            }
        }

        pub fn lc_output(&self, lc_idx: u8, out_idx: u8) -> OMUX {
            LogicOut {
                lc: lc_idx,
                i: out_idx,
            }
        }

        pub fn lc_input_c_mode(&self, lc_idx: u8) -> InputCMode {
            LogicInputC(lc_idx)
        }
        pub fn lc_carry_en(&self, lc_idx: u8) -> bitmux::InvertedBool {
            LogicCarryEn(lc_idx)
        }

        pub fn lc_async_choice(&self, lc_idx: u8) -> Mux2 {
            LogicAsyncMux(lc_idx)
        }
        pub fn lc_clk_choice(&self, lc_idx: u8) -> Mux2 {
            LogicClkMux(lc_idx)
        }
        pub fn lc_shift_reg_mode(&self, lc_idx: u8) -> bool {
            LogicShiftMode(lc_idx)
        }
        pub fn lc_input_c_bypass_mode(&self, lc_idx: u8) -> bool {
            LogicBypassMode(lc_idx)
        }
    }
}

magic_tile_impl_gen! {
    impl on LogicTileRef trait GenericRoutingRefTrait, GenericRoutingRefMutTrait {
        fn rmux(&self, rmux_idx: u8) -> RMUX {
            RMUXRef {
                is_bram: false,
                i: rmux_idx,
            }
        }
    }
}
