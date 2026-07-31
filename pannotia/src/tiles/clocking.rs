//! Clocking resources (PLL, clock distribution)

use std::borrow::{Borrow, BorrowMut};

use super::hard_ip::Mux13Inv;
use super::*;

use bitmux::BitstreamField;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct PLLTileRef<D: DebugTracer, Ref: Borrow<Bitstream<D>>> {
    pub(super) r: Ref,
    pub(super) p: TilePos,
    pub(super) _d: PhantomData<D>,
}
impl<D: DebugTracer, Ref: Borrow<Bitstream<D>>> TileRefTrait<D, Ref> for PLLTileRef<D, Ref> {
    fn tile_type(&self) -> TileType {
        TileType::PLL
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

struct PLLWireTo(u8);
impl FieldPositionCalculator for PLLWireTo {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 11, "BBMUX index out of range");

        let mut ybase = 20 + 2 * self.0 as u32;
        if self.0 >= 6 {
            // there is a gap in the middle
            ybase += 4;
        }

        bitmux::bittable!(
            TileRelativeBitPos { x: 9 + #x, y: ybase + #y },
            8   6   4   2   0,
            .   7   5   3   1,
        )[biti]
    }
}

struct PLLGlobal2Local(u8);
impl FieldPositionCalculator for PLLGlobal2Local {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 11, "GlobalToLocalMux index out of range");

        let mut y = 20 + 2 * self.0 as u32;
        if self.0 >= 6 {
            // there is a gap in the middle
            y += 4;
        }

        let x = 19 - biti as u32;

        TileRelativeBitPos { y, x }
    }
}

impl<D: DebugTracer, Ref: Borrow<Bitstream<D>>> PLLTileRef<D, Ref> {
    pub fn to_pll(&self, idx: u8) -> Mux13Inv {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: PLLWireTo(idx),
            _d: PhantomData,
        };
        Mux13Inv::get(ref_)
    }

    pub fn global_to_local(&self, idx: u8) -> GlobalToLocalMux {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: PLLGlobal2Local(idx),
            _d: PhantomData,
        };
        GlobalToLocalMux::get(ref_)
    }
}
impl<D: DebugTracer, Ref: BorrowMut<Bitstream<D>>> PLLTileRef<D, Ref> {
    pub fn set_to_pll(&mut self, idx: u8, val: Mux13Inv) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: PLLWireTo(idx),
            _d: PhantomData,
        };
        val.set(ref_);
    }

    pub fn set_global_to_local(&mut self, idx: u8, val: GlobalToLocalMux) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: PLLGlobal2Local(idx),
            _d: PhantomData,
        };
        val.set(ref_);
    }
}

/// The clock enable signals in a GCLKSW tile default to an opposite sense invert bit
pub type InvertedMux17Inv = super::hard_ip::Mux17InvGeneric<true>;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct GCLKSWTileRef<D: DebugTracer, Ref: Borrow<Bitstream<D>>> {
    pub(super) r: Ref,
    pub(super) p: TilePos,
    pub(super) _d: PhantomData<D>,
}
impl<D: DebugTracer, Ref: Borrow<Bitstream<D>>> TileRefTrait<D, Ref> for GCLKSWTileRef<D, Ref> {
    fn tile_type(&self) -> TileType {
        TileType::GCLKSW
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

struct GCLKSWFabricToClock(u8);
impl FieldPositionCalculator for GCLKSWFabricToClock {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 6, "IOMUX index out of range");

        let mut ybase = 14 + 6 * self.0 as u32;
        if self.0 >= 3 {
            // there is a gap in the middle
            ybase += 4;
        }

        bitmux::bittable!(
            TileRelativeBitPos { x: 6 + #x, y: ybase + #y },
            8	7	6	3	1,
            .	.	.	.	.,
            9	4	5	2	0,

        )[biti]
    }
}

impl<D: DebugTracer, Ref: Borrow<Bitstream<D>>> GCLKSWTileRef<D, Ref> {
    pub fn fabric_to_clock(&self, idx: u8) -> super::hard_ip::Mux17Inv {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: GCLKSWFabricToClock(idx),
            _d: PhantomData,
        };
        super::hard_ip::Mux17Inv::get(ref_)
    }
}
impl<D: DebugTracer, Ref: BorrowMut<Bitstream<D>>> GCLKSWTileRef<D, Ref> {
    pub fn set_fabric_to_clock(&mut self, idx: u8, val: super::hard_ip::Mux17Inv) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: GCLKSWFabricToClock(idx),
            _d: PhantomData,
        };
        val.set(ref_);
    }
}
