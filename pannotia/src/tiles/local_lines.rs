//! Inputs from routing to logic or BRAM
//!
//! This defines common mux bit related stuff,
//! but it defers the actual accessor functions to the logic/BRAM tiles.
//! This is because the _logical_ purpose of the inputs is very different.

use std::fmt::Display;

use super::*;

pub(crate) struct IMUXRef {
    pub(crate) is_bram: bool,
    pub(crate) i: u8,
}
impl FieldPositionCalculator for IMUXRef {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.i < 64, "IMUX index out of range");
        // There are 64 IMUXes per tile, which we _logically_ group as 16x 4 in logic tiles,
        // but which are _physically_ grouped as 2 columns of 32 (with a gap in the middle).
        // The second column also has its bits mirrored relative to the first column.
        let imux_row = (self.i / 2) as u32;
        let imux_col = self.i % 2;

        let mut y_offs = imux_row * 2;
        // These tiles all have a 4-row gap in the middle for global routing bits
        if self.i >= 32 {
            y_offs += 4;
        }

        // We can perform a basic lookup of the 6x2 bit block now
        let (y, xbase) = bitmux::bittable!(
            (y_offs + #y, #x),
            0   2   4   6   8   9,
            1   3   5   7   11  10,
        )[biti];

        // Deal with inverting the second column
        let mut x = match imux_col {
            0 => xbase,
            1 => 11 - xbase,
            _ => unreachable!(),
        } + 15; // + 15 to skip over RMUX

        // For a BRAM tile, this is scooted right by 1
        if self.is_bram {
            x += 1;
        }

        TileRelativeBitPos { y, x }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum IMUX {
    None,
    I(u8),
}
impl Default for IMUX {
    fn default() -> Self {
        Self::None
    }
}
impl Display for IMUX {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IMUX::None => write!(f, "<unset>"),
            IMUX::I(i) => write!(f, "#{i}"),
        }
    }
}
impl IMUX {
    pub(crate) fn from_bits(bits: u32) -> Self {
        bitmux::twohot!(3, 9, match bits {
            #bits => IMUX::I(#val),
            0 => IMUX::None,
            _ => panic!("invalid IMUX {bits:010b}"),
        })
    }

    pub(crate) fn to_bits(self) -> u32 {
        bitmux::twohot!(3, 9, match self {
            IMUX::I(#val) => #bits,
            IMUX::None => 0,
            _ => panic!("invalid IMUX {}", self),
        })
    }
}
