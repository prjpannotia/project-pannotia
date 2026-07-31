//! I/O tiles

use std::borrow::{Borrow, BorrowMut};
use std::fmt::Display;

use super::*;

use bitmux::{BitGetter, BitSetter, BitstreamField};

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
        Self::from_bits(b.get_bits::<6>())
    }
    fn set(&self, mut b: impl bitmux::BitSetter) {
        b.set_bits::<6>(self.to_bits());
    }
}
impl TopBottomIOLocalMux {
    fn from_bits(bits: u32) -> Self {
        bitmux::twohot!(2, 3, match bits {
            #bits => Self::I(#val),
            0b1_00_000 => Self::I(6),
            0 => Self::None,
            _ => panic!("invalid TopBottomIOLocalMux {bits:06b}"),
        })
    }

    fn to_bits(self) -> u32 {
        bitmux::twohot!(2, 3, match self {
            Self::I(#val) => #bits,
            Self::I(6) => 0b1_00_000,
            Self::None => 0,
            _ => panic!("invalid TopBottomIOLocalMux {}", self),
        })
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
        Self::from_bits(b.get_bits::<7>())
    }
    fn set(&self, mut b: impl bitmux::BitSetter) {
        b.set_bits::<7>(self.to_bits());
    }
}
impl LeftRightIOLocalMux {
    fn from_bits(bits: u32) -> Self {
        bitmux::twohot!(2, 4, match bits {
            #bits => Self::I(#val),
            0b1_00_0000 => Self::I(8),
            0 => Self::None,
            _ => panic!("invalid LeftRightIOLocalMux {bits:07b}"),
        })
    }

    fn to_bits(self) -> u32 {
        bitmux::twohot!(2, 4, match self {
            Self::I(#val) => #bits,
            Self::I(8) => 0b1_00_0000,
            Self::None => 0,
            _ => panic!("invalid LeftRightIOLocalMux {}", self),
        })
    }
}

struct TopBottomIOLocalLine {
    is_bottom: bool,
    i: u8,
}
impl FieldPositionCalculator for TopBottomIOLocalLine {
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
        Self::from_bits(b.get_bits::<6>())
    }
    fn set(&self, mut b: impl bitmux::BitSetter) {
        b.set_bits::<6>(self.to_bits());
    }
}
impl IOLocalToClockMux {
    fn from_bits(bits: u32) -> Self {
        bitmux::twohot!(2, 4, match bits {
            #bits => Self::I(#val),
            0 => Self::None,
            _ => panic!("invalid IOLocalToClockMux {bits:06b}"),
        })
    }

    fn to_bits(self) -> u32 {
        bitmux::twohot!(2, 4, match self {
            Self::I(#val) => #bits,
            Self::None => 0,
            _ => panic!("invalid IOLocalToClockMux {}", self),
        })
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
        Self::from_bits(b.get_bits::<7>())
    }
    fn set(&self, mut b: impl bitmux::BitSetter) {
        b.set_bits::<7>(self.to_bits());
    }
}
impl LocalToIOMux {
    fn from_bits(bits: u32) -> Self {
        let invert = bits & 0b1_00_0000 != 0;
        bitmux::twohot!(2, 4, match bits & 0b11_1111 {
            #bits => Self::I { invert, i: #val },
            0 if invert => Self::GND,
            0 if !invert => Self::VCC,
            _ => panic!("invalid LocalToIOMux {bits:07b}"),
        })
    }

    fn to_bits(self) -> u32 {
        let mut bits = bitmux::twohot!(2, 4, match self {
                Self::I { i: #val, .. } => #bits,
                Self::GND => 0b1_00_0000,
                Self::VCC => 0,
            _ => panic!("invalid LocalToIOMux {}", self),
        });
        if let Self::I { invert: true, .. } = self {
            bits |= 0b1_00_0000;
        }
        bits
    }
}

struct TopBottomIOLocal2IO {
    is_bottom: bool,
    i: u8,
}
impl FieldPositionCalculator for TopBottomIOLocal2IO {
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
            .	4	2	0,
            6	5	3	1,
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
            .	4	2	0,
            6	5	3	1,
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
pub struct TopBottomIOTileRef<D: DebugTracer, Ref: Borrow<Bitstream<D>>> {
    pub(super) r: Ref,
    pub(super) p: TilePos,
    pub(super) _d: PhantomData<D>,
}
impl<D: DebugTracer, Ref: Borrow<Bitstream<D>>> TileRefTrait<D, Ref>
    for TopBottomIOTileRef<D, Ref>
{
    fn tile_type(&self) -> TileType {
        TileType::TopBottomIO
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

impl<D: DebugTracer, Ref: Borrow<Bitstream<D>>> TopBottomIOTileRef<D, Ref> {
    pub fn local_line(&self, idx: u8) -> TopBottomIOLocalMux {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: TopBottomIOLocalLine {
                is_bottom: self.p.y == 0,
                i: idx,
            },
            _d: PhantomData,
        };
        TopBottomIOLocalMux::get(ref_)
    }

    pub fn global_to_local(&self, idx: u8) -> GlobalToLocalMux {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: TopBottomIOGlobal2Local {
                is_bottom: self.p.y == 0,
                i: idx,
            },
            _d: PhantomData,
        };
        GlobalToLocalMux::get(ref_)
    }

    pub fn local_to_clock(&self, idx: u8) -> IOLocalToClockMux {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: TopBottomIOLocal2Clk {
                is_bottom: self.p.y == 0,
                i: idx,
            },
            _d: PhantomData,
        };
        IOLocalToClockMux::get(ref_)
    }

