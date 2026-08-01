//! Clocking resources (PLL, clock distribution)

use std::borrow::{Borrow, BorrowMut};

use super::hard_ip::Mux13Inv;
use super::*;

use bitmux::{BitGetter, BitSetter, BitstreamField};

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

    pub fn fb_phase_coarse(&self) -> u8 {
        let bitstream = self.r.borrow();
        bitstream.get_aux_array_bits(1, 1, 23..23 + 8) as u8
    }
    pub fn fb_phase_fine(&self) -> u8 {
        let bitstream = self.r.borrow();
        bitstream.get_aux_array_bits(1, 1, 20..20 + 3) as u8
    }
    pub fn out_phase_coarse(&self, idx: u8) -> u8 {
        assert!(idx < 5, "invalid output index");
        let bitstream = self.r.borrow();
        let biti = 35 + 13 * idx as usize;
        bitstream.get_aux_array_bits(1, 1, biti..biti + 8) as u8
    }
    pub fn out_phase_fine(&self, idx: u8) -> u8 {
        assert!(idx < 5, "invalid output index");
        let bitstream = self.r.borrow();
        let biti = 31 + 13 * idx as usize;
        bitstream.get_aux_array_bits(1, 1, biti..biti + 3) as u8
    }

    pub fn out_enable(&self, idx: u8) -> bool {
        assert!(idx < 5, "invalid output index");
        let bitstream = self.r.borrow();
        let biti = 34 + 13 * idx as usize;
        bitstream.get_aux_array_bit(1, 1, biti)
    }
    pub fn out_cascade(&self, idx: u8) -> bool {
        assert!(idx > 0 && idx < 5, "invalid output index");
        let bitstream = self.r.borrow();
        let biti = 30 + 13 * idx as usize;
        bitstream.get_aux_array_bit(1, 1, biti)
    }

    fn _clkdiv_lo_time(&self, idx: u8) -> u8 {
        let bitstream = self.r.borrow();
        let biti = 95 + 18 * idx as usize;
        bitstream.get_aux_array_bits(1, 1, biti..biti + 8) as u8
    }
    fn _clkdiv_hi_time(&self, idx: u8) -> u8 {
        let bitstream = self.r.borrow();
        let biti = 95 + 9 + 18 * idx as usize;
        bitstream.get_aux_array_bits(1, 1, biti..biti + 8) as u8
    }
    fn _clkdiv_trim(&self, idx: u8) -> bool {
        let bitstream = self.r.borrow();
        let biti = 95 + 8 + 18 * idx as usize;
        bitstream.get_aux_array_bit(1, 1, biti)
    }
    fn _clkdiv_bypass(&self, idx: u8) -> bool {
        let bitstream = self.r.borrow();
        let biti = 95 + 17 + 18 * idx as usize;
        bitstream.get_aux_array_bit(1, 1, biti)
    }
    pub fn out_div_lo_time(&self, idx: u8) -> u8 {
        assert!(idx < 5, "invalid output index");
        self._clkdiv_lo_time(4 - idx)
    }
    pub fn out_div_hi_time(&self, idx: u8) -> u8 {
        assert!(idx < 5, "invalid output index");
        self._clkdiv_hi_time(4 - idx)
    }
    pub fn out_div_duty_cycle_adjust(&self, idx: u8) -> bool {
        assert!(idx < 5, "invalid output index");
        self._clkdiv_trim(4 - idx)
    }
    pub fn out_div_bypass(&self, idx: u8) -> bool {
        assert!(idx < 5, "invalid output index");
        self._clkdiv_bypass(4 - idx)
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

    pub fn set_fb_phase_coarse(&mut self, val: u8) {
        let bitstream = self.r.borrow_mut();
        bitstream.set_aux_array_bits(1, 1, 23..23 + 8, val as u32);
    }
    pub fn set_fb_phase_fine(&mut self, val: u8) {
        assert!(val & !0b111 == 0, "invalid setting");
        let bitstream = self.r.borrow_mut();
        bitstream.set_aux_array_bits(1, 1, 20..20 + 3, val as u32);
    }
    pub fn set_out_phase_coarse(&mut self, idx: u8, val: u8) {
        assert!(idx < 5, "invalid output index");
        let bitstream = self.r.borrow_mut();
        let biti = 35 + 13 * idx as usize;
        bitstream.set_aux_array_bits(1, 1, biti..biti + 8, val as u32);
    }
    pub fn set_out_phase_fine(&mut self, idx: u8, val: u8) {
        assert!(idx < 5, "invalid output index");
        assert!(val & !0b111 == 0, "invalid setting");
        let bitstream = self.r.borrow_mut();
        let biti = 31 + 13 * idx as usize;
        bitstream.set_aux_array_bits(1, 1, biti..biti + 3, val as u32);
    }

    pub fn set_out_enable(&mut self, idx: u8, val: bool) {
        assert!(idx < 5, "invalid output index");
        let bitstream = self.r.borrow_mut();
        let biti = 34 + 13 * idx as usize;
        bitstream.set_aux_array_bit(1, 1, biti, val);
    }
    pub fn set_out_cascade(&mut self, idx: u8, val: bool) {
        assert!(idx > 0 && idx < 5, "invalid output index");
        let bitstream = self.r.borrow_mut();
        let biti = 30 + 13 * idx as usize;
        bitstream.set_aux_array_bit(1, 1, biti, val);
    }

    fn _set_clkdiv_lo_time(&mut self, idx: u8, val: u8) {
        let bitstream = self.r.borrow_mut();
        let biti = 95 + 18 * idx as usize;
        bitstream.set_aux_array_bits(1, 1, biti..biti + 8, val as u32);
    }
    fn _set_clkdiv_hi_time(&mut self, idx: u8, val: u8) {
        let bitstream = self.r.borrow_mut();
        let biti = 95 + 9 + 18 * idx as usize;
        bitstream.set_aux_array_bits(1, 1, biti..biti + 8, val as u32);
    }
    fn _set_clkdiv_trim(&mut self, idx: u8, val: bool) {
        let bitstream = self.r.borrow_mut();
        let biti = 95 + 8 + 18 * idx as usize;
        bitstream.set_aux_array_bit(1, 1, biti, val);
    }
    fn _set_clkdiv_bypass(&mut self, idx: u8, val: bool) {
        let bitstream = self.r.borrow_mut();
        let biti = 95 + 17 + 18 * idx as usize;
        bitstream.set_aux_array_bit(1, 1, biti, val);
    }
    pub fn set_out_div_lo_time(&mut self, idx: u8, val: u8) {
        assert!(idx < 5, "invalid output index");
        self._set_clkdiv_lo_time(4 - idx, val);
    }
    pub fn set_out_div_hi_time(&mut self, idx: u8, val: u8) {
        assert!(idx < 5, "invalid output index");
        self._set_clkdiv_hi_time(4 - idx, val);
    }
    pub fn set_out_div_duty_cycle_adjust(&mut self, idx: u8, val: bool) {
        assert!(idx < 5, "invalid output index");
        self._set_clkdiv_trim(4 - idx, val);
    }
    pub fn set_out_div_bypass(&mut self, idx: u8, val: bool) {
        assert!(idx < 5, "invalid output index");
        self._set_clkdiv_bypass(4 - idx, val);
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

struct GCLKSWEnable(u8);
impl FieldPositionCalculator for GCLKSWEnable {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 6, "IOMUX index out of range");

        let mut ybase = 17 + 6 * self.0 as u32;
        if self.0 >= 3 {
            // there is a gap in the middle
            ybase += 4;
        }

        bitmux::bittable!(
            TileRelativeBitPos { x: 6 + #x, y: ybase + #y },
            9	4	5	2	0,
            .	.	.	.	.,
            8	7	6	3	1,

        )[biti]
    }
}

struct GCLKSWGlobal2Local(u8);
impl FieldPositionCalculator for GCLKSWGlobal2Local {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 12, "GlobalToLocalMux index out of range");

        let y = [20, 23, 24, 27, 28, 31, 36, 39, 40, 43, 44, 47][self.0 as usize];

        let x = 12 + biti as u32;

        TileRelativeBitPos { y, x }
    }
}

