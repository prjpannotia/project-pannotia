//! Block RAM tiles (9216 bits)
//!
//! A block RAM tile is also similar to a logic tile, with the obvious difference
//! that a block RAM requires many more signals than a single logic cell.
//!
//! ## Outputs
//!
//! Because a block RAM has 36 output wires which is approximately equal to 2×16,
//! there are no output muxes in a BRAM tile. Instead, all output wiring is fixed.
//!
//! The right-going neighbor wires (`T1_E`) get bits [0-7] and [9-16] of port A's
//! data output. This corresponds to 16 "non-parity" bits.
//!
//! The inputs to `RMUX` which in a logic tile _would've_ come from LE outputs
//! instead get bits [0-7] and [9-16] of port B's data output.
//!
//! The remaining 4 output bits (2 from each of port A/B) replace
//! what in a logic tile are the 4 global-to-local wire inputs into the `RMUX`.
//! A BRAM tile still _has_ global-to-local wires though (6 rather than 4),
//! so those then replace what in a logic tile would be RMUX-to-RMUX self-wires.
//! A BRAM tile doesn't have those.
//!
//! ## Inputs
//!
//! Address and data inputs can use a similar set of `IMUX` as LE inputs.
//! Each port has 18 data lines and 13 address lines for a total of 31 inputs per port.
//! This equals a grand total of 62 inputs per BRAM which is 2 less than the 64 inputs
//! in a logic tile. These 2 extra wires can be used as clock enables or async resets.
//!
//! Because feeding a block RAM's data output right back into its input is
//! not nearly as useful as feeding a LE's output back into its input,
//! and because BRAMs need a much larger set of _unique_ inputs,
//! BRAM `IMUX`es contain a different mix of input signals from a logic tile
//! (and do not contain such loopback paths, instead replacing them with extra `RMUX` choices).
//! BRAM `CtrlMUX`es likewise also do not contain these wires,
//! instead replacing them with additional neighbor wires from the left neighbor.
//!
//! However, `IMUX`es do not cover the BRAM's actual control signals (e.g. read/write enables)
//! and also does not give the ability to invert these control signals where desired.
//! BRAM tiles thus contain an additional layer of [TMUX] followed by [KMUX] for these signals.
//! A [TMUX] preselects from (mostly) the various `T*` wires found in the tile.
//! A [KMUX] then selects from amongst the `TMUX` outputs, optionally including an invert bit.
//!
//! ## Diagram
//!
//! ```text
//!                                     +------------+                                  +----------+
//! output wires other than T1_E <------|            |--- TMUX-to-KMUX wires ---------->| 16× KMUX |--> 6× useless wires
//! general-purpose routing wires ----->| 16× TMUX   |<-+                               +----------+
//!                                     | 6× 16 RMUX |--+ RMUX-to- *TMUX* wires                 |
//!                                     | 4× CtrlMUX |--------------------------+               +------ 10× control signals ----+
//!                                     |            |<-----------------+       | 3× 16 local lines (RMUX-to-IMUX)              |
//!                                     +------------+                  |       v                                               |
//!                                         ^ |                         |   +------------+                                      |
//!                 +--------------------+  | | 4× control signal       |   | 4× 16 IMUX |<-- T1_W wires                        |
//! global wires -> | 6× global-to-local | -+ |    preselections        |   +------------+                                      |
//!                 +--------------------+  | | +---------------------- | ----+     |   2× (18× data wires, 13× address wires)  |
//!                                         v v v    16× wires (port B) | 2× bonus  | =62× total wires                          |
//!                                        _______   +4× wires (parity) |    wires  v                                           |
//!                                        \_____/                      |   +---------+                                         |
//!                                          |                          +---|         |<----------------------------------------+
//!                tile-wide control signals |                              |   RAM   |----> T1_E wires (port A)
//!              (clock+enable, async reset) +----------------------------->|         |
//!                                                                         +---------+
//! ```

use std::borrow::{Borrow, BorrowMut};

