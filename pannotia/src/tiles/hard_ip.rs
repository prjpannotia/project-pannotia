//! Interface to external hard IP blocks
//!
//! Hard IP tiles exist at the edges of the array, similar to I/O tiles.
//! However, IP tiles do not need two stages of "global" → "local" → IP
//! and instead directly picks wires from the interconnect to send to the IP.
//!
//! Each wire can come from either general-purpose interconnect or global interconnect,
//! and they all have a programmable invert bit.
//!
//! ## Top hard IP cells
//!
//! Top-edge hard IP interfaces have up to 12 wires to the IP and 12 wires from the IP.
//!
//! Each wire going _to_ the IP can chose from _any_ of the `T4Y` wires entering the tile
//! (or from one of the global clock lines).
//!
//! Each wire coming _from_ the IP can drive up 2 possible outputs, the "corresponding" one
//! or the "neighboring" one. For example, wire `0` from the IP can drive either `T4Y` wire `0` or `1`.
//! Likewise, wire `1` from the IP can also drive either `T4Y` wire `0` or `1`.
//! IP wires `2` and `3` can drive `T4Y` wires `2` or `3`, and so on.
//! Because you usually want to be able to use _all_ the wires, this effectively gives
//! two _useful_ choices of "go straight" or "swap pair" for each group of 2 wires.
//!
//! ## Left/right hard IP cells
//!
//! Top-edge hard IP interfaces have up to 20 wires to the IP and 20 wires from the IP.
//!
//! For the first 12 of these wires, each of them can chose from _any_ of the
//! `T4X` wires entering the tile (or from one of the global clock lines).
//! For the remaining 8 wires, each of them can chose from _any_ of the
//! `T1` neighbor wires entering the tile (or from one of the global clock lines).
//!
//! The first 12 output wires drive onto `T4X` outputs, either "straight" or "swapped"
//! (just like in top hard IP cells). The remaining 8 output wires drive onto `T1` neighbor wires.
//! Because 8 is half of 16, each of these wires drives _2_ `T1` wires
//! (in order, so output `12` drives `T1` wires `0` and `1`, `13` drives `2` and `3`, etc).

use std::fmt::Display;

use super::*;

/// A mux with 13 choices and an optional invert
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Mux13Inv {
    VCC,
    GND,
    I { invert: bool, i: u8 },
}
impl Default for Mux13Inv {
    fn default() -> Self {
        Self::VCC
    }
}
impl Display for Mux13Inv {
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
impl bitmux::BitstreamField for Mux13Inv {
    fn get(b: impl bitmux::BitGetter) -> Self {
        let bits = b.get_bits::<9>();
        let invert = bits & 0b1_0000_0000 != 0;
        bitmux::twohot!(3, 4, match bits & 0b1111_1111 {
            #bits => Self::I { invert, i: #val },
            0b1000_0000 => Self::I { invert, i: 12 },
            0 if invert => Self::GND,
            0 if !invert => Self::VCC,
            _ => panic!("invalid Mux13Inv {bits:09b}"),
        })
    }
    fn set(&self, mut b: impl bitmux::BitSetter) {
        let mut bits = bitmux::twohot!(3, 4, match self {
            Self::I { i: #val, .. } => #bits,
            Self::I { i: 12, .. } => 0b1000_0000,
            Self::GND => 0b1_0000_0000,
            Self::VCC => 0,
            _ => panic!("invalid Mux13Inv {}", self),
        });
        if let Self::I { invert: true, .. } = self {
            bits |= 0b1_0000_0000;
        }
        b.set_bits::<9>(bits);
    }
}

/// A mux with 17 choices and an optional invert
///
/// This has a generic parameter to control if the sense of the "invert" bit
/// should be flipped relative to the "default" (where 0 = do not invert)
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Mux17InvGeneric<const IS_INVERTED: bool> {
    VCC,
    GND,
    I { invert: bool, i: u8 },
}
impl<const IS_INVERTED: bool> Default for Mux17InvGeneric<IS_INVERTED> {
    fn default() -> Self {
        Self::VCC
    }
}
impl<const IS_INVERTED: bool> Display for Mux17InvGeneric<IS_INVERTED> {
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
impl<const IS_INVERTED: bool> bitmux::BitstreamField for Mux17InvGeneric<IS_INVERTED> {
    fn get(b: impl bitmux::BitGetter) -> Self {
        let bits = b.get_bits::<10>();
        let invert = (bits & 0b10_0000_0000 != 0) ^ IS_INVERTED;
        bitmux::twohot!(4, 4, match bits & 0b1_1111_1111 {
            #bits => Self::I { invert, i: #val },
            0b1_0000_0000 => Self::I { invert, i: 16 },
            0 if invert => Self::GND,
            0 if !invert => Self::VCC,
            _ => panic!("invalid Mux17Inv {bits:010b}"),
        })
    }
    fn set(&self, mut b: impl bitmux::BitSetter) {
        let mut bits = bitmux::twohot!(4, 4, match self {
            Self::I { i: #val, .. } => #bits,
            Self::I { i: 16, .. } => 0b1_0000_0000,
            Self::GND => 0b10_0000_0000,
            Self::VCC => 0,
            _ => panic!("invalid Mux17Inv {}", self),
        });
        if let Self::I { invert: true, .. } = self {
            bits |= 0b10_0000_0000;
        }
        if IS_INVERTED {
            bits ^= 0b10_0000_0000;
        }
        b.set_bits::<10>(bits);
    }
}

/// A mux with 17 choices and an optional invert
pub type Mux17Inv = Mux17InvGeneric<false>;

struct TopIPToExtMux(u8);
impl FieldPositionCalculator for TopIPToExtMux {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 12, "BBMUX index out of range");

