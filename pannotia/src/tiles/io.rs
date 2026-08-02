//! I/O tiles
//!
//! Because I/O tiles are on the edge of the array,
//! they integrate a bit differently into the general routing.
//! The biggest difference is that wires which would normally
//! go "over"/"through" the tile instead "fold back around"
//! and exit in the opposite direction from which they came.
//! (These wires _can_ also be used as inputs into the tile's function.)
//!
//! All I/O elements contain embedded registers
//! which can be bypassed on the output path. On the input path,
//! both the registered and unregistered signals can be used.
//!
//! ## Left/right IOs
//!
//! These tiles contain 6 I/Os. Outputs are driven onto neighbor wires (wasting 4 of 16).
//!
//! Supposedly, `T4X` wires are driven by a `LoopMUX`, but it is not clear if this actually exists.
//!
//! # Top/bottom IOs
//!
//! These tiles contain 4 I/Os. Outputs are driven onto `T4Y` wires (wasting 4 of 12).

use std::fmt::Display;

use super::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum TopBottomIOLocalMux {
    None,
    I(u8),
}
impl Default for TopBottomIOLocalMux {
    fn default() -> Self {
        Self::None
    }
}
impl Display for TopBottomIOLocalMux {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "<unset>"),
            Self::I(i) => write!(f, "#{i}"),
        }
    }
}
impl bitmux::BitstreamField for TopBottomIOLocalMux {
    fn get(b: impl bitmux::BitGetter) -> Self {
        let bits = b.get_bits::<6>();
        bitmux::twohot!(2, 3, match bits {
            #bits => Self::I(#val),
            0b1_00_000 => Self::I(6),
            0 => Self::None,
            _ => panic!("invalid TopBottomIOLocalMux {bits:06b}"),
        })
    }
    fn set(&self, mut b: impl bitmux::BitSetter) {
        let bits = bitmux::twohot!(2, 3, match self {
            Self::I(#val) => #bits,
            Self::I(6) => 0b1_00_000,
            Self::None => 0,
            _ => panic!("invalid TopBottomIOLocalMux {}", self),
        });
        b.set_bits::<6>(bits);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum LeftRightIOLocalMux {
    None,
    I(u8),
}
impl Default for LeftRightIOLocalMux {
    fn default() -> Self {
        Self::None
    }
}
impl Display for LeftRightIOLocalMux {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "<unset>"),
            Self::I(i) => write!(f, "#{i}"),
        }
    }
}
impl bitmux::BitstreamField for LeftRightIOLocalMux {
    fn get(b: impl bitmux::BitGetter) -> Self {
        let bits = b.get_bits::<7>();
        bitmux::twohot!(2, 4, match bits {
            #bits => Self::I(#val),
            0b1_00_0000 => Self::I(8),
            0 => Self::None,
            _ => panic!("invalid LeftRightIOLocalMux {bits:07b}"),
        })
    }
    fn set(&self, mut b: impl bitmux::BitSetter) {
        let bits = bitmux::twohot!(2, 4, match self {
            Self::I(#val) => #bits,
            Self::I(8) => 0b1_00_0000,
            Self::None => 0,
            _ => panic!("invalid LeftRightIOLocalMux {}", self),
        });
        b.set_bits::<7>(bits);
    }
}

struct TopBottomIOLocalLine {
    is_bottom: bool,
    i: u8,
}
impl FieldPositionCalculator for TopBottomIOLocalLine {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.i < 32, "local line index out of range");

        // there are 4 columns of 8 muxes
        let col_of_8 = self.i / 8;
        let idx_within_8 = (self.i % 8) as u32;

        // within each column, the entries count down for the first 4,
        // then the entries count *up* for the second 4,
        // but the second half does *not* flip vertically.
        // there is also a gap of 4 bit rows in between them
        let ybase = if idx_within_8 < 4 {
            2 + 2 * idx_within_8
        } else {
            20 - 2 * (idx_within_8 - 4)
        };

        // now we can look up the basic shape
        let (xbase, mut y) = bitmux::bittable!(
            (#x, ybase + #y),
            3   2   0,
            5   4   1,
        )[biti];

        // the odd columns are mirrored horizontally,
        // and there's a 1-column gap between cols 1 and 2
        let x = match col_of_8 {
            0 => xbase,
            1 => 5 - xbase,
            2 => 7 + xbase,
            3 => 12 - xbase,
            _ => unreachable!(),
        };

        // a bottom IO is entirely mirrored
        if self.is_bottom {
            y = 21 - y;
        }

        TileRelativeBitPos { y, x }
    }
}

struct LeftRightIOLocalLine(u8);
impl FieldPositionCalculator for LeftRightIOLocalLine {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 48, "local line index out of range");

        // there are 2 columns of 24, where the bits flip horizontally,
        // but they are in a slightly rearranged order
        let (is_rhs_column, row_within_column) = match self.0 {
            0..=11 => (true, self.0 as u32),
            12..=23 => (false, self.0 as u32 - 6),
            24..=35 => (true, self.0 as u32 - 12),
            36..=41 => (false, self.0 as u32 - 36),
            42..=47 => (false, self.0 as u32 - 24),
            _ => unreachable!(),
        };

        // there is a large gap in the middle
        let ybase = row_within_column * 2 + if row_within_column >= 12 { 20 } else { 0 };

        // now look up the basic shape
        let (xbase, y) = bitmux::bittable!(
            (#x, ybase + #y),
            0   2   4   6,
            1   3   5   .,
        )[biti];

        // finally deal with the columns
        let x = if is_rhs_column {
            16 + xbase
        } else {
            15 - xbase
        };

        TileRelativeBitPos { y, x }
    }
}

