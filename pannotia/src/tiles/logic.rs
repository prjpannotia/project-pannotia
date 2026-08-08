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
//! ## Control signals
//!
//! Control signals for the register (flip-flop) have much more restricted routing,
//! and many resources must be shared by all LEs within a tile. Specifically, a tile has:
//! - 2x clock+clock-enable pairs
//! - 2x async control wires
//! - 1 sync clear wires
//! - 1 sync load wires
//!
//! Every LE can only pick from amongst these, as follows:
//! - using one clock+enable pair, or the other
//! - using one of the async control wires
//!   (this cannot be disabled, so "not using async reset" consumes one of the tile's wires)
//! - whether or not to use sync control signals (both load _and_ clear will be used)
//!
//! For the clock and async control signals only, the source can come from a global net.
//! The appropriate global net must first be selected via a "global to local" mux.
//! Otherwise, for all of of the control signals, the source can come from general routing.
//! In that case, a signal must be selected with a [CtrlMux],
//! which is similar to an `IMUX` but with more choices.
//!
//! ## Fast paths
//!
//! There are two fast paths between logic elements, carry chains and a "shift register" path.
//! Both of these connect _from_ the logic element above. "Above" means the LE
//! with an index 1 _smaller_ within the tile (for LEs 1 to 15 inclusive),
//! or the signals from LE 15 in the tile with a y-coordinate 1 _higher_ (for LE 0).
//!
//! The tiles at the top edge of the array do not have fast path inputs,
//! and these are supposedly hardwired to 0.
//!
//! ## Logic elements
//!
//! Each logic element consists of a LUT4 and a register.
//!
//! In the "default" configuration, the register's input comes from the output of the LUT4:
//!
//! ```text
//!                 +------------------>
//!      +------+   |       +----+
//! A -->|      |   |       |    |
//! B -->| LUT4 |---+-------| FF |----->
//! C -->|      |           |    |
//! D -->|      |      clk -|>   |
//!      +------+           +----+
//! ```
//!
//! However, input C to the LUT is special, because it can be used for carry chains:
//!
//! ```text
//!                           +------------------>
//!                +------+   |       +----+
//!           A -->|      |   |       |    |
//! Cin-->|\  B -->| LUT  |---+-------| FF |----->
//!       | |------|      |           |    |
//! C---->|/  D -->|      |-+    clk -|>   |
//!                +------+ |         +----+
//!                         v
//!                        Cout
//! ```
//!
//! The carry _out_ from a LUT is computed from inputs `A`, `B`, and `Cin` (not `C`),
//! using the "bottom" half of the LUT.
//! This computation is not affected by how LUT input C is configured.
//! The _data_ out from a LUT is the only thing affected by the input C mode.
//!
//! In practice, to implement an adder, the "bottom" half of the LUT implements a majority gate over
//! `A`, `B`, and `Cin`, and the "top" half of the LUT implements an XOR gate over the same inputs.
//! Input D is not used. However, other configurations _are_ permitted by the hardware.
//!
//! Logic elements also contain a "register feedback" path.
//! This takes the output of the register and replaces input C with it.
//!
//! ```text
//!                           +------------------>
//!                +------+   |       +----+
//!           A -->|      |   |       |    |
//! Cin-->|\  B -->| LUT  |---+-------| FF |--+-->
//! C---->| |------|      |           |    |  |
//!    +--|/  D -->|      |-+    clk -|>   |  |
//!    |           +------+ |         +----+  |
//!    |                    v                 |
//!    |                   Cout               |
//!    +--------------------------------------+
//! ```
//!
//! This fast path can be used to optimize state machines and similar logic.
//!
//! The register can also take its input from the register above, to implement shift registers:
//!
//! ```text
//!                         shift in
//!                             v
//!                           +-|---------------->
//!                +------+   | +-|\  +----+
//!           A -->|      |   |   | |-|    |
//! Cin-->|\  B -->| LUT  |---+---|/  | FF |--+-->
//! C---->| |------|      |           |    |  |
//!    +--|/  D -->|      |-+    clk -|>   |  |
//!    |           +------+ |         +----+  |
//!    |                    v                 |
//!    |                   Cout               |
//!    +--------------------------------------+
//!                                           v
//!                                     shift out
//! ```
//!
//! When the shift register feature is being used, the LUT output remains available via certain OMUX paths.
//! The shift register feature can also be combined with the "register feedback" feature.
//!
//! The register can be optionally forced to take its input from input C when a "sync load" signal is asserted:
//!
//! ```text
//!                             shift in
//!                                 v
//!                               +-|---------------------------------->
//!                    +------+   | +-|\
//!               A -->|      |   |   | |-|\
//!     Cin-->|\  B -->| LUT  |---+---|/  | |--|\        +----+
//! C-+------>| |------|      |         +-|/   | |-------| FF |--+----->
//!   |    +--|/  D -->|      |-+       |  | 0-|/        |    |  |
//!   |    |           +------+ |       |  |    |   clk -|>   |  |
//!   |    |                    v       |  |    |        +----+  |
//!   |    |                   Cout     |  |    |                |
//!   |    +----------------------------|--|----|----------------+
//!   +---------------------------------+  |    |                |
//!                             sync load -+    |                v
//!                             sync clear -----+           shift out
//! ```
//!
//! If sync load is a constant 1, this allows the LUT and register to implement completely unrelated functionality
//! (at the cost of losing one useful LUT input and tile-wide packing constraints over the sync control signals).
//!
//! Finally, the register contains an async reset:
//!
//! ```text
//!                             shift in
//!                                 v
//!                               +-|---------------------------------->
//!                    +------+   | +-|\              async reset
//!               A -->|      |   |   | |-|\               |
//!     Cin-->|\  B -->| LUT  |---+---|/  | |--|\        +-V--+
//! C-+------>| |------|      |         +-|/   | |-------|    |--+----->
//!   |    +--|/  D -->|      |-+       |  | 0-|/        | FF |  |
//!   |    |           +------+ |       |  |    |   clk -|>   |  |
//!   |    |                    v       |  |    |        +----+  |
//!   |    |                   Cout     |  |    |                |
//!   |    +----------------------------|--|----|----------------+
//!   +---------------------------------+  |    |                |
//!                             sync load -+    |                v
//!                             sync clear -----+           shift out
//! ```
//!
//! Features that are _not_ present include:
//! - no async set
//! - no default power-up state
//! - cannot be used as distributed RAM

