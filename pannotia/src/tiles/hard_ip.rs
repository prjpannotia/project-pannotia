//! Interface to external hard IP blocks

use std::borrow::{Borrow, BorrowMut};
use std::fmt::Display;

use super::*;

use bitmux::{BitGetter, BitSetter, BitstreamField};

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
        Self::from_bits(b.get_bits::<9>())
    }
    fn set(&self, mut b: impl bitmux::BitSetter) {
        b.set_bits::<9>(self.to_bits());
    }
}
impl Mux13Inv {
    pub(crate) fn from_bits(bits: u32) -> Self {
        let invert = bits & 0b1_0000_0000 != 0;
        bitmux::twohot!(3, 4, match bits & 0b1111_1111 {
            #bits => Self::I { invert, i: #val },
            0b1000_0000 => Self::I { invert, i: 12 },
            0 if invert => Self::GND,
            0 if !invert => Self::VCC,
            _ => panic!("invalid Mux13Inv {bits:03b}"),
        })
    }

    pub(crate) fn to_bits(self) -> u32 {
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
        bits
    }
}

/// A mux with 17 choices and an optional invert
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Mux17Inv {
    VCC,
    GND,
    I { invert: bool, i: u8 },
}
impl Default for Mux17Inv {
    fn default() -> Self {
        Self::VCC
    }
}
impl Display for Mux17Inv {
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
impl bitmux::BitstreamField for Mux17Inv {
    fn get(b: impl bitmux::BitGetter) -> Self {
        Self::from_bits(b.get_bits::<10>())
    }
    fn set(&self, mut b: impl bitmux::BitSetter) {
        b.set_bits::<10>(self.to_bits());
    }
}
impl Mux17Inv {
    pub(crate) fn from_bits(bits: u32) -> Self {
        let invert = bits & 0b10_0000_0000 != 0;
        bitmux::twohot!(4, 4, match bits & 0b1_1111_1111 {
            #bits => Self::I { invert, i: #val },
            0b1_0000_0000 => Self::I { invert, i: 16 },
            0 if invert => Self::GND,
            0 if !invert => Self::VCC,
            _ => panic!("invalid Mux17Inv {bits:03b}"),
        })
    }

    pub(crate) fn to_bits(self) -> u32 {
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
        bits
    }
}

pub(crate) struct TopIPToExtMux(u8);
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

pub(crate) struct TopIPFromExtMux(u8);
impl FieldPositionCalculator for TopIPFromExtMux {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 12, "OMUX index out of range");

        // The 12 OMUXes group into 2 columns of 6
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

pub(crate) struct TopIPGlobal2Local(u8);
impl FieldPositionCalculator for TopIPGlobal2Local {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 12, "GlobalToLocalMux index out of range");

        // The 12 BBMUXes group into 2 columns of 6,
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

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct TopIPTileRef<D: DebugTracer, Ref: Borrow<Bitstream<D>>> {
    pub(super) r: Ref,
    pub(super) p: TilePos,
    pub(super) _d: PhantomData<D>,
}
impl<D: DebugTracer, Ref: Borrow<Bitstream<D>>> TileRefTrait<D, Ref> for TopIPTileRef<D, Ref> {
    fn tile_type(&self) -> TileType {
        TileType::TopIP
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

impl<D: DebugTracer, Ref: Borrow<Bitstream<D>>> TopIPTileRef<D, Ref> {
    pub fn to_ip(&self, idx: u8) -> Mux13Inv {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: TopIPToExtMux(idx),
            _d: PhantomData,
        };
        Mux13Inv::get(ref_)
    }

    pub fn from_ip(&self, idx: u8) -> Mux2 {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: TopIPFromExtMux(idx),
            _d: PhantomData,
        };
        Mux2::from(ref_.get_bit(0))
    }

    pub fn global_to_local(&self, idx: u8) -> GlobalToLocalMux {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: TopIPGlobal2Local(idx),
            _d: PhantomData,
        };
        GlobalToLocalMux::get(ref_)
    }
}
impl<D: DebugTracer, Ref: BorrowMut<Bitstream<D>>> TopIPTileRef<D, Ref> {
    pub fn set_to_ip(&mut self, idx: u8, val: Mux13Inv) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: TopIPToExtMux(idx),
            _d: PhantomData,
        };
        val.set(ref_);
    }

    pub fn set_from_ip(&mut self, idx: u8, val: Mux2) {
        let mut ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: TopIPFromExtMux(idx),
            _d: PhantomData,
        };
        ref_.set_bit(0, val.into());
    }

    pub fn set_global_to_local(&mut self, idx: u8, val: GlobalToLocalMux) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: TopIPGlobal2Local(idx),
            _d: PhantomData,
        };
        val.set(ref_);
    }
}