struct TopBottomIOGlobal2Local {
    is_bottom: bool,
    i: u8,
}
impl FieldPositionCalculator for TopBottomIOGlobal2Local {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.i < 8, "GlobalToLocalMux index out of range");

        // these muxes are permuted in the following order
        let (xbase, mut y) = [
            (15, 10),
            (15, 13),
            (15, 11),
            (15, 12),
            (23, 10),
            (23, 13),
            (23, 11),
            (23, 12),
        ][self.i as usize];

        // bits are otherwise just linear
        let x = xbase + biti as u32;

        // a bottom IO is entirely mirrored
        if self.is_bottom {
            y = 21 - y;
        }

        TileRelativeBitPos { y, x }
    }
}

struct LeftRightIOGlobal2Local(u8);
impl FieldPositionCalculator for LeftRightIOGlobal2Local {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 12, "GlobalToLocalMux index out of range");

        // there are two columns, they are mirrored horizontally
        // the RHS column has the evens, the LHS column has the odds
        let is_rhs_column = self.0 % 2 == 0;

        // the rows are as follows
        let y = [24, 27, 28, 39, 40, 43][self.0 as usize / 2];

        // straight line blocks, "fanning out" from the center
        let x = if is_rhs_column {
            12 + biti as u32
        } else {
            11 - biti as u32
        };

        TileRelativeBitPos { y, x }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum IOLocalToClockMux {
    None,
    I(u8),
}
impl Default for IOLocalToClockMux {
    fn default() -> Self {
        Self::None
    }
}
impl Display for IOLocalToClockMux {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "<unset>"),
            Self::I(i) => write!(f, "#{i}"),
        }
    }
}
impl bitmux::BitstreamField for IOLocalToClockMux {
    fn get(b: impl bitmux::BitGetter) -> Self {
        let bits = b.get_bits::<6>();
        bitmux::twohot!(2, 4, match bits {
            #bits => Self::I(#val),
            0 => Self::None,
            _ => panic!("invalid IOLocalToClockMux {bits:06b}"),
        })
    }
    fn set(&self, mut b: impl bitmux::BitSetter) {
        let bits = bitmux::twohot!(2, 4, match self {
            Self::I(#val) => #bits,
            Self::None => 0,
            _ => panic!("invalid IOLocalToClockMux {}", self),
        });
        b.set_bits::<6>(bits);
    }
}

struct TopBottomIOLocal2Clk {
    is_bottom: bool,
    i: u8,
}
impl FieldPositionCalculator for TopBottomIOLocal2Clk {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.i < 8, "LocalToClockMux index out of range");

        // there are 4 columns of 2 muxes, but they're out of order.
        // the odd columns are also mirrored horizontally,
        // and there's a 1-column gap between cols 1 and 2
        let (xbase, ybase, need_xflip) = [
            (0, 10, false),
            (0, 12, false),
            (7, 10, false),
            (7, 12, false),
            (5, 10, true),
            (5, 12, true),
            (12, 10, true),
            (12, 12, true),
        ][self.i as usize];

        // now we can look up the basic shape
        let (xoffs, mut y) = bitmux::bittable!(
            (#x, ybase + #y),
            4   2   0,
            5   3   1,
        )[biti];

        let x = if !need_xflip {
            xbase + xoffs
        } else {
            xbase - xoffs
        };

        // a bottom IO is entirely mirrored
        if self.is_bottom {
            y = 21 - y;
        }

        TileRelativeBitPos { y, x }
    }
}

