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

impl<D: DebugTracer, Ref: Borrow<Bitstream<D>>> TopBottomIOTileRef<D, Ref> {}
impl<D: DebugTracer, Ref: BorrowMut<Bitstream<D>>> TopBottomIOTileRef<D, Ref> {}

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

impl<D: DebugTracer, Ref: Borrow<Bitstream<D>>> LeftRightIOTileRef<D, Ref> {}
impl<D: DebugTracer, Ref: BorrowMut<Bitstream<D>>> LeftRightIOTileRef<D, Ref> {}