use std::borrow::Borrow;

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

/// The choice of inputs for LUT input C
///
/// This LUT input is used to implement the special functions of:
/// - carry chains
/// - "register feedback" mode
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[bitmux::bitenum]
pub enum InputCMode {
    /// Input C comes from the "normal" path, selected from the local lines via an [IMUX]
    IMUX = "00",
    /// Input C comes from the output of the register (flip-flop)
    FlipFlop = "01",
    /// Input C comes from the carry-out of the LUT above
    // Both 10 and 11 seem to work, which matches vendor simulation model
    Carry = "1x",
}
impl Default for InputCMode {
    fn default() -> Self {
        Self::IMUX
    }
}
impl Display for InputCMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InputCMode::IMUX => write!(f, "imux"),
            InputCMode::FlipFlop => write!(f, "ff"),
            InputCMode::Carry => write!(f, "carry"),
        }
    }
}
impl FromStr for InputCMode {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("imux") {
            Ok(Self::IMUX)
        } else if s.eq_ignore_ascii_case("ff") {
            Ok(Self::FlipFlop)
        } else if s.eq_ignore_ascii_case("carry") {
            Ok(Self::Carry)
        } else {
            Err(())
        }
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
impl FromStr for OMUX {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("comb") {
            Ok(Self::Combinatorial)
        } else if s.eq_ignore_ascii_case("ff") {
            Ok(Self::FlipFlop)
        } else {
            Err(())
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

/// A LUT value, a `u16` with a modified [Default]
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct LUT(pub u16);
impl Default for LUT {
    fn default() -> Self {
        Self(0xffff)
    }
}
impl ::bitmux::BitstreamField for LUT {
    // NOTE: The LUT bits are _inverted_ in the bitstream
    fn get(b: impl BitGetter) -> Self {
        Self((b.get_bits::<16>() as u16) ^ 0xffff)
    }
    fn set(&self, mut b: impl BitSetter) {
        b.set_bits::<16>((self.0 ^ 0xffff) as u32)
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

        /// The logic function implemented in this logic cell
        ///
        /// The bit at index `(d << 3) + (c << 2) + (b << 1) + (a << 0)`
        /// (with the least significant bit being bit 0)
        /// is chosen by a given set of inputs `abcd`.
        pub fn lut(&self, lc_idx: u8) -> LUT {
            LogicLUT(lc_idx)
        }
        /// This routes signals from RMUX local lines into each LUT
        pub fn lut_input(&self, lc_idx: u8, inp_idx: u8) -> IMUX {
            IMUXRef {
                is_bram: false,
                i: lc_idx * 4 + inp_idx,
            }
        }
        pub fn lut_inp_a(&self, lc_idx: u8) -> IMUX = {
            self.lut_input(lc_idx, 0)
        }
        pub fn lut_inp_b(&self, lc_idx: u8) -> IMUX = {
            self.lut_input(lc_idx, 1)
        }
        pub fn lut_inp_c(&self, lc_idx: u8) -> IMUX = {
            self.lut_input(lc_idx, 2)
        }
        pub fn lut_inp_d(&self, lc_idx: u8) -> IMUX = {
            self.lut_input(lc_idx, 3)
        }

        /// This controls whether the LUT combinatorial output or the flip-flop output is used.
        ///
        /// Because each logic cell has multiple OMUX, both can be used at the same time.
        pub fn lc_output(&self, lc_idx: u8, out_idx: u8) -> OMUX {
            LogicOut {
                lc: lc_idx,
                i: out_idx,
            }
        }
        pub fn lc_output_neigh(&self, lc_idx: u8) -> OMUX = {
            self.lc_output(lc_idx, 0)
        }
        pub fn lc_output_imux(&self, lc_idx: u8) -> OMUX = {
            self.lc_output(lc_idx, 1)
        }
        pub fn lc_output_rmux(&self, lc_idx: u8) -> OMUX = {
            self.lc_output(lc_idx, 2)
        }

        /// The special mux on input C of the LUT.
        pub fn lc_input_c_mode(&self, lc_idx: u8) -> InputCMode {
            LogicInputC(lc_idx)
        }
        /// Whether or not the carry output of this LUT can be used
        ///
        /// This defaults to _enabled_. When disabled, the carry output value is always 1.
        pub fn lc_carry_en(&self, lc_idx: u8) -> bitmux::InvertedBool {
            LogicCarryEn(lc_idx)
        }

        pub fn lc_async_choice(&self, lc_idx: u8) -> Mux2 {
            LogicAsyncMux(lc_idx)
        }
        pub fn lc_clk_choice(&self, lc_idx: u8) -> Mux2 {
            LogicClkMux(lc_idx)
        }
        /// Controls whether the register uses the output from the register above, instead of the LUT
        ///
        /// This is used to implement fast shift registers.
        pub fn lc_shift_reg_mode(&self, lc_idx: u8) -> bool {
            LogicShiftMode(lc_idx)
        }
        /// Controls whether the sync load/clear function is enabled for this flip-flop
        ///
        /// If it is enabled, and the _tile-wide_ sync load signal is asserted,
        /// the data to be loaded comes from the same signal that goes into input C of the LUT.
        ///
        /// Synchronous clear has priority over synchronous load.
        pub fn lc_enable_sync_ctrl(&self, lc_idx: u8) -> bool {
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