struct LeftRightIOLocal2Clk(u8);
impl FieldPositionCalculator for LeftRightIOLocal2Clk {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 12, "LocalToClockMux index out of range");

        // there are *3* (yes, an odd number) chunks of these muxes
        let (xbase, ybase, need_xflip) = match self.0 {
            0..=3 => (10, 16 + 2 * (self.0 as u32), true),
            4..=7 => (5, 16 + 2 * (self.0 as u32 - 4), false),
            8..=11 => (5, 44 + 2 * (self.0 as u32 - 8), false),
            _ => unreachable!(),
        };

        // now we can look up the basic shape
        let (xoffs, y) = bitmux::bittable!(
            (#x, ybase + #y),
            4   2   0,
            5   3   1,
        )[biti];

        let x = if !need_xflip {
            xbase + xoffs
        } else {
            xbase - xoffs
        };

        TileRelativeBitPos { y, x }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum LocalToIOMux {
    VCC,
    GND,
    I { invert: bool, i: u8 },
}
impl Default for LocalToIOMux {
    fn default() -> Self {
        Self::VCC
    }
}
impl Display for LocalToIOMux {
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
impl bitmux::BitstreamField for LocalToIOMux {
    fn get(b: impl bitmux::BitGetter) -> Self {
        let bits = b.get_bits::<7>();
        let invert = bits & 0b1_00_0000 != 0;
        bitmux::twohot!(2, 4, match bits & 0b11_1111 {
            #bits => Self::I { invert, i: #val },
            0 if invert => Self::GND,
            0 if !invert => Self::VCC,
            _ => panic!("invalid LocalToIOMux {bits:07b}"),
        })
    }
    fn set(&self, mut b: impl bitmux::BitSetter) {
        let mut bits = bitmux::twohot!(2, 4, match self {
            Self::I { i: #val, .. } => #bits,
            Self::GND => 0b1_00_0000,
            Self::VCC => 0,
        _ => panic!("invalid LocalToIOMux {}", self),
        });
        if let Self::I { invert: true, .. } = self {
            bits |= 0b1_00_0000;
        }
        b.set_bits::<7>(bits);
    }
}

struct TopBottomIOLocal2IO {
    is_bottom: bool,
    i: u8,
}
impl FieldPositionCalculator for TopBottomIOLocal2IO {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.i < 24, "local to io index out of range");

        // there are 3 columns of 8 muxes
        let col_of_8 = self.i / 8;
        let idx_within_8 = (self.i % 8) as u32;

        // we are using our own custom numbering,
        // so each column just counts down.
        // but there is still a gap of 4 rows in the middle
        let ybase = 2 + 2 * idx_within_8 + if idx_within_8 >= 4 { 4 } else { 0 };

        // now we can look up the basic shape
        let (xbase, mut y) = bitmux::bittable!(
            (#x, ybase + #y),
            .   4   2   0,
            6   5   3   1,
        )[biti];

        // the middle column is mirrored horizontally
        let x = match col_of_8 {
            0 => 15 + xbase,
            1 => 22 - xbase,
            2 => 23 + xbase,
            _ => unreachable!(),
        };

        // a bottom IO is entirely mirrored
        if self.is_bottom {
            y = 21 - y;
        }

        TileRelativeBitPos { y, x }
    }
}

struct LeftRightIOLocal2IO(u8);
impl FieldPositionCalculator for LeftRightIOLocal2IO {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 36, "local to io index out of range");

        // there are 2 columns, of 16 and 20 entries
        // they are "mainly" split into blocks of 8,
        // except the rhs column has a bonus 4 more
        // this is all simplified because of our custom numbering
        let (is_rhs_column, ybase) = match self.0 {
            0..=7 => (false, 2 * (self.0 as u32)),
            8..=15 => (false, 52 + 2 * (self.0 as u32 - 8)),
            16..=23 => (true, 2 * (self.0 as u32 - 16)),
            24..=35 => (true, 44 + 2 * (self.0 as u32 - 24)),
            _ => unreachable!(),
        };

        // now we can look up the basic shape
        let (xbase, y) = bitmux::bittable!(
            (#x, ybase + #y),
            .   4   2   0,
            6   5   3   1,
        )[biti];

        // the right column is mirrored horizontally
        let x = if !is_rhs_column {
            4 + xbase
        } else {
            11 - xbase
        };

        TileRelativeBitPos { y, x }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum IOClockMux {
    VCC,
    GND,
    ViaGlobalToLocal { invert: bool },
    ViaLocalToClock { invert: bool },
}
impl Default for IOClockMux {
    fn default() -> Self {
        Self::VCC
    }
}
impl Display for IOClockMux {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VCC => write!(f, "VCC"),
            Self::GND => write!(f, "GND"),
            Self::ViaGlobalToLocal { invert } => {
                if *invert {
                    write!(f, "!")?;
                }
                write!(f, "glb2loc")
            }
            Self::ViaLocalToClock { invert } => {
                if *invert {
                    write!(f, "!")?;
                }
                write!(f, "loc2clk")
            }
        }
    }
}
impl bitmux::BitstreamField for IOClockMux {
    fn get(b: impl bitmux::BitGetter) -> Self {
        let bits = b.get_bits::<3>();
        let invert = bits & 0b100 != 0;
        match bits & 0b11 {
            0b01 => Self::ViaLocalToClock { invert },
            0b10 => Self::ViaGlobalToLocal { invert },
            0b00 if invert => Self::GND,
            0b00 if !invert => Self::VCC,
            _ => panic!("invalid IOClockMux {bits:03b}"),
        }
    }
    fn set(&self, mut b: impl bitmux::BitSetter) {
        let bits = match self {
            Self::VCC => 0b000,
            Self::GND => 0b100,
            Self::ViaLocalToClock { invert } => 0b01 | if *invert { 0b100 } else { 0 },
            Self::ViaGlobalToLocal { invert } => 0b10 | if *invert { 0b100 } else { 0 },
        };
        b.set_bits::<3>(bits);
    }
}

struct TopBottomIOClockMux {
    is_bottom: bool,
    i: u8,
}
impl FieldPositionCalculator for TopBottomIOClockMux {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.i < 8, "clock mux index out of range");

        // We just have this giant bag of bits
        let (x, mut y) = bitmux::bittable!(
            (27 + #x, #y),
            .   .   .   .   .   .   .,
            .   .   .   .   .   .   .,
            .   .   .   .   .   .   .,
            .   .   .   .   .   .   .,
            .   .   .   .   .   .   .,
            .   .   .   .   .   .   .,
            .   .   .   .   .   .   .,
            .   .   .   .   .   .   .,
            0   1   .   .   .   7   6,
            12   13 .   .   .   19  18,
            .   .   .   .   .   20  8,
            .   .   .   .   .   14  2,
            .   .   .   .   .   17  5,
            .   .   .   .   .   23  11,
            15  16  .   .   .   22  21,
            3   4   .   .   .   10  9,
            .   .   .   .   .   .   .,
            .   .   .   .   .   .   .,
            .   .   .   .   .   .   .,
            .   .   .   .   .   .   .,
            .   .   .   .   .   .   .,
            .   .   .   .   .   .   .,
        )[self.i as usize * 3 + biti];

        // a bottom IO is entirely mirrored
        if self.is_bottom {
            y = 21 - y;
        }

        TileRelativeBitPos { y, x }
    }
}

struct LeftRightIOClockMux(u8);
impl FieldPositionCalculator for LeftRightIOClockMux {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 12, "clock mux index out of range");

        // We just have this giant bag of bits
        bitmux::bittable!(
            TileRelativeBitPos { x: 2 + #x, y: 24 + #y },
            0   1   2   5,
            3   4   .   .,
            6   7   .   .,
            9   10  11  8,
            12  13  14  17,
            15  16  .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            18  19  .   .,
            21  22  23  20,
            24  25  26  29,
            27  28  .   .,
            30  31  .   .,
            33  34  35  32,
        )[self.0 as usize * 3 + biti]
    }
}