use super::generic_routing::{GenericRoutingRefMutTrait, GenericRoutingRefTrait, RMUX, RMUXRef};
use super::local_lines::{CtrlMux, CtrlMuxRef, IMUX, IMUXRef};

use super::*;

use bitmux::{BitGetter, BitSetter};

make_tile_ref! {
    /// Access to a BRAM tile
    BRAMTileRef = TileType::RoutingOnly
}

/// (Helper) access to BRAM preloaded data
#[derive(Debug)]
struct InitVal {}
impl FieldPositionCalculator for InitVal {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        // each row contains 144 bits, and there are 64 rows
        let mut y = biti as u32 / 144;
        // the bottom half has to jump over the mid-tile gap
        if y >= 32 {
            y += 4;
        }

        // 144 = 18 * 8, and the bits within a row are interleaved
        // 8 instances of "every 18th bit" comes first, starting from bit 0
        // (i.e. 0, 18, 36, ...)
        // this repeats 18 times to fill out all the bits
        let bitpos_col_before_rearrange = biti as u32 % 144;
        let group_of_8 = bitpos_col_before_rearrange % 18;
        let idx_within_8 = bitpos_col_before_rearrange / 18;
        let x = 36 + group_of_8 * 8 + idx_within_8;

        TileRelativeBitPos { y, x }
    }
}

/// (Helper) access to TMUX
#[derive(Debug)]
struct TMUXRef(u8);
impl FieldPositionCalculator for TMUXRef {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 16, "TMUX index out of range");

        // the TMUX vertical position pattern repeats in groups of 4
        let group_of_4 = self.0 / 4;
        let within_group_4 = self.0 % 4;
        let y_base =
            [0, 16, 32 + 4, 48 + 4][group_of_4 as usize] + [2, 6, 8, 12][within_group_4 as usize];

        // Now we can look up the main 4x2 block
        let (x, y) = const {
            bitmux::bittable!(
                (28u8 + #x, #y as u8),
                0   2   4   5,
                1   3   7   6,
            )
        }[biti];
        TileRelativeBitPos {
            x: x as u32,
            y: (y_base + y) as u32,
        }
    }
}

/// A routing mux to preselect control signals for a BRAM
///
/// This mux can either be unprogrammed or have 15 choices.
/// The exact set of inputs depends on the specific mux.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum TMUX {
    None,
    I(u8),
}
impl Default for TMUX {
    fn default() -> Self {
        Self::None
    }
}
impl Display for TMUX {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "<unset>"),
            Self::I(i) => write!(f, "#{i}"),
        }
    }
}
impl FromStr for TMUX {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_noinv_helper(s, 15).map(|x| match x {
            Some(i) => Self::I(i),
            None => Self::None,
        })
    }
}
impl bitmux::BitstreamField for TMUX {
    fn get(b: impl bitmux::BitGetter) -> Self {
        let bits = b.get_bits::<8>();
        bitmux::twohot!(3, 5, match bits {
            #bits => Self::I(#val),
            0 => Self::None,
            _ => panic!("invalid TMUX {bits:08b}"),
        })
    }
    fn set(&self, mut b: impl bitmux::BitSetter) {
        let bits = bitmux::twohot!(3, 5, match self {
            Self::I(#val) => #bits,
            Self::None => 0,
            _ => panic!("invalid TMUX {}", self),
        });
        b.set_bits::<8>(bits);
    }
}

/// (Helper) access to KMUX
#[derive(Debug)]
struct KMUXRef(u8);
impl FieldPositionCalculator for KMUXRef {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 16, "KMUX index out of range");

        // the KMUX vertical position pattern repeats in groups of 4,
        // slotting "in between" the TMUX.
        // except, for some reason, number 0 is at the very bottom!
        let remapped_idx = match self.0 {
            0 => 15,
            _ => self.0 - 1,
        };

        let group_of_4 = remapped_idx / 4;
        let within_group_4 = remapped_idx % 4;
        let y_base =
            [0, 16, 32 + 4, 48 + 4][group_of_4 as usize] + [0, 4, 10, 14][within_group_4 as usize];