        // The 12 BBMUXes group into 2 columns of 6,
        // where the second column has its bits mirrored horizontally.
        let is_second_col = self.0 >= 6;
        let inst_within_col = self.0 % 6;

        // This is the "baseline" shape
        let (xbase, mut y) = bitmux::bittable!(
            (#x, 54 + inst_within_col as u32 * 2 + #y),
            0   2   4   6   .   .   .,
            1   3   5   7   .   .   8,
        )[biti];

        // Within each column of 6, there is a gap of 2 rows between the top 3 and bottom 3.
        if inst_within_col >= 3 {
            y += 2;
        }

        // The second column is all the way over here
        let x = if !is_second_col { xbase } else { 29 - xbase };

        TileRelativeBitPos { y, x }
    }
}

struct TopIPFromExtMux(u8);
impl FieldPositionCalculator for TopIPFromExtMux {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 12, "output mux index out of range");

        // The 12 output muxes group into 2 columns of 6
        let is_second_col = self.0 >= 6;
        let inst_within_col = self.0 % 6;

        // This is the "baseline" location
        let mut y = 54 + inst_within_col as u32 * 2;

        // Within each column of 6, there is a gap of 2 rows between the top 3 and bottom 3.
        if inst_within_col >= 3 {
            y += 2;
        }

        // The location of each column is as follows
        let x = if !is_second_col { 30 } else { 32 };

        TileRelativeBitPos { y, x }
    }
}

struct TopIPGlobal2Local(u8);
impl FieldPositionCalculator for TopIPGlobal2Local {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 12, "GlobalToLocalMux index out of range");

        // The 12 SeamMUXes group into 2 columns of 6,
        // where the second column has its bits mirrored horizontally.
        let is_second_col = self.0 >= 6;
        let inst_within_col = self.0 % 6;

        // This is the "baseline" shape
        let mut y = 54 + inst_within_col as u32 * 2;

        // Within each column of 6, there is a gap of 2 rows between the top 3 and bottom 3.
        if inst_within_col >= 3 {
            y += 2;
        }

        // The location of each column is as follows
        let x = if !is_second_col {
            9 + biti as u32
        } else {
            20 - biti as u32
        };

        TileRelativeBitPos { y, x }
    }
}

make_tile_ref! {
    TopIPTileRef = TileType::TopIP
}

magic_tile_impl_gen! {
    impl TopIPTileRef {
        pub fn to_ip(&self, idx: u8) -> Mux13Inv {
            TopIPToExtMux(idx)
        }

        pub fn from_ip(&self, idx: u8) -> Mux2 {
            TopIPFromExtMux(idx)
        }

        pub fn global_to_local(&self, idx: u8) -> GlobalToLocalMux {
            TopIPGlobal2Local(idx)
        }
    }
}

struct LeftRightIPToExtMux13(u8);
impl FieldPositionCalculator for LeftRightIPToExtMux13 {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 12, "BBMUX index out of range");

        // This is the "baseline" shape
        let (x, mut y) = bitmux::bittable!(
            (9 + #x, 12 + self.0 as u32 * 2 + #y),
            8   6   4   2   0,
            .   7   5   3   1,
        )[biti];

        // These two exist after the mid-tile gap
        if self.0 >= 10 {
            y += 4;
        }

        TileRelativeBitPos { y, x }
    }
}

struct LeftRightIPToExtMux17(u8);
impl FieldPositionCalculator for LeftRightIPToExtMux17 {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 8, "BBMUX index out of range");

        // This is the "baseline" shape
        let (x, ybase) = bitmux::bittable!(
            (9 + #x, #y),
            9   7   6   3   1,
            .   .   .   .   .,
            4   8   5   2   0,
        )[biti];

        // Even and odd instances alternate being mirrored vertically
        let y = if self.0 % 2 == 0 {
            40 + (self.0 as u32 / 2) * 6 + ybase
        } else {
            45 + (self.0 as u32 / 2) * 6 - ybase
        };

        TileRelativeBitPos { y, x }
    }
}

struct LeftRightIPFromExtMux(u8);
impl FieldPositionCalculator for LeftRightIPFromExtMux {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 12, "output mux index out of range");
        TileRelativeBitPos {
            x: 12,
            y: self.0 as u32,
        }
    }
}

struct LeftRightIPGlobal2Local(u8);
impl FieldPositionCalculator for LeftRightIPGlobal2Local {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 20, "GlobalToLocalMux index out of range");

        // This is the "baseline" shape
        let x = 19 - biti as u32;

        let y = if self.0 <= 9 {
            // These 10 are in a regular pattern
            12 + self.0 as u32 * 2
        } else {
            // These are scattered around
            [36, 38, 40, 45, 46, 51, 52, 57, 58, 63][self.0 as usize - 10]
        };

        TileRelativeBitPos { y, x }
    }
}

make_tile_ref! {
    LeftRightIPTileRef = TileType::LeftRightIP
}

magic_tile_impl_gen! {
    impl LeftRightIPTileRef {
        pub fn to_ip_13(&self, idx: u8) -> Mux13Inv {
            LeftRightIPToExtMux13(idx)
        }
        pub fn to_ip_17(&self, idx: u8) -> Mux17Inv {
            LeftRightIPToExtMux17(idx)
        }

        pub fn from_ip(&self, idx: u8) -> Mux2 {
            LeftRightIPFromExtMux(idx)
        }

        pub fn global_to_local(&self, idx: u8) -> GlobalToLocalMux {
            LeftRightIPGlobal2Local(idx)
        }
    }
}