struct TopBottomIOOutMux {
    is_bottom: bool,
    i: u8,
}
impl FieldPositionCalculator for TopBottomIOOutMux {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.i < 8, "out mux index out of range");

        let (x, mut y) = [
            (27, 0),
            (27, 1),
            (33, 0),
            (32, 0),
            (28, 0),
            (28, 1),
            (33, 1),
            (32, 1),
        ][self.i as usize];

        // a bottom IO is entirely mirrored
        if self.is_bottom {
            y = 21 - y;
        }

        TileRelativeBitPos { y, x }
    }
}

struct LeftRightIOOutMux(u8);
impl FieldPositionCalculator for LeftRightIOOutMux {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 12, "out mux index out of range");

        let y = [0, 2, 10, 12, 18, 20, 44, 46, 48, 50, 58, 60][self.0 as usize];

        TileRelativeBitPos { y, x: 3 }
    }
}

struct TopBottomIOInDaDelay {
    is_bottom: bool,
    i: u8,
}
impl FieldPositionCalculator for TopBottomIOInDaDelay {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.i < 4, "io instance index out of range");

        let (xbase, mut y) = [(4, 0), (4, 1), (21, 0), (21, 1)][self.i as usize];

        let x = xbase - biti as u32;

        // a bottom IO is entirely mirrored
        if self.is_bottom {
            y = 21 - y;
        }

        TileRelativeBitPos { y, x }
    }
}

struct LeftRightIOInDaDelay(u8);
impl FieldPositionCalculator for LeftRightIOInDaDelay {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 6, "io instance index out of range");

        bitmux::bittable!(
            TileRelativeBitPos { x: #x, y: self.0 as u32 * 10 + #y },
            .   .,
            .   .,
            .   .,
            .   .,
            .   .,
            .   .,
            1   0,
            .   2,
            .   .,
            .   .,
        )[biti]
    }
}

struct TopBottomIOInRegDelay {
    is_bottom: bool,
    i: u8,
}
impl FieldPositionCalculator for TopBottomIOInRegDelay {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.i < 4, "io instance index out of range");

        let (xbase, mut y) = [(6, 0), (6, 1), (23, 0), (23, 1)][self.i as usize];

        let x = if self.i < 2 {
            xbase + biti as u32
        } else {
            xbase + [1, 0, 2][biti]
        };

        // a bottom IO is entirely mirrored
        if self.is_bottom {
            y = 21 - y;
        }

        TileRelativeBitPos { y, x }
    }
}

struct LeftRightIOInRegDelay(u8);
impl FieldPositionCalculator for LeftRightIOInRegDelay {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 6, "io instance index out of range");

        bitmux::bittable!(
            TileRelativeBitPos { x: #x, y: self.0 as u32 * 10 + #y },
            .   .,
            .   .,
            .   .,
            .   .,
            .   .,
            .   .,
            .   .,
            0   .,
            1   2,
            .   .,
        )[biti]
    }
}

struct TopBottomOutDelay {
    is_bottom: bool,
    i: u8,
}
impl FieldPositionCalculator for TopBottomOutDelay {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.i < 4, "io instance index out of range");

        let (x, mut y) = [(15, 0), (15, 1), (26, 0), (26, 1)][self.i as usize];

        // a bottom IO is entirely mirrored
        if self.is_bottom {
            y = 21 - y;
        }

        TileRelativeBitPos { y, x }
    }
}

struct LeftRightOutDelay(u8);
impl FieldPositionCalculator for LeftRightOutDelay {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 6, "io instance index out of range");

        TileRelativeBitPos {
            x: 0,
            y: self.0 as u32 * 10 + 9,
        }
    }
}

struct TopBottomOutReg {
    is_bottom: bool,
    i: u8,
}
impl FieldPositionCalculator for TopBottomOutReg {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.i < 4, "io instance index out of range");

        let (x, mut y) = [(28, 7), (32, 7), (28, 16), (32, 16)][self.i as usize];

        // a bottom IO is entirely mirrored
        if self.is_bottom {
            y = 21 - y;
        }

        TileRelativeBitPos { y, x }
    }
}

struct LeftRightOutReg(u8);
impl FieldPositionCalculator for LeftRightOutReg {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 6, "io instance index out of range");

        TileRelativeBitPos {
            x: 0,
            y: self.0 as u32 * 10 + 5,
        }
    }
}

struct TopBottomOEReg {
    is_bottom: bool,
    i: u8,
}
impl FieldPositionCalculator for TopBottomOEReg {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.i < 4, "io instance index out of range");

        let (x, mut y) = [(28, 5), (32, 5), (28, 18), (32, 18)][self.i as usize];

        // a bottom IO is entirely mirrored
        if self.is_bottom {
            y = 21 - y;
        }

        TileRelativeBitPos { y, x }
    }
}

struct LeftRightOEReg(u8);
impl FieldPositionCalculator for LeftRightOEReg {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 6, "io instance index out of range");

        TileRelativeBitPos {
            x: 0,
            y: self.0 as u32 * 10 + 3,
        }
    }
}

struct TopBottomInPU {
    is_bottom: bool,
    i: u8,
}
impl FieldPositionCalculator for TopBottomInPU {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.i < 4, "io instance index out of range");

        let (x, mut y) = [(27, 3), (33, 3), (27, 20), (33, 20)][self.i as usize];

        // a bottom IO is entirely mirrored
        if self.is_bottom {
            y = 21 - y;
        }

        TileRelativeBitPos { y, x }
    }
}