        // Now we can look up the main 4x2 block ( + 1 extra invert bit)
        let (x, y) = const {
            bitmux::bittable!(
                (28u8 + #x, #y as u8),
                .   .   .   .   5   4   2   0,
                .   8   .   .   6   7   3   1,
            )
        }[biti];
        TileRelativeBitPos {
            x: x as u32,
            y: (y_base + y) as u32,
        }
    }
}

/// A routing mux to select control signals for a BRAM
///
/// This mux can either be unprogrammed or have 15 choices.
/// The exact set of inputs depends on the specific mux.
///
/// This also supports programmable inversion
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum KMUX {
    VCC,
    GND,
    I { invert: bool, i: u8 },
}
impl Default for KMUX {
    fn default() -> Self {
        Self::VCC
    }
}
impl Display for KMUX {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VCC => write!(f, "VCC"),
            Self::GND => write!(f, "GND"),
            Self::I { invert, i } => {
                if *invert {
                    write!(f, "!")?;
                }
                write!(f, "#{i}")
            }
        }
    }
}
impl FromStr for KMUX {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("vcc") {
            Ok(Self::VCC)
        } else if s.eq_ignore_ascii_case("gnd") {
            Ok(Self::GND)
        } else {
            parse_inv_helper(s, 15).map(|(invert, i)| Self::I { invert, i })
        }
    }
}
impl bitmux::BitstreamField for KMUX {
    fn get(b: impl bitmux::BitGetter) -> Self {
        let bits = b.get_bits::<9>();
        let invert = bits & 0b1_0000_0000 != 0;
        bitmux::twohot!(3, 5, match bits & 0b1111_1111 {
            #bits => Self::I { invert, i: #val },
            0 if invert => Self::GND,
            0 if !invert => Self::VCC,
            _ => panic!("invalid KMUX {bits:09b}"),
        })
    }
    fn set(&self, mut b: impl bitmux::BitSetter) {
        let mut bits = bitmux::twohot!(3, 5, match self {
            Self::I { i: #val, .. } => #bits,
            Self::GND => 0b1_0000_0000,
            Self::VCC => 0,
            _ => panic!("invalid KMUX {}", self),
        });
        if let Self::I { invert: true, .. } = self {
            bits |= 0b1_0000_0000;
        }
        b.set_bits::<9>(bits);
    }
}

/// (Helper) access to clock mux
#[derive(Debug)]
struct TileClk(u8);
impl FieldPositionCalculator for TileClk {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        let y = match self.0 {
            0 => 32,
            1 => 35,
            _ => panic!("clock index out of range"),
        };
        bitmux::bittable!(
            TileRelativeBitPos { x: 32 + #x, y },
            2   1   0   3
        )[biti]
    }
}
/// (Helper) access to clock enable mux
#[derive(Debug)]
struct TileClkEn(u8);
impl FieldPositionCalculator for TileClkEn {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        match self.0 {
            0 => bitmux::bittable!(
                TileRelativeBitPos { x: 32 + #x, y: 34 },
                2   1   0   3
            )[biti],
            1 => bitmux::bittable!(
                TileRelativeBitPos { x: 30 + #x, y: 34 + #y },
                1   2,
                0   3,
            )[biti],
            _ => panic!("clock enable index out of range"),
        }
    }
}
/// (Helper) access to async control signal mux
#[derive(Debug)]
struct TileAsync(u8);
impl FieldPositionCalculator for TileAsync {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        match self.0 {
            0 => bitmux::bittable!(
                TileRelativeBitPos { x: 30 + #x, y: 32 + #y },
                0   3,
                1   2,
            )[biti],
            1 => bitmux::bittable!(
                TileRelativeBitPos { x: 32 + #x, y: 33 },
                2   1   0   3
            )[biti],
            _ => panic!("async index out of range"),
        }
    }
}

/// Block RAM port data width in bits
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[bitmux::bitenum]
pub enum PortWidth {
    X36 = "10000",
    X18 = "00000",
    X9 = "01000",
    X4 = "01100",
    X2 = "01110",
    X1 = "01111",
}
impl Default for PortWidth {
    fn default() -> Self {
        Self::X18
    }
}
impl Display for PortWidth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::X36 => write!(f, "36"),
            Self::X18 => write!(f, "18"),
            Self::X9 => write!(f, "9"),
            Self::X4 => write!(f, "4"),
            Self::X2 => write!(f, "2"),
            Self::X1 => write!(f, "1"),
        }
    }
}
impl FromStr for PortWidth {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "36" => Ok(Self::X36),
            "18" => Ok(Self::X18),
            "9" => Ok(Self::X9),
            "4" => Ok(Self::X4),
            "2" => Ok(Self::X2),
            "1" => Ok(Self::X1),
            _ => Err(()),
        }
    }
}
/// (Helper) access to BRAM port A width
#[derive(Debug)]
struct PortAWidth {}
impl FieldPositionCalculator for PortAWidth {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        bitmux::bittable!(
            TileRelativeBitPos { x: 32 + #x, y: #y },
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   3,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   2   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   1,
            .   .   .   .,
            .   .   .   0,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   4   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
        )[biti]
    }
}
/// (Helper) access to BRAM port B width
#[derive(Debug)]
struct PortBWidth {}
impl FieldPositionCalculator for PortBWidth {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        bitmux::bittable!(
            TileRelativeBitPos { x: 32 + #x, y: #y },
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   1   0,
            .   .   .   .,
            .   .   .   4,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   2   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   3   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
        )[biti]
    }
}