    pub fn local_to_io(&self, custom_idx: u8) -> LocalToIOMux {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: TopBottomIOLocal2IO {
                is_bottom: self.p.y == 0,
                i: custom_idx,
            },
            _d: PhantomData,
        };
        LocalToIOMux::get(ref_)
    }
}
impl<D: DebugTracer, Ref: BorrowMut<Bitstream<D>>> TopBottomIOTileRef<D, Ref> {
    pub fn set_local_line(&mut self, idx: u8, val: TopBottomIOLocalMux) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: TopBottomIOLocalLine {
                is_bottom: self.p.y == 0,
                i: idx,
            },
            _d: PhantomData,
        };
        val.set(ref_);
    }

    pub fn set_global_to_local(&mut self, idx: u8, val: GlobalToLocalMux) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: TopBottomIOGlobal2Local {
                is_bottom: self.p.y == 0,
                i: idx,
            },
            _d: PhantomData,
        };
        val.set(ref_);
    }

    pub fn set_local_to_clock(&mut self, idx: u8, val: IOLocalToClockMux) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: TopBottomIOLocal2Clk {
                is_bottom: self.p.y == 0,
                i: idx,
            },
            _d: PhantomData,
        };
        val.set(ref_);
    }

    pub fn set_local_to_io(&mut self, custom_idx: u8, val: LocalToIOMux) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: TopBottomIOLocal2IO {
                is_bottom: self.p.y == 0,
                i: custom_idx,
            },
            _d: PhantomData,
        };
        val.set(ref_);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct LeftRightIOTileRef<D: DebugTracer, Ref: Borrow<Bitstream<D>>> {
    pub(super) r: Ref,
    pub(super) p: TilePos,
    pub(super) _d: PhantomData<D>,
}
impl<D: DebugTracer, Ref: Borrow<Bitstream<D>>> TileRefTrait<D, Ref>
    for LeftRightIOTileRef<D, Ref>
{
    fn tile_type(&self) -> TileType {
        TileType::LeftRightIO
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

impl<D: DebugTracer, Ref: Borrow<Bitstream<D>>> LeftRightIOTileRef<D, Ref> {
    pub fn local_line(&self, idx: u8) -> LeftRightIOLocalMux {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: LeftRightIOLocalLine(idx),
            _d: PhantomData,
        };
        LeftRightIOLocalMux::get(ref_)
    }

    pub fn global_to_local(&self, idx: u8) -> GlobalToLocalMux {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: LeftRightIOGlobal2Local(idx),
            _d: PhantomData,
        };
        GlobalToLocalMux::get(ref_)
    }

    pub fn local_to_clock(&self, idx: u8) -> IOLocalToClockMux {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: LeftRightIOLocal2Clk(idx),
            _d: PhantomData,
        };
        IOLocalToClockMux::get(ref_)
    }

    pub fn local_to_io(&self, custom_idx: u8) -> LocalToIOMux {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: LeftRightIOLocal2IO(custom_idx),
            _d: PhantomData,
        };
        LocalToIOMux::get(ref_)
    }
}
impl<D: DebugTracer, Ref: BorrowMut<Bitstream<D>>> LeftRightIOTileRef<D, Ref> {
    pub fn set_local_line(&mut self, idx: u8, val: LeftRightIOLocalMux) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: LeftRightIOLocalLine(idx),
            _d: PhantomData,
        };
        val.set(ref_);
    }

    pub fn set_global_to_local(&mut self, idx: u8, val: GlobalToLocalMux) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: LeftRightIOGlobal2Local(idx),
            _d: PhantomData,
        };
        val.set(ref_);
    }

    pub fn set_local_to_clock(&mut self, idx: u8, val: IOLocalToClockMux) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: LeftRightIOLocal2Clk(idx),
            _d: PhantomData,
        };
        val.set(ref_);
    }

    pub fn set_local_to_io(&mut self, custom_idx: u8, val: LocalToIOMux) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: LeftRightIOLocal2IO(custom_idx),
            _d: PhantomData,
        };
        val.set(ref_);
    }
}