struct TopBottomOutPU {
    is_bottom: bool,
    i: u8,
}
impl FieldPositionCalculator for TopBottomOutPU {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.i < 4, "io instance index out of range");

        let (x, mut y) = [(27, 5), (33, 5), (27, 18), (33, 18)][self.i as usize];

        // a bottom IO is entirely mirrored
        if self.is_bottom {
            y = 21 - y;
        }

        TileRelativeBitPos { y, x }
    }
}

struct TopBottomOEPU {
    is_bottom: bool,
    i: u8,
}
impl FieldPositionCalculator for TopBottomOEPU {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.i < 4, "io instance index out of range");

        let (x, mut y) = [(27, 7), (33, 7), (27, 16), (33, 16)][self.i as usize];

        // a bottom IO is entirely mirrored
        if self.is_bottom {
            y = 21 - y;
        }

        TileRelativeBitPos { y, x }
    }
}

struct LeftRightInPU(u8);
impl FieldPositionCalculator for LeftRightInPU {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 6, "io instance index out of range");

        TileRelativeBitPos {
            x: 1,
            y: self.0 as u32 * 10 + 1,
        }
    }
}

struct LeftRightOutPU(u8);
impl FieldPositionCalculator for LeftRightOutPU {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 6, "io instance index out of range");

        TileRelativeBitPos {
            x: 1,
            y: self.0 as u32 * 10 + 3,
        }
    }
}

struct LeftRightOEPU(u8);
impl FieldPositionCalculator for LeftRightOEPU {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 6, "io instance index out of range");

        TileRelativeBitPos {
            x: 1,
            y: self.0 as u32 * 10 + 5,
        }
    }
}

/// The function of the sync/async control signal on the IOB register
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[bitmux::bitenum]
pub enum RegCtrlMode {
    Reset = "0",
    Set = "1",
}
impl Display for RegCtrlMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Reset => write!(f, "R"),
            Self::Set => write!(f, "S"),
        }
    }
}
impl Default for RegCtrlMode {
    fn default() -> Self {
        Self::Reset
    }
}
impl From<bool> for RegCtrlMode {
    fn from(value: bool) -> Self {
        match value {
            false => Self::Reset,
            true => Self::Set,
        }
    }
}
impl From<RegCtrlMode> for bool {
    fn from(value: RegCtrlMode) -> Self {
        match value {
            RegCtrlMode::Reset => false,
            RegCtrlMode::Set => true,
        }
    }
}

struct TopBottomInSMode {
    is_bottom: bool,
    i: u8,
}
impl FieldPositionCalculator for TopBottomInSMode {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.i < 4, "io instance index out of range");

        let (x, mut y) = [(27, 2), (33, 2), (27, 21), (33, 21)][self.i as usize];

        // a bottom IO is entirely mirrored
        if self.is_bottom {
            y = 21 - y;
        }

        TileRelativeBitPos { y, x }
    }
}

struct TopBottomInAMode {
    is_bottom: bool,
    i: u8,
}
impl FieldPositionCalculator for TopBottomInAMode {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.i < 4, "io instance index out of range");

        let (x, mut y) = [(28, 2), (32, 2), (28, 21), (32, 21)][self.i as usize];

        // a bottom IO is entirely mirrored
        if self.is_bottom {
            y = 21 - y;
        }

        TileRelativeBitPos { y, x }
    }
}

struct TopBottomOutSMode {
    is_bottom: bool,
    i: u8,
}
impl FieldPositionCalculator for TopBottomOutSMode {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.i < 4, "io instance index out of range");

        let (x, mut y) = [(27, 4), (33, 4), (27, 19), (33, 19)][self.i as usize];

        // a bottom IO is entirely mirrored
        if self.is_bottom {
            y = 21 - y;
        }

        TileRelativeBitPos { y, x }
    }
}

struct TopBottomOutAMode {
    is_bottom: bool,
    i: u8,
}
impl FieldPositionCalculator for TopBottomOutAMode {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.i < 4, "io instance index out of range");

        let (x, mut y) = [(28, 4), (32, 4), (28, 19), (32, 19)][self.i as usize];

        // a bottom IO is entirely mirrored
        if self.is_bottom {
            y = 21 - y;
        }

        TileRelativeBitPos { y, x }
    }
}

struct TopBottomOESMode {
    is_bottom: bool,
    i: u8,
}
impl FieldPositionCalculator for TopBottomOESMode {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.i < 4, "io instance index out of range");

        let (x, mut y) = [(27, 6), (33, 6), (27, 17), (33, 17)][self.i as usize];

        // a bottom IO is entirely mirrored
        if self.is_bottom {
            y = 21 - y;
        }

        TileRelativeBitPos { y, x }
    }
}

struct TopBottomOEAMode {
    is_bottom: bool,
    i: u8,
}
impl FieldPositionCalculator for TopBottomOEAMode {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.i < 4, "io instance index out of range");

        let (x, mut y) = [(28, 6), (32, 6), (28, 17), (32, 17)][self.i as usize];

        // a bottom IO is entirely mirrored
        if self.is_bottom {
            y = 21 - y;
        }

        TileRelativeBitPos { y, x }
    }
}

struct LeftRightInSMode(u8);
impl FieldPositionCalculator for LeftRightInSMode {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 6, "io instance index out of range");

        TileRelativeBitPos {
            x: 1,
            y: self.0 as u32 * 10 + 0,
        }
    }
}

struct LeftRightInAMode(u8);
impl FieldPositionCalculator for LeftRightInAMode {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 6, "io instance index out of range");

        TileRelativeBitPos {
            x: 0,
            y: self.0 as u32 * 10 + 0,
        }
    }
}

struct LeftRightOutSMode(u8);
impl FieldPositionCalculator for LeftRightOutSMode {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 6, "io instance index out of range");

        TileRelativeBitPos {
            x: 1,
            y: self.0 as u32 * 10 + 2,
        }
    }
}