/// (Helper) access to BRAM port A output reg enable
#[derive(Debug)]
struct OutRegA {}
impl FieldPositionCalculator for OutRegA {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        TileRelativeBitPos { x: 34, y: 29 }
    }
}
/// (Helper) access to BRAM port B output reg enable
#[derive(Debug)]
struct OutRegB {}
impl FieldPositionCalculator for OutRegB {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        TileRelativeBitPos { x: 35, y: 39 }
    }
}
/// (Helper) access to BRAM port A write thru mode
#[derive(Debug)]
struct WriteThruA {}
impl FieldPositionCalculator for WriteThruA {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        TileRelativeBitPos { x: 35, y: 9 }
    }
}
/// (Helper) access to BRAM port B write thru mode
#[derive(Debug)]
struct WriteThruB {}
impl FieldPositionCalculator for WriteThruB {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        TileRelativeBitPos { x: 34, y: 59 }
    }
}
/// (Helper) access to BRAM port A in reg reset enable
#[derive(Debug)]
struct UseRstInA {}
impl FieldPositionCalculator for UseRstInA {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        TileRelativeBitPos { x: 35, y: 55 }
    }
}
/// (Helper) access to BRAM port B in reg reset enable
#[derive(Debug)]
struct UseRstInB {}
impl FieldPositionCalculator for UseRstInB {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        TileRelativeBitPos { x: 35, y: 61 }
    }
}
/// (Helper) access to BRAM port A out reg reset enable
#[derive(Debug)]
struct UseRstOutA {}
impl FieldPositionCalculator for UseRstOutA {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        TileRelativeBitPos { x: 35, y: 59 }
    }
}
/// (Helper) access to BRAM port B out reg reset enable
#[derive(Debug)]
struct UseRstOutB {}
impl FieldPositionCalculator for UseRstOutB {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        TileRelativeBitPos { x: 35, y: 65 }
    }
}
/// (Helper) access to BRAM port A in reg clock enable
#[derive(Debug)]
struct UseClkEnInA {}
impl FieldPositionCalculator for UseClkEnInA {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        TileRelativeBitPos { x: 35, y: 3 }
    }
}
/// (Helper) access to BRAM port B in reg clock enable
#[derive(Debug)]
struct UseClkEnInB {}
impl FieldPositionCalculator for UseClkEnInB {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        TileRelativeBitPos { x: 35, y: 19 }
    }
}
/// (Helper) access to BRAM port A out reg clock enable
#[derive(Debug)]
struct UseClkEnOutA {}
impl FieldPositionCalculator for UseClkEnOutA {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        TileRelativeBitPos { x: 35, y: 7 }
    }
}
/// (Helper) access to BRAM port B out reg clock enable
#[derive(Debug)]
struct UseClkEnOutB {}
impl FieldPositionCalculator for UseClkEnOutB {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        TileRelativeBitPos { x: 35, y: 29 }
    }
}

