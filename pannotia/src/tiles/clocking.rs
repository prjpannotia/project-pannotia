//! Clocking resources (PLL, clock distribution)

use super::hard_ip::Mux13Inv;
use super::*;

make_tile_ref! {
    /// Access to a PLL tile
    PLLTileRef = TileType::PLL
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

magic_tile_impl_gen! {
    impl PLLTileRef {
        pub fn to_pll(&self, idx: u8) -> Mux13Inv {
            PLLWireTo(idx)
        }

        pub fn global_to_local(&self, idx: u8) -> GlobalToLocalMux {
            PLLGlobal2Local(idx)
        }

        // get specific divider's parameters

        pub fn out_div_lo_time(&self, idx: u8) -> u8 = {
            assert!(idx < 5, "invalid output index");
            self.clkdiv_lo_time(4 - idx)
        }
        pub fn out_div_hi_time(&self, idx: u8) -> u8 = {
            assert!(idx < 5, "invalid output index");
            self.clkdiv_hi_time(4 - idx)
        }
        pub fn out_div_duty_cycle_adjust(&self, idx: u8) -> bool = {
            assert!(idx < 5, "invalid output index");
            self.clkdiv_trim(4 - idx)
        }
        pub fn out_div_bypass(&self, idx: u8) -> bool = {
            assert!(idx < 5, "invalid output index");
            self.clkdiv_bypass(4 - idx)
        }

        pub fn fb_div_lo_time(&self) -> u8 = {
            self.clkdiv_lo_time(5)
        }
        pub fn fb_div_hi_time(&self) -> u8 = {
            self.clkdiv_hi_time(5)
        }
        pub fn fb_div_duty_cycle_adjust(&self) -> bool = {
            self.clkdiv_trim(5)
        }
        pub fn fb_div_bypass(&self) -> bool = {
            self.clkdiv_bypass(5)
        }

        pub fn in_div_lo_time(&self) -> u8 = {
            self.clkdiv_lo_time(6)
        }
        pub fn in_div_hi_time(&self) -> u8 = {
            self.clkdiv_hi_time(6)
        }
        pub fn in_div_duty_cycle_adjust(&self) -> bool = {
            self.clkdiv_trim(6)
        }
        pub fn in_div_bypass(&self) -> bool = {
            self.clkdiv_bypass(6)
        }
    }
}

macro_rules! _magic_pll_impl_gen_one_item {
    // as an integer
    (read $self:ident $nbits:literal bits in $r:ty $body:block) => {
        let bit_start = $body;
        let bits_range = bit_start..bit_start + $nbits;
        let bitstream = $self.r.borrow();
        bitstream.get_aux_array_bits(1, 1, bits_range) as $r
    };
    (write $self:ident $val:ident $nbits:literal bits in $r:ty $body:block) => {
        assert!(
            $val & !(((1u64 << $nbits) - 1) as $r) == 0,
            "invalid setting"
        );
        let bit_start = $body;
        let bits_range = bit_start..bit_start + $nbits;
        let bitstream = $self.r.borrow_mut();
        bitstream.set_aux_array_bits(1, 1, bits_range, $val as u32);
    };

    // as a single bit
    (read $self:ident bool $body:block) => {
        let biti = $body;
        let bitstream = $self.r.borrow();
        bitstream.get_aux_array_bit(1, 1, biti)
    };
    (write $self:ident $val:ident bool $body:block) => {
        let biti = $body;
        let bitstream = $self.r.borrow_mut();
        bitstream.set_aux_array_bit(1, 1, biti, $val);
    };

    (read $self:ident invert bool $body:block) => {
        let biti = $body;
        let bitstream = $self.r.borrow();
        !bitstream.get_aux_array_bit(1, 1, biti)
    };
    (write $self:ident $val:ident invert bool $body:block) => {
        let biti = $body;
        let bitstream = $self.r.borrow_mut();
        bitstream.set_aux_array_bit(1, 1, biti, !$val);
    };
}

macro_rules! _magic_pll_impl_gen_items {
    (read $($(#[$attr:meta])* $v:vis fn $f:ident(&$self:ident $($args:tt)* ) -> $($nbits:literal bits in)? $(!$invert:ident)? $($r:ident)? $body:block)* ) => {
        $(
            #[doc = concat!("Read the field `", stringify!($f), "`\n\n")]
            $(#[$attr])*
            $v fn $f(&$self $($args)* ) -> $($invert)? $($r)? {
                _magic_pll_impl_gen_one_item! { read $self $($nbits bits in)? $(invert $invert)? $($r)? $body }
            }
        )*
    };
    (write $($(#[$attr:meta])* $v:vis fn $f:ident(&$self:ident $($args:tt)* ) -> $($nbits:literal bits in)? $(!$invert:ident)? $($r:ident)? $body:block)* ) => {
        mident::mident! {
            $(
                #[doc = concat!("Write the field `", stringify!($f), "`\n\n")]
                $(#[$attr])*
                $v fn #concat(set_ $f)(&mut $self $($args)*, val: $($invert)? $($r)? ) {
                    _magic_pll_impl_gen_one_item! { write $self val $($nbits bits in)? $(invert $invert)? $($r)? $body }
                }
            )*
        }
    };
}

macro_rules! magic_pll_impl {
    (impl $impl_on:ident { $($inside:tt)* }) => {
        impl<D: DebugTracer, Ref: std::borrow::Borrow<Bitstream<D>>> $impl_on<D, Ref> {
            _magic_pll_impl_gen_items!{ read $($inside)* }
        }
        impl<D: DebugTracer, Ref: std::borrow::BorrowMut<Bitstream<D>>> $impl_on<D, Ref> {
            _magic_pll_impl_gen_items!{ write $($inside)* }
        }
    };
}
magic_pll_impl! {
    impl PLLTileRef {
        pub fn fb_phase_coarse(&self) -> 8 bits in u8 {
            23
        }
        pub fn fb_phase_fine(&self) -> 3 bits in u8 {
            20
        }
        pub fn out_phase_coarse(&self, idx: u8) -> 8 bits in u8 {
            assert!(idx < 5, "invalid output index");
            35 + 13 * idx as usize
        }
        pub fn out_phase_fine(&self, idx: u8) -> 3 bits in u8 {
            assert!(idx < 5, "invalid output index");
            31 + 13 * idx as usize
        }

        pub fn out_enable(&self, idx: u8) -> bool {
            assert!(idx < 5, "invalid output index");
            34 + 13 * idx as usize
        }
        pub fn out_cascade(&self, idx: u8) -> bool {
            assert!(idx > 0 && idx < 5, "invalid output index");
            30 + 13 * idx as usize
        }

        // generic clk div functions, where index is magically assumed
        fn clkdiv_lo_time(&self, idx: u8) -> 8 bits in u8 {
            95 + 18 * idx as usize
        }
        fn clkdiv_hi_time(&self, idx: u8) -> 8 bits in u8 {
            95 + 9 + 18 * idx as usize
        }
        fn clkdiv_trim(&self, idx: u8) -> bool {
            5 + 8 + 18 * idx as usize
        }
        fn clkdiv_bypass(&self, idx: u8) -> bool {
            5 + 17 + 18 * idx as usize
        }

        pub fn vco_div2(&self) -> bool {
            229
        }

        pub fn analog_icp(&self) -> 3 bits in u8 {
            221
        }
        pub fn analog_rlpf(&self) -> 2 bits in u8 {
            230
        }
        pub fn analog_rref(&self) -> 2 bits in u8 {
            232
        }
        pub fn analog_rvi(&self) -> 2 bits in u8 {
            234
        }
        pub fn analog_ivco(&self) -> 3 bits in u8 {
            236
        }

        // FIXME: This is totally undocumented
        pub fn reg_ctrl(&self) -> 2 bits in u8 {
            0
        }
        pub fn enabled(&self) -> bool {
            2
        }
        pub fn clock_feedback_mux(&self) -> 2 bits in u8 {
            3
        }
        pub fn feedback_delay(&self) -> 3 bits in u8 {
            5
        }
        pub fn clock_mux_0(&self) -> 3 bits in u8 {
            8
        }
        // AGRV2K doesn't have this
        pub fn clock_mux_1(&self) -> 3 bits in u8 {
            11
        }
        pub fn gclk_mux(&self) -> 3 bits in u8 {
            14
        }
        pub fn use_internal_fb(&self) -> bool {
            17
        }
        pub fn enable_dedicated_out_n(&self) -> !bool {
            18
        }
        pub fn enable_dedicated_out_p(&self) -> !bool {
            19
        }
    }
}

/// The clock enable signals in a GCLKSW tile default to an opposite sense invert bit
pub type InvertedMux17Inv = super::hard_ip::Mux17InvGeneric<true>;

make_tile_ref! {
    /// Access to a GCLKSW tile
    GCLKSWTileRef = TileType::GCLKSW
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

magic_tile_impl_gen! {
    impl GCLKSWTileRef {
        pub fn fabric_to_clock(&self, idx: u8) -> super::hard_ip::Mux17Inv {
            GCLKSWFabricToClock(idx)
        }

        pub fn clock_enable(&self, idx: u8) -> InvertedMux17Inv {
            GCLKSWEnable(idx)
        }

        pub fn global_to_local(&self, idx: u8) -> GlobalToLocalMux {
            GCLKSWGlobal2Local(idx)
        }

        pub fn clock_to_fabric(&self, idx: u8) -> Mux2 {
            GCLKSWClock2Fabric(idx)
        }

        pub fn cen_is_registered(&self, idx: u8) -> bool {
            GCLKSWClockEnReg(idx)
        }

        pub fn clock_dist_mux(&self, idx: u8) -> GCLKMux4 {
            GCLKSWMux(idx)
        }
    }
}