struct LeftRightOutAMode(u8);
impl FieldPositionCalculator for LeftRightOutAMode {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 6, "io instance index out of range");

        TileRelativeBitPos {
            x: 0,
            y: self.0 as u32 * 10 + 2,
        }
    }
}

struct LeftRightOESMode(u8);
impl FieldPositionCalculator for LeftRightOESMode {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 6, "io instance index out of range");

        TileRelativeBitPos {
            x: 1,
            y: self.0 as u32 * 10 + 4,
        }
    }
}

struct LeftRightOEAMode(u8);
impl FieldPositionCalculator for LeftRightOEAMode {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 6, "io instance index out of range");

        TileRelativeBitPos {
            x: 0,
            y: self.0 as u32 * 10 + 4,
        }
    }
}

/// Read (via possibly-dynamic dispatch) generic IO tile functionality
pub trait IOTileCommon {
    fn num_ios(&self) -> u8;

    fn global_to_local(&self, idx: u8) -> GlobalToLocalMux;
    fn local_to_clock(&self, idx: u8) -> IOLocalToClockMux;
    fn local_to_io(&self, custom_idx: u8) -> LocalToIOMux;
    fn clock_mux(&self, idx: u8) -> IOClockMux;
    fn out_mux(&self, io_idx: u8, out_idx: u8) -> super::logic::OMUX;

    fn out_clock_global_to_local(&self, io_idx: u8) -> GlobalToLocalMux {
        self.global_to_local(io_idx * 2 + 0)
    }
    fn in_clock_global_to_local(&self, io_idx: u8) -> GlobalToLocalMux {
        self.global_to_local(io_idx * 2 + 1)
    }
    fn out_clock_local_to_clock(&self, io_idx: u8) -> IOLocalToClockMux {
        self.local_to_clock(io_idx * 2 + 0)
    }
    fn in_clock_local_to_clock(&self, io_idx: u8) -> IOLocalToClockMux {
        self.local_to_clock(io_idx * 2 + 1)
    }
    fn out_clock_choice(&self, io_idx: u8) -> IOClockMux {
        self.clock_mux(io_idx * 2 + 0)
    }
    fn in_clock_choice(&self, io_idx: u8) -> IOClockMux {
        self.clock_mux(io_idx * 2 + 1)
    }

    fn local_to_io_out(&self, io_idx: u8) -> LocalToIOMux;
    fn local_to_io_oe(&self, io_idx: u8) -> LocalToIOMux;
    fn local_to_out_clk_en(&self, io_idx: u8) -> LocalToIOMux;
    fn local_to_in_clk_en(&self, io_idx: u8) -> LocalToIOMux;
    fn local_to_async_ctrl(&self, io_idx: u8) -> LocalToIOMux;
    fn local_to_sync_ctrl(&self, io_idx: u8) -> LocalToIOMux;

    fn in_data_delay(&self, io_idx: u8) -> u8;
    fn in_reg_delay(&self, io_idx: u8) -> u8;
    fn out_delay(&self, io_idx: u8) -> bool;

    fn out_use_reg(&self, io_idx: u8) -> bool;
    fn oe_use_reg(&self, io_idx: u8) -> bool;

    fn in_powerup_state(&self, io_idx: u8) -> bool;
    fn out_powerup_state(&self, io_idx: u8) -> bool;
    fn oe_powerup_state(&self, io_idx: u8) -> bool;

    fn in_async_mode(&self, io_idx: u8) -> RegCtrlMode;
    fn out_async_mode(&self, io_idx: u8) -> RegCtrlMode;
    fn oe_async_mode(&self, io_idx: u8) -> RegCtrlMode;

    fn in_sync_mode(&self, io_idx: u8) -> RegCtrlMode;
    fn out_sync_mode(&self, io_idx: u8) -> RegCtrlMode;
    fn oe_sync_mode(&self, io_idx: u8) -> RegCtrlMode;
}
/// Write (via possibly-dynamic dispatch) generic IO tile functionality
pub trait IOTileCommonMut: IOTileCommon {
    fn set_global_to_local(&mut self, idx: u8, val: GlobalToLocalMux);
    fn set_local_to_clock(&mut self, idx: u8, val: IOLocalToClockMux);
    fn set_local_to_io(&mut self, custom_idx: u8, val: LocalToIOMux);
    fn set_clock_mux(&mut self, idx: u8, val: IOClockMux);
    fn set_out_mux(&mut self, io_idx: u8, out_idx: u8, val: super::logic::OMUX);

    fn set_out_clock_global_to_local(&mut self, io_idx: u8, val: GlobalToLocalMux) {
        self.set_global_to_local(io_idx * 2 + 0, val);
    }
    fn set_in_clock_global_to_local(&mut self, io_idx: u8, val: GlobalToLocalMux) {
        self.set_global_to_local(io_idx * 2 + 1, val);
    }
    fn set_out_clock_local_to_clock(&mut self, io_idx: u8, val: IOLocalToClockMux) {
        self.set_local_to_clock(io_idx * 2 + 0, val);
    }
    fn set_in_clock_local_to_clock(&mut self, io_idx: u8, val: IOLocalToClockMux) {
        self.set_local_to_clock(io_idx * 2 + 1, val);
    }
    fn set_out_clock_choice(&mut self, io_idx: u8, val: IOClockMux) {
        self.set_clock_mux(io_idx * 2 + 0, val);
    }
    fn set_in_clock_choice(&mut self, io_idx: u8, val: IOClockMux) {
        self.set_clock_mux(io_idx * 2 + 1, val);
    }