#[derive(Debug)]
struct RsenDly {}
impl FieldPositionCalculator for RsenDly {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        [
            TileRelativeBitPos { x: 34, y: 13 },
            TileRelativeBitPos { x: 34, y: 23 },
        ][biti]
    }
}
#[derive(Debug)]
struct DlyTime {}
impl FieldPositionCalculator for DlyTime {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        [
            TileRelativeBitPos { x: 34, y: 25 },
            TileRelativeBitPos { x: 34, y: 39 },
        ][biti]
    }
}

/// Block RAM port clocking mode
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[bitmux::bitenum]
pub enum ClockMode {
    /// Port A uses clock 0, port B uses clock 1
    Independent = "00",
    /// Input registers use clock 0, output registers use clock 1
    InputOutput = "01",
    /// Input data uses clock 0, output data uses clock 1, input address port A/B uses clock 0/1
    ///
    /// It appears that the intended use case for this might be that
    /// port A only performs writes (using clock 0),
    /// and port B only performs reads (using clock 1).
    ReadWrite = "1x",
}
impl Default for ClockMode {
    fn default() -> Self {
        Self::Independent
    }
}
impl Display for ClockMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Independent => write!(f, "independent"),
            Self::InputOutput => write!(f, "input_output"),
            Self::ReadWrite => write!(f, "read_write"),
        }
    }
}
impl FromStr for ClockMode {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("independent") {
            Ok(Self::Independent)
        } else if s.eq_ignore_ascii_case("input_output") {
            Ok(Self::InputOutput)
        } else if s.eq_ignore_ascii_case("read_write") {
            Ok(Self::ReadWrite)
        } else {
            Err(())
        }
    }
}
/// (Helper) access to BRAM clocking mode
#[derive(Debug)]
struct ClkModeRef {}
impl FieldPositionCalculator for ClkModeRef {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        [
            TileRelativeBitPos { x: 35, y: 49 },
            TileRelativeBitPos { x: 34, y: 49 },
        ][biti]
    }
}

/// (Helper) access to BRAM "packed" mode
#[derive(Debug)]
struct PackedModeAddressOverride {}
impl FieldPositionCalculator for PackedModeAddressOverride {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        TileRelativeBitPos { x: 34, y: 3 }
    }
}

impl<D: DebugTracer, Ref: Borrow<Bitstream<D>>> BRAMTileRef<D, Ref> {
    pub fn init_data(&self, sink: &mut impl BitSink) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: InitVal {},
            _d: PhantomData,
        };
        for i in 0..9216 {
            sink.set(i, ref_.get_bit(i));
        }
    }
}
impl<D: DebugTracer, Ref: BorrowMut<Bitstream<D>>> BRAMTileRef<D, Ref> {
    pub fn set_init_data(&mut self, source: &impl BitSource) {
        let mut ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: InitVal {},
            _d: PhantomData,
        };
        for i in 0..9216 {
            ref_.set_bit(i, source.get(i));
        }
    }
}

