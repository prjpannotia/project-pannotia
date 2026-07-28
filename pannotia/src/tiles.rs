use std::borrow::{Borrow, BorrowMut};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct TilePos {
    pub y: u32,
    pub x: u32,
}

#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum TileType {
    Logic,
}

pub trait TileRefTrait {
    fn tile_type(&self) -> Option<TileType>;
}

pub struct TileRef<Ref: Borrow<crate::container::Bitstream>> {
    r: Ref,
    p: TilePos,
}
impl<Ref: Borrow<crate::container::Bitstream>> TileRefTrait for TileRef<Ref> {
    fn tile_type(&self) -> Option<TileType> {
        let family = self.r.borrow().family();
        match family {
            crate::chips::Family::AGRV2K => todo!(),
        }
    }
}
impl<Ref: Borrow<crate::container::Bitstream>> TileRef<Ref> {
    pub(crate) fn new(r: Ref, p: TilePos) -> Self {
        Self { r, p }
    }

    pub fn as_logic_tile(self) -> LogicTileRef<Ref> {
        LogicTileRef {
            r: self.r,
            p: self.p,
        }
    }
}

pub struct LogicTileRef<Ref: Borrow<crate::container::Bitstream>> {
    r: Ref,
    p: TilePos,
}
impl<Ref: Borrow<crate::container::Bitstream>> LogicTileRef<Ref> {
    pub fn lut(&self) -> u16 {
        let bits = self.r.borrow();
        bits.get_logic_array_bit(123, 456);
        12345
    }
}
impl<Ref: BorrowMut<crate::container::Bitstream>> LogicTileRef<Ref> {
    pub fn set_lut(&mut self, val: u16) {
        let bits = self.r.borrow_mut();
        bits.set_logic_array_bit(123, 456, val == 12345);
    }
}