    fn set_local_to_io_out(&mut self, io_idx: u8, val: LocalToIOMux);
    fn set_local_to_io_oe(&mut self, io_idx: u8, val: LocalToIOMux);
    fn set_local_to_out_clk_en(&mut self, io_idx: u8, val: LocalToIOMux);
    fn set_local_to_in_clk_en(&mut self, io_idx: u8, val: LocalToIOMux);
    fn set_local_to_async_ctrl(&mut self, io_idx: u8, val: LocalToIOMux);
    fn set_local_to_sync_ctrl(&mut self, io_idx: u8, val: LocalToIOMux);

    fn set_in_data_delay(&mut self, io_idx: u8, val: u8);
    fn set_in_reg_delay(&mut self, io_idx: u8, val: u8);
    fn set_out_delay(&mut self, io_idx: u8, val: bool);

    fn set_out_use_reg(&mut self, io_idx: u8, val: bool);
    fn set_oe_use_reg(&mut self, io_idx: u8, val: bool);

    fn set_in_powerup_state(&mut self, io_idx: u8, val: bool);
    fn set_out_powerup_state(&mut self, io_idx: u8, val: bool);
    fn set_oe_powerup_state(&mut self, io_idx: u8, val: bool);

    fn set_in_async_mode(&mut self, io_idx: u8, val: RegCtrlMode);
    fn set_out_async_mode(&mut self, io_idx: u8, val: RegCtrlMode);
    fn set_oe_async_mode(&mut self, io_idx: u8, val: RegCtrlMode);

    fn set_in_sync_mode(&mut self, io_idx: u8, val: RegCtrlMode);
    fn set_out_sync_mode(&mut self, io_idx: u8, val: RegCtrlMode);
    fn set_oe_sync_mode(&mut self, io_idx: u8, val: RegCtrlMode);
}

make_tile_ref! {
    /// Access to a top/bottom IO tile
    TopBottomIOTileRef = TileType::TopBottomIO
}

magic_tile_impl_gen! {
    impl TopBottomIOTileRef {
        pub fn local_line(&self, idx: u8) -> TopBottomIOLocalMux {
            TopBottomIOLocalLine {
                is_bottom: self.p.y == 0,
                i: idx,
            }
        }
    }
}
magic_tile_impl_gen! {
    impl on TopBottomIOTileRef trait IOTileCommon, IOTileCommonMut, get {
        fn num_ios(&self) -> u8 {
            4
        }
    } {
        fn global_to_local(&self, idx: u8) -> GlobalToLocalMux {
            TopBottomIOGlobal2Local {
                is_bottom: self.p.y == 0,
                i: idx,
            }
        }

        fn local_to_clock(&self, idx: u8) -> IOLocalToClockMux {
            TopBottomIOLocal2Clk {
                is_bottom: self.p.y == 0,
                i: idx,
            }
        }

        fn local_to_io(&self, custom_idx: u8) -> LocalToIOMux {
            TopBottomIOLocal2IO {
                is_bottom: self.p.y == 0,
                i: custom_idx,
            }
        }
        fn local_to_io_out(&self, io_idx: u8) -> LocalToIOMux = {
            self.local_to_io([0, 1, 7, 6][io_idx as usize])
        }
        fn local_to_io_oe(&self, io_idx: u8) -> LocalToIOMux = {
            self.local_to_io([2, 3, 5, 4][io_idx as usize])
        }
        fn local_to_out_clk_en(&self, io_idx: u8) -> LocalToIOMux = {
            self.local_to_io([10, 11, 13, 12][io_idx as usize])
        }
        fn local_to_in_clk_en(&self, io_idx: u8) -> LocalToIOMux = {
            self.local_to_io([8, 9, 15, 14][io_idx as usize])
        }
        fn local_to_async_ctrl(&self, io_idx: u8) -> LocalToIOMux = {
            self.local_to_io([16, 17, 23, 22][io_idx as usize])
        }
        fn local_to_sync_ctrl(&self, io_idx: u8) -> LocalToIOMux = {
            self.local_to_io([18, 19, 21, 20][io_idx as usize])
        }

        fn clock_mux(&self, idx: u8) -> IOClockMux {
            TopBottomIOClockMux {
                is_bottom: self.p.y == 0,
                i: idx,
            }
        }

        fn out_mux(&self, io_idx: u8, out_idx: u8) -> super::logic::OMUX {
            TopBottomIOOutMux {
                is_bottom: self.p.y == 0,
                i: io_idx * 2 + out_idx,
            }
        }

        fn in_data_delay(&self, io_idx: u8) -> 3 bits in u8 {
            TopBottomIOInDaDelay {
                is_bottom: self.p.y == 0,
                i: io_idx,
            }
        }
        fn in_reg_delay(&self, io_idx: u8) -> 3 bits in u8 {
            TopBottomIOInRegDelay {
                is_bottom: self.p.y == 0,
                i: io_idx,
            }
        }
        fn out_delay(&self, io_idx: u8) -> bool {
            TopBottomOutDelay {
                is_bottom: self.p.y == 0,
                i: io_idx,
            }
        }

        fn out_use_reg(&self, io_idx: u8) -> bool {
            TopBottomOutReg {
                is_bottom: self.p.y == 0,
                i: io_idx,
            }
        }
        fn oe_use_reg(&self, io_idx: u8) -> bool {
            TopBottomOEReg {
                is_bottom: self.p.y == 0,
                i: io_idx,
            }
        }

        fn in_powerup_state(&self, io_idx: u8) -> bool {
            TopBottomInPU {
                is_bottom: self.p.y == 0,
                i: io_idx,
            }
        }
        fn out_powerup_state(&self, io_idx: u8) -> bool {
            TopBottomOutPU {
                is_bottom: self.p.y == 0,
                i: io_idx,
            }
        }
        fn oe_powerup_state(&self, io_idx: u8) -> bool {
            TopBottomOEPU {
                is_bottom: self.p.y == 0,
                i: io_idx,
            }
        }

        fn in_async_mode(&self, io_idx: u8) -> RegCtrlMode {
            TopBottomInAMode {
                is_bottom: self.p.y == 0,
                i: io_idx,
            }
        }
        fn out_async_mode(&self, io_idx: u8) -> RegCtrlMode {
            TopBottomOutAMode {
                is_bottom: self.p.y == 0,
                i: io_idx,
            }
        }
        fn oe_async_mode(&self, io_idx: u8) -> RegCtrlMode {
            TopBottomOEAMode {
                is_bottom: self.p.y == 0,
                i: io_idx,
            }
        }

        fn in_sync_mode(&self, io_idx: u8) -> RegCtrlMode {
            TopBottomInSMode {
                is_bottom: self.p.y == 0,
                i: io_idx,
            }
        }
        fn out_sync_mode(&self, io_idx: u8) -> RegCtrlMode {
            TopBottomOutSMode {
                is_bottom: self.p.y == 0,
                i: io_idx,
            }
        }
        fn oe_sync_mode(&self, io_idx: u8) -> RegCtrlMode {
            TopBottomOESMode {
                is_bottom: self.p.y == 0,
                i: io_idx,
            }
        }
    }
}