magic_tile_impl_gen! {
    impl BRAMTileRef {
        pub fn global_to_local(&self, inp_idx: u8) -> GlobalToLocalMux {
            GlobalToLocalMuxRef {
                is_bram: true,
                i: inp_idx,
            }
        }

        pub fn control_signal_preselect(&self, inp_idx: u8) -> CtrlMux {
            CtrlMuxRef {
                is_bram: true,
                i: inp_idx,
            }
        }

        pub fn imux(&self, i: u8) -> IMUX {
            IMUXRef { is_bram: true, i }
        }
        pub fn addr_a(&self, bit: u8) -> IMUX = {
            assert!(bit < 13, "invalid address bit index");
            self.imux(12 - bit)
        }
        pub fn addr_b(&self, bit: u8) -> IMUX = {
            assert!(bit < 13, "invalid address bit index");
            self.imux(51 + bit)
        }
        pub fn data_in_a(&self, bit: u8) -> IMUX = {
            assert!(bit < 18, "invalid data bit index");
            self.imux(30 - bit)
        }
        pub fn data_in_b(&self, bit: u8) -> IMUX = {
            assert!(bit < 18, "invalid data bit index");
            self.imux(33 + bit)
        }
        pub fn imux_xtra(&self, idx: u8) -> IMUX = {
            assert!(idx < 2, "invalid extra IMUX index");
            self.imux(31 + idx)
        }

        pub fn tmux(&self, i: u8) -> TMUX {
            TMUXRef(i)
        }
        pub fn kmux(&self, i: u8) -> KMUX {
            KMUXRef(i)
        }
        pub fn read_en_a(&self) -> KMUX = {
            self.kmux(6)
        }
        pub fn read_en_b(&self) -> KMUX = {
            self.kmux(7)
        }
        pub fn write_en_a(&self) -> KMUX = {
            self.kmux(3)
        }
        pub fn write_en_b(&self) -> KMUX = {
            self.kmux(0)
        }
        pub fn addr_stall_a(&self) -> KMUX = {
            self.kmux(4)
        }
        pub fn addr_stall_b(&self) -> KMUX = {
            self.kmux(5)
        }
        pub fn byte_en_a(&self, bit: u8) -> KMUX = {
            assert!(bit < 2, "invalid byte enable bit index");
            self.kmux(1 + bit)
        }
        pub fn byte_en_b(&self, bit: u8) -> KMUX = {
            assert!(bit < 2, "invalid byte enable bit index");
            self.kmux(8 + bit)
        }

        pub fn clock_mux(&self, clk_idx: u8) -> Mux3Inv {
            TileClk(clk_idx)
        }
        pub fn clock_en_mux(&self, clk_idx: u8) -> Mux3Inv {
            TileClkEn(clk_idx)
        }
        pub fn async_mux(&self, clk_idx: u8) -> Mux3Inv {
            TileAsync(clk_idx)
        }

        pub fn use_packed_mode_address_override(&self) -> bool {
            PackedModeAddressOverride {}
        }
        pub fn clock_choices_mode(&self) -> ClockMode {
            ClkModeRef {}
        }

        pub fn width_a(&self) -> PortWidth {
            PortAWidth {}
        }
        pub fn width_b(&self) -> PortWidth {
            PortBWidth {}
        }

        pub fn use_output_register_a(&self) -> bool {
            OutRegA {}
        }
        pub fn use_output_register_b(&self) -> bool {
            OutRegB {}
        }

        pub fn use_rst_in_a(&self) -> bool {
            UseRstInA {}
        }
        pub fn use_rst_in_b(&self) -> bool {
            UseRstInB {}
        }
        pub fn use_rst_out_a(&self) -> bool {
            UseRstOutA {}
        }
        pub fn use_rst_out_b(&self) -> bool {
            UseRstOutB {}
        }

        pub fn use_clk_en_in_a(&self) -> bool {
            UseClkEnInA {}
        }
        pub fn use_clk_en_in_b(&self) -> bool {
            UseClkEnInB {}
        }
        pub fn use_clk_en_out_a(&self) -> bool {
            UseClkEnOutA {}
        }
        pub fn use_clk_en_out_b(&self) -> bool {
            UseClkEnOutB {}
        }

        pub fn write_thru_a(&self) -> bool {
            WriteThruA {}
        }
        pub fn write_thru_b(&self) -> bool {
            WriteThruB {}
        }

        pub fn rsen_delay(&self) -> 2 bits in u8 {
            RsenDly {}
        }
        pub fn delay_time(&self) -> 2 bits in u8 {
            DlyTime {}
        }
    }
}

magic_tile_impl_gen! {
    impl on BRAMTileRef trait GenericRoutingRefTrait, GenericRoutingRefMutTrait {
        fn rmux(&self, rmux_idx: u8) -> RMUX {
            RMUXRef {
                is_bram: true,
                i: rmux_idx,
            }
        }
    }
}