struct GCLKSWClock2Fabric(u8);
impl FieldPositionCalculator for GCLKSWClock2Fabric {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 12, "output mux index out of range");

        let y = [20, 22, 44, 46][self.0 as usize];

        TileRelativeBitPos { y, x: 0 }
    }
}

struct GCLKSWClockEnReg(u8);
impl FieldPositionCalculator for GCLKSWClockEnReg {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 6, "clock index out of range");

        let y = [3, 9, 15, 21, 27, 43][self.0 as usize];

        TileRelativeBitPos { y, x: 0 }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct GCLKMux4(pub u8);
impl Default for GCLKMux4 {
    fn default() -> Self {
        Self(0)
    }
}
impl Display for GCLKMux4 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}
impl bitmux::BitstreamField for GCLKMux4 {
    fn get(b: impl bitmux::BitGetter) -> Self {
        Self(b.get_bits::<2>() as u8)
    }
    fn set(&self, mut b: impl bitmux::BitSetter) {
        assert!(self.0 < 4, "invalid GCLKMux4 {}", self);
        b.set_bits::<2>(self.0 as u32);
    }
}

struct GCLKSWMux(u8);
impl FieldPositionCalculator for GCLKSWMux {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 5, "clock index out of range");

        let y = 7 + 6 * self.0 as u32 - biti as u32;

        TileRelativeBitPos { y, x: 0 }
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

    pub fn clock_enable(&self, idx: u8) -> InvertedMux17Inv {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: GCLKSWEnable(idx),
            _d: PhantomData,
        };
        InvertedMux17Inv::get(ref_)
    }

    pub fn global_to_local(&self, idx: u8) -> GlobalToLocalMux {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: GCLKSWGlobal2Local(idx),
            _d: PhantomData,
        };
        GlobalToLocalMux::get(ref_)
    }

    pub fn clock_to_fabric(&self, idx: u8) -> Mux2 {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: GCLKSWClock2Fabric(idx),
            _d: PhantomData,
        };
        ref_.get_bit(0).into()
    }

    pub fn cen_is_registered(&self, idx: u8) -> bool {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: GCLKSWClockEnReg(idx),
            _d: PhantomData,
        };
        ref_.get_bit(0)
    }

    pub fn clock_dist_mux(&self, idx: u8) -> GCLKMux4 {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: GCLKSWMux(idx),
            _d: PhantomData,
        };
        GCLKMux4::get(ref_)
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

    pub fn set_clock_enable(&mut self, idx: u8, val: InvertedMux17Inv) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: GCLKSWEnable(idx),
            _d: PhantomData,
        };
        val.set(ref_);
    }

    pub fn set_global_to_local(&mut self, idx: u8, val: GlobalToLocalMux) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: GCLKSWGlobal2Local(idx),
            _d: PhantomData,
        };
        val.set(ref_);
    }

    pub fn set_clock_to_fabric(&mut self, idx: u8, val: Mux2) {
        let mut ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: GCLKSWClock2Fabric(idx),
            _d: PhantomData,
        };
        ref_.set_bit(0, val.into());
    }

    pub fn set_cen_is_registered(&mut self, idx: u8, val: bool) {
        let mut ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: GCLKSWClockEnReg(idx),
            _d: PhantomData,
        };
        ref_.set_bit(0, val);
    }

    pub fn set_clock_dist_mux(&mut self, idx: u8, val: GCLKMux4) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: GCLKSWMux(idx),
            _d: PhantomData,
        };
        val.set(ref_);
    }
}