make_tile_ref! {
    /// Access to a left/right IO tile
    LeftRightIOTileRef = TileType::LeftRightIO
}

magic_tile_impl_gen! {
    impl LeftRightIOTileRef {
        pub fn local_line(&self, idx: u8) -> LeftRightIOLocalMux {
            LeftRightIOLocalLine(idx)
        }
    }
}
magic_tile_impl_gen! {
    impl on LeftRightIOTileRef trait IOTileCommon, IOTileCommonMut, get {
        fn num_ios(&self) -> u8 {
            6
        }
    } {
        fn global_to_local(&self, idx: u8) -> GlobalToLocalMux {
            LeftRightIOGlobal2Local(idx)
        }

        fn local_to_clock(&self, idx: u8) -> IOLocalToClockMux {
            LeftRightIOLocal2Clk(idx)
        }

        fn local_to_io(&self, custom_idx: u8) -> LocalToIOMux {
            LeftRightIOLocal2IO(custom_idx)
        }
        fn local_to_io_out(&self, io_idx: u8) -> LocalToIOMux = {
            self.local_to_io(16 + io_idx)
        }
        fn local_to_io_oe(&self, io_idx: u8) -> LocalToIOMux = {
            self.local_to_io([22, 23, 0, 1, 2, 3][io_idx as usize])
        }
        fn local_to_out_clk_en(&self, io_idx: u8) -> LocalToIOMux = {
            self.local_to_io([4, 5, 6, 7, 28, 29][io_idx as usize])
        }
        fn local_to_in_clk_en(&self, io_idx: u8) -> LocalToIOMux = {
            self.local_to_io(30 + io_idx)
        }
        fn local_to_async_ctrl(&self, io_idx: u8) -> LocalToIOMux = {
            self.local_to_io(8 + io_idx)
        }
        fn local_to_sync_ctrl(&self, io_idx: u8) -> LocalToIOMux = {
            self.local_to_io([14, 15, 24, 25, 26, 27][io_idx as usize])
        }

        fn clock_mux(&self, idx: u8) -> IOClockMux {
            LeftRightIOClockMux(idx)
        }

        fn out_mux(&self, io_idx: u8, out_idx: u8) -> super::logic::OMUX {
            LeftRightIOOutMux(io_idx * 2 + out_idx)
        }

        fn in_data_delay(&self, io_idx: u8) -> 3 bits in u8 {
            LeftRightIOInDaDelay(io_idx)
        }
        fn in_reg_delay(&self, io_idx: u8) -> 3 bits in u8 {
            LeftRightIOInRegDelay(io_idx)
        }
        fn out_delay(&self, io_idx: u8) -> bool {
            LeftRightOutDelay(io_idx)
        }

        fn out_use_reg(&self, io_idx: u8) -> bool {
            LeftRightOutReg(io_idx)
        }
        fn oe_use_reg(&self, io_idx: u8) -> bool {
            LeftRightOEReg(io_idx)
        }

        fn in_powerup_state(&self, io_idx: u8) -> bool {
            LeftRightInPU(io_idx)
        }
        fn out_powerup_state(&self, io_idx: u8) -> bool {
            LeftRightOutPU(io_idx)
        }
        fn oe_powerup_state(&self, io_idx: u8) -> bool {
            LeftRightOEPU(io_idx)
        }

        fn in_async_mode(&self, io_idx: u8) -> RegCtrlMode {
            LeftRightInAMode(io_idx)
        }
        fn out_async_mode(&self, io_idx: u8) -> RegCtrlMode {
            LeftRightOutAMode(io_idx)
        }
        fn oe_async_mode(&self, io_idx: u8) -> RegCtrlMode {
            LeftRightOEAMode(io_idx)
        }

        fn in_sync_mode(&self, io_idx: u8) -> RegCtrlMode {
            LeftRightInSMode(io_idx)
        }
        fn out_sync_mode(&self, io_idx: u8) -> RegCtrlMode {
            LeftRightOutSMode(io_idx)
        }
        fn oe_sync_mode(&self, io_idx: u8) -> RegCtrlMode {
            LeftRightOESMode(io_idx)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const _ENSURE_DYN_SAFE_0: Option<&dyn IOTileCommon> = None;
    const _ENSURE_DYN_SAFE_1: Option<&dyn IOTileCommonMut> = None;
}
