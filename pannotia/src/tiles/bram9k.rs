//! Block RAM tiles (9216 bits)

use std::borrow::{Borrow, BorrowMut};

use super::generic_routing::{GenericRoutingRefMutTrait, GenericRoutingRefTrait, RMUX, RMUXRef};
use super::local_lines::{CtrlMux, CtrlMuxRef, IMUX, IMUXRef};

use super::*;

use bitmux::{BitGetter, BitSetter, BitstreamField};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct BRAMTileRef<D: DebugTracer, Ref: Borrow<Bitstream<D>>> {
    pub(super) r: Ref,
    pub(super) p: TilePos,
    pub(super) _d: PhantomData<D>,
}
impl<D: DebugTracer, Ref: Borrow<Bitstream<D>>> TileRefTrait<D, Ref> for BRAMTileRef<D, Ref> {
    fn tile_type(&self) -> TileType {
        TileType::RoutingOnly
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

struct InitVal {}
impl FieldPositionCalculator for InitVal {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        // each row contains 144 bits, and there are 64 rows
        let mut y = biti as u32 / 144;
        // the bottom half has to jump over the mid-tile gap
        if y >= 32 {
            y += 4;
        }

        // 144 = 18 * 8, and the bits within a row are interleaved
        // 8 instances of "every 18th bit" comes first, starting from bit 0
        // (i.e. 0, 18, 36, ...)
        // this repeats 18 times to fill out all the bits
        let bitpos_col_before_rearrange = biti as u32 % 144;
        let group_of_8 = bitpos_col_before_rearrange % 18;
        let idx_within_8 = bitpos_col_before_rearrange / 18;
        let x = 36 + group_of_8 * 8 + idx_within_8;

        TileRelativeBitPos { y, x }
    }
}

struct TMUXRef(u8);
impl FieldPositionCalculator for TMUXRef {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 16, "TMUX index out of range");

        // the TMUX vertical position pattern repeats in groups of 4
        let group_of_4 = self.0 / 4;
        let within_group_4 = self.0 % 4;
        let y_base =
            [0, 16, 32 + 4, 48 + 4][group_of_4 as usize] + [2, 6, 8, 12][within_group_4 as usize];

        // Now we can look up the main 4x2 block
        bitmux::bittable!(
            TileRelativeBitPos { x: 28 + #x, y: y_base + #y },
            0   2   4   5,
            1   3   7   6,
        )[biti]
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum TMUX {
    None,
    I(u8),
}
impl Default for TMUX {
    fn default() -> Self {
        Self::None
    }
}
impl Display for TMUX {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "<unset>"),
            Self::I(i) => write!(f, "#{i}"),
        }
    }
}
impl bitmux::BitstreamField for TMUX {
    fn get(b: impl bitmux::BitGetter) -> Self {
        Self::from_bits(b.get_bits::<8>())
    }
    fn set(&self, mut b: impl bitmux::BitSetter) {
        b.set_bits::<8>(self.to_bits());
    }
}
impl TMUX {
    pub(crate) fn from_bits(bits: u32) -> Self {
        bitmux::twohot!(3, 5, match bits {
            #bits => Self::I(#val),
            0 => Self::None,
            _ => panic!("invalid TMUX {bits:08b}"),
        })
    }

    pub(crate) fn to_bits(self) -> u32 {
        bitmux::twohot!(3, 5, match self {
            Self::I(#val) => #bits,
            Self::None => 0,
            _ => panic!("invalid TMUX {}", self),
        })
    }
}

struct KMUXRef(u8);
impl FieldPositionCalculator for KMUXRef {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 16, "KMUX index out of range");

        // the KMUX vertical position pattern repeats in groups of 4,
        // slotting "in between" the TMUX.
        // except, for some reason, number 0 is at the very bottom!
        let remapped_idx = match self.0 {
            0 => 15,
            _ => self.0 - 1,
        };

        let group_of_4 = remapped_idx / 4;
        let within_group_4 = remapped_idx % 4;
        let y_base =
            [0, 16, 32 + 4, 48 + 4][group_of_4 as usize] + [0, 4, 10, 14][within_group_4 as usize];

        // Now we can look up the main 4x2 block ( + 1 extra invert bit)
        bitmux::bittable!(
            TileRelativeBitPos { x: 28 + #x, y: y_base + #y },
            .   .   .   .   5   4   2   0,
            .   8   .   .   6   7   3   1,
        )[biti]
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum KMUX {
    VCC,
    GND,
    I { invert: bool, i: u8 },
}
impl Default for KMUX {
    fn default() -> Self {
        Self::VCC
    }
}
impl Display for KMUX {
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
impl bitmux::BitstreamField for KMUX {
    fn get(b: impl bitmux::BitGetter) -> Self {
        Self::from_bits(b.get_bits::<9>())
    }
    fn set(&self, mut b: impl bitmux::BitSetter) {
        b.set_bits::<9>(self.to_bits());
    }
}
impl KMUX {
    pub(crate) fn from_bits(bits: u32) -> Self {
        let invert = bits & 0b1_00000000 != 0;
        bitmux::twohot!(3, 5, match bits & 0b1111_1111 {
            #bits => Self::I { invert, i: #val },
            0 if invert => Self::GND,
            0 if !invert => Self::VCC,
            _ => panic!("invalid KMUX {bits:09b}"),
        })
    }

    pub(crate) fn to_bits(self) -> u32 {
        let mut bits = bitmux::twohot!(
            3,
            5,
            match self {
                Self::I { i: #val, .. } => #bits,
                Self::GND => 0b1_00000000,
                Self::VCC => 0,
                _ => panic!("invalid KMUX {}", self),
            }
        );
        if let Self::I { invert: true, .. } = self {
            bits |= 0b1_00000000;
        }
        bits
    }
}

struct TileClk(u8);
impl FieldPositionCalculator for TileClk {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        let y = match self.0 {
            0 => 32,
            1 => 35,
            _ => panic!("clock index out of range"),
        };
        bitmux::bittable!(
            TileRelativeBitPos { x: 32 + #x, y },
            2   1   0   3
        )[biti]
    }
}
struct TileClkEn(u8);
impl FieldPositionCalculator for TileClkEn {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        match self.0 {
            0 => bitmux::bittable!(
                TileRelativeBitPos { x: 32 + #x, y: 34 },
                2   1   0   3
            )[biti],
            1 => bitmux::bittable!(
                TileRelativeBitPos { x: 30 + #x, y: 34 + #y },
                1   2,
                0   3,
            )[biti],
            _ => panic!("clock enable index out of range"),
        }
    }
}
struct TileAsync(u8);
impl FieldPositionCalculator for TileAsync {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        match self.0 {
            0 => bitmux::bittable!(
                TileRelativeBitPos { x: 30 + #x, y: 32 + #y },
                0   3,
                1   2,
            )[biti],
            1 => bitmux::bittable!(
                TileRelativeBitPos { x: 32 + #x, y: 33 },
                2   1   0   3
            )[biti],
            _ => panic!("async index out of range"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[bitmux::bitenum]
pub enum PortWidth {
    X36 = "10000",
    X18 = "00000",
    X9 = "01000",
    X4 = "01100",
    X2 = "01110",
    X1 = "01111",
}
impl Default for PortWidth {
    fn default() -> Self {
        Self::X18
    }
}
impl Display for PortWidth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PortWidth::X36 => write!(f, "36"),
            PortWidth::X18 => write!(f, "18"),
            PortWidth::X9 => write!(f, "9"),
            PortWidth::X4 => write!(f, "4"),
            PortWidth::X2 => write!(f, "2"),
            PortWidth::X1 => write!(f, "1"),
        }
    }
}
struct PortAWidth {}
impl FieldPositionCalculator for PortAWidth {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        bitmux::bittable!(
            TileRelativeBitPos { x: 32 + #x, y: #y },
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   3,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   2   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   1,
            .   .   .   .,
            .   .   .   0,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   4   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
        )[biti]
    }
}
struct PortBWidth {}
impl FieldPositionCalculator for PortBWidth {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        bitmux::bittable!(
            TileRelativeBitPos { x: 32 + #x, y: #y },
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   1   0,
            .   .   .   .,
            .   .   .   4,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   2   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   3   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
            .   .   .   .,
        )[biti]
    }
}

struct OutRegA {}
impl FieldPositionCalculator for OutRegA {
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        TileRelativeBitPos { x: 34, y: 29 }
    }
}
struct OutRegB {}
impl FieldPositionCalculator for OutRegB {
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        TileRelativeBitPos { x: 35, y: 39 }
    }
}
struct WriteThruA {}
impl FieldPositionCalculator for WriteThruA {
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        TileRelativeBitPos { x: 35, y: 9 }
    }
}
struct WriteThruB {}
impl FieldPositionCalculator for WriteThruB {
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        TileRelativeBitPos { x: 34, y: 59 }
    }
}
struct UseRstInA {}
impl FieldPositionCalculator for UseRstInA {
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        TileRelativeBitPos { x: 35, y: 55 }
    }
}

struct UseRstInB {}
impl FieldPositionCalculator for UseRstInB {
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        TileRelativeBitPos { x: 35, y: 61 }
    }
}
struct UseRstOutA {}
impl FieldPositionCalculator for UseRstOutA {
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        TileRelativeBitPos { x: 35, y: 59 }
    }
}
struct UseRstOutB {}
impl FieldPositionCalculator for UseRstOutB {
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        TileRelativeBitPos { x: 35, y: 65 }
    }
}

struct UseClkEnInA {}
impl FieldPositionCalculator for UseClkEnInA {
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        TileRelativeBitPos { x: 35, y: 3 }
    }
}
struct UseClkEnInB {}
impl FieldPositionCalculator for UseClkEnInB {
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        TileRelativeBitPos { x: 35, y: 19 }
    }
}
struct UseClkEnOutA {}
impl FieldPositionCalculator for UseClkEnOutA {
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        TileRelativeBitPos { x: 35, y: 7 }
    }
}
struct UseClkEnOutB {}
impl FieldPositionCalculator for UseClkEnOutB {
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        TileRelativeBitPos { x: 35, y: 29 }
    }
}

struct RsenDly {}
impl FieldPositionCalculator for RsenDly {
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        [
            TileRelativeBitPos { x: 34, y: 13 },
            TileRelativeBitPos { x: 34, y: 23 },
        ][biti]
    }
}
struct DlyTime {}
impl FieldPositionCalculator for DlyTime {
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        [
            TileRelativeBitPos { x: 34, y: 25 },
            TileRelativeBitPos { x: 34, y: 39 },
        ][biti]
    }
}

impl<D: DebugTracer, Ref: Borrow<Bitstream<D>>> BRAMTileRef<D, Ref> {
    pub fn global_to_local(&self, inp_idx: u8) -> GlobalToLocalMux {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: GlobalToLocalMuxRef {
                is_bram: true,
                i: inp_idx,
            },
            _d: PhantomData,
        };
        GlobalToLocalMux::get(ref_)
    }

    pub fn control_signal_preselect(&self, inp_idx: u8) -> CtrlMux {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: CtrlMuxRef {
                is_bram: true,
                i: inp_idx,
            },
            _d: PhantomData,
        };
        CtrlMux::get(ref_)
    }

    pub fn imux(&self, i: u8) -> IMUX {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: IMUXRef { is_bram: true, i },
            _d: PhantomData,
        };
        IMUX::get(ref_)
    }
    pub fn addr_a(&self, bit: u8) -> IMUX {
        assert!(bit < 13, "invalid address bit index");
        self.imux(12 - bit)
    }
    pub fn addr_b(&self, bit: u8) -> IMUX {
        assert!(bit < 13, "invalid address bit index");
        self.imux(51 + bit)
    }
    pub fn data_in_a(&self, bit: u8) -> IMUX {
        assert!(bit < 18, "invalid data bit index");
        self.imux(30 - bit)
    }
    pub fn data_in_b(&self, bit: u8) -> IMUX {
        assert!(bit < 18, "invalid data bit index");
        self.imux(33 + bit)
    }
    pub fn imux_xtra(&self, idx: u8) -> IMUX {
        assert!(idx < 2, "invalid extra IMUX index");
        self.imux(31 + idx)
    }

    pub fn init_data(&self, sink: &mut impl BitSink) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: InitVal {},
            _d: PhantomData,
        };
        for i in 0..9216 {
            sink.set(i, ref_.get_bit(i));
        }
    }

    pub fn tmux(&self, i: u8) -> TMUX {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: TMUXRef(i),
            _d: PhantomData,
        };
        TMUX::get(ref_)
    }
    pub fn kmux(&self, i: u8) -> KMUX {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: KMUXRef(i),
            _d: PhantomData,
        };
        KMUX::get(ref_)
    }
    pub fn read_en_a(&self) -> KMUX {
        self.kmux(6)
    }
    pub fn read_en_b(&self) -> KMUX {
        self.kmux(7)
    }
    pub fn write_en_a(&self) -> KMUX {
        self.kmux(3)
    }
    pub fn write_en_b(&self) -> KMUX {
        self.kmux(0)
    }
    pub fn addr_stall_a(&self) -> KMUX {
        self.kmux(4)
    }
    pub fn addr_stall_b(&self) -> KMUX {
        self.kmux(5)
    }
    pub fn byte_en_a(&self, bit: u8) -> KMUX {
        assert!(bit < 2, "invalid byte enable bit index");
        self.kmux(1 + bit)
    }
    pub fn byte_en_b(&self, bit: u8) -> KMUX {
        assert!(bit < 2, "invalid byte enable bit index");
        self.kmux(8 + bit)
    }

    pub fn clock_mux(&self, clk_idx: u8) -> Mux3Inv {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: TileClk(clk_idx),
            _d: PhantomData,
        };
        Mux3Inv::get(ref_)
    }
    pub fn clock_en_mux(&self, clk_idx: u8) -> Mux3Inv {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: TileClkEn(clk_idx),
            _d: PhantomData,
        };
        Mux3Inv::get(ref_)
    }
    pub fn async_mux(&self, clk_idx: u8) -> Mux3Inv {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: TileAsync(clk_idx),
            _d: PhantomData,
        };
        Mux3Inv::get(ref_)
    }

    pub fn width_a(&self) -> PortWidth {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: PortAWidth {},
            _d: PhantomData,
        };
        PortWidth::get(ref_)
    }
    pub fn width_b(&self) -> PortWidth {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: PortBWidth {},
            _d: PhantomData,
        };
        PortWidth::get(ref_)
    }

    pub fn use_output_register_a(&self) -> bool {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: OutRegA {},
            _d: PhantomData,
        };
        ref_.get_bit(0)
    }
    pub fn use_output_register_b(&self) -> bool {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: OutRegB {},
            _d: PhantomData,
        };
        ref_.get_bit(0)
    }

    pub fn use_rst_in_a(&self) -> bool {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: UseRstInA {},
            _d: PhantomData,
        };
        ref_.get_bit(0)
    }
    pub fn use_rst_in_b(&self) -> bool {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: UseRstInB {},
            _d: PhantomData,
        };
        ref_.get_bit(0)
    }
    pub fn use_rst_out_a(&self) -> bool {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: UseRstOutA {},
            _d: PhantomData,
        };
        ref_.get_bit(0)
    }
    pub fn use_rst_out_b(&self) -> bool {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: UseRstOutB {},
            _d: PhantomData,
        };
        ref_.get_bit(0)
    }

    pub fn use_clk_en_in_a(&self) -> bool {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: UseClkEnInA {},
            _d: PhantomData,
        };
        ref_.get_bit(0)
    }
    pub fn use_clk_en_in_b(&self) -> bool {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: UseClkEnInB {},
            _d: PhantomData,
        };
        ref_.get_bit(0)
    }
    pub fn use_clk_en_out_a(&self) -> bool {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: UseClkEnOutA {},
            _d: PhantomData,
        };
        ref_.get_bit(0)
    }
    pub fn use_clk_en_out_b(&self) -> bool {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: UseClkEnOutB {},
            _d: PhantomData,
        };
        ref_.get_bit(0)
    }

    pub fn write_thru_a(&self) -> bool {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: WriteThruA {},
            _d: PhantomData,
        };
        ref_.get_bit(0)
    }
    pub fn write_thru_b(&self) -> bool {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: WriteThruB {},
            _d: PhantomData,
        };
        ref_.get_bit(0)
    }

    pub fn rsen_delay(&self) -> u8 {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: RsenDly {},
            _d: PhantomData,
        };
        ref_.get_bits::<2>() as u8
    }
    pub fn delay_time(&self) -> u8 {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: DlyTime {},
            _d: PhantomData,
        };
        ref_.get_bits::<2>() as u8
    }
}
impl<D: DebugTracer, Ref: BorrowMut<Bitstream<D>>> BRAMTileRef<D, Ref> {
    pub fn set_global_to_local(&mut self, inp_idx: u8, val: GlobalToLocalMux) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: GlobalToLocalMuxRef {
                is_bram: true,
                i: inp_idx,
            },
            _d: PhantomData,
        };
        val.set(ref_);
    }

    pub fn set_control_signal_preselect(&mut self, inp_idx: u8, val: CtrlMux) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: CtrlMuxRef {
                is_bram: true,
                i: inp_idx,
            },
            _d: PhantomData,
        };
        val.set(ref_);
    }

    pub fn set_imux(&mut self, i: u8, val: IMUX) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: IMUXRef { is_bram: true, i },
            _d: PhantomData,
        };
        val.set(ref_);
    }
    pub fn set_addr_a(&mut self, bit: u8, val: IMUX) {
        assert!(bit < 13, "invalid address bit index");
        self.set_imux(12 - bit, val);
    }
    pub fn set_addr_b(&mut self, bit: u8, val: IMUX) {
        assert!(bit < 13, "invalid address bit index");
        self.set_imux(51 + bit, val);
    }
    pub fn set_data_in_a(&mut self, bit: u8, val: IMUX) {
        assert!(bit < 18, "invalid data bit index");
        self.set_imux(30 - bit, val);
    }
    pub fn set_data_in_b(&mut self, bit: u8, val: IMUX) {
        assert!(bit < 18, "invalid data bit index");
        self.set_imux(33 + bit, val);
    }
    pub fn set_imux_xtra(&mut self, idx: u8, val: IMUX) {
        assert!(idx < 2, "invalid extra IMUX index");
        self.set_imux(31 + idx, val);
    }

    pub fn set_init_data(&mut self, source: &impl BitSource) {
        let mut ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: InitVal {},
            _d: PhantomData,
        };
        for i in 0..9216 {
            ref_.set_bit(i, source.get(i));
        }
    }

    pub fn set_tmux(&mut self, i: u8, val: TMUX) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: TMUXRef(i),
            _d: PhantomData,
        };
        val.set(ref_);
    }
    pub fn set_kmux(&mut self, i: u8, val: KMUX) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: KMUXRef(i),
            _d: PhantomData,
        };
        val.set(ref_);
    }
    pub fn set_read_en_a(&mut self, val: KMUX) {
        self.set_kmux(6, val);
    }
    pub fn set_read_en_b(&mut self, val: KMUX) {
        self.set_kmux(7, val);
    }
    pub fn set_write_en_a(&mut self, val: KMUX) {
        self.set_kmux(3, val);
    }
    pub fn set_write_en_b(&mut self, val: KMUX) {
        self.set_kmux(0, val);
    }
    pub fn set_addr_stall_a(&mut self, val: KMUX) {
        self.set_kmux(4, val);
    }
    pub fn set_addr_stall_b(&mut self, val: KMUX) {
        self.set_kmux(5, val);
    }
    pub fn set_byte_en_a(&mut self, bit: u8, val: KMUX) {
        assert!(bit < 2, "invalid byte enable bit index");
        self.set_kmux(1 + bit, val)
    }
    pub fn set_byte_en_b(&mut self, bit: u8, val: KMUX) {
        assert!(bit < 2, "invalid byte enable bit index");
        self.set_kmux(8 + bit, val)
    }

    pub fn set_clock_mux(&mut self, clk_idx: u8, val: Mux3Inv) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: TileClk(clk_idx),
            _d: PhantomData,
        };
        val.set(ref_);
    }
    pub fn set_clock_en_mux(&mut self, clk_idx: u8, val: Mux3Inv) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: TileClkEn(clk_idx),
            _d: PhantomData,
        };
        val.set(ref_);
    }
    pub fn set_async_mux(&mut self, clk_idx: u8, val: Mux3Inv) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: TileAsync(clk_idx),
            _d: PhantomData,
        };
        val.set(ref_);
    }

    pub fn set_width_a(&mut self, val: PortWidth) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: PortAWidth {},
            _d: PhantomData,
        };
        val.set(ref_);
    }
    pub fn set_width_b(&mut self, val: PortWidth) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: PortBWidth {},
            _d: PhantomData,
        };
        val.set(ref_);
    }

    pub fn set_use_output_register_a(&mut self, val: bool) {
        let mut ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: OutRegA {},
            _d: PhantomData,
        };
        ref_.set_bit(0, val);
    }
    pub fn set_use_output_register_b(&mut self, val: bool) {
        let mut ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: OutRegB {},
            _d: PhantomData,
        };
        ref_.set_bit(0, val);
    }

    pub fn set_use_rst_in_a(&mut self, val: bool) {
        let mut ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: UseRstInA {},
            _d: PhantomData,
        };
        ref_.set_bit(0, val);
    }
    pub fn set_use_rst_in_b(&mut self, val: bool) {
        let mut ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: UseRstInB {},
            _d: PhantomData,
        };
        ref_.set_bit(0, val);
    }
    pub fn set_use_rst_out_a(&mut self, val: bool) {
        let mut ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: UseRstOutA {},
            _d: PhantomData,
        };
        ref_.set_bit(0, val);
    }
    pub fn set_use_rst_out_b(&mut self, val: bool) {
        let mut ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: UseRstOutB {},
            _d: PhantomData,
        };
        ref_.set_bit(0, val);
    }

    pub fn set_use_clk_en_in_a(&mut self, val: bool) {
        let mut ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: UseClkEnInA {},
            _d: PhantomData,
        };
        ref_.set_bit(0, val);
    }
    pub fn set_use_clk_en_in_b(&mut self, val: bool) {
        let mut ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: UseClkEnInB {},
            _d: PhantomData,
        };
        ref_.set_bit(0, val);
    }
    pub fn set_use_clk_en_out_a(&mut self, val: bool) {
        let mut ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: UseClkEnOutA {},
            _d: PhantomData,
        };
        ref_.set_bit(0, val);
    }
    pub fn set_use_clk_en_out_b(&mut self, val: bool) {
        let mut ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: UseClkEnOutB {},
            _d: PhantomData,
        };
        ref_.set_bit(0, val);
    }

    pub fn set_write_thru_a(&mut self, val: bool) {
        let mut ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: WriteThruA {},
            _d: PhantomData,
        };
        ref_.set_bit(0, val);
    }
    pub fn set_write_thru_b(&mut self, val: bool) {
        let mut ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: WriteThruB {},
            _d: PhantomData,
        };
        ref_.set_bit(0, val);
    }

    pub fn set_rsen_delay(&mut self, val: u8) {
        let mut ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: RsenDly {},
            _d: PhantomData,
        };
        assert!(val & !0b11 == 0, "invalid setting");
        ref_.set_bits::<2>(val as u32);
    }
    pub fn set_delay_time(&mut self, val: u8) {
        let mut ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: DlyTime {},
            _d: PhantomData,
        };
        assert!(val & !0b11 == 0, "invalid setting");
        ref_.set_bits::<2>(val as u32);
    }
}

impl<D: DebugTracer, Ref: Borrow<Bitstream<D>>> GenericRoutingRefTrait for BRAMTileRef<D, Ref> {
    fn rmux(&self, rmux_idx: u8) -> RMUX {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: RMUXRef {
                is_bram: false,
                i: rmux_idx,
            },
            _d: PhantomData,
        };
        RMUX::get(ref_)
    }
}
impl<D: DebugTracer, Ref: BorrowMut<Bitstream<D>>> GenericRoutingRefMutTrait
    for BRAMTileRef<D, Ref>
{
    fn set_rmux(&mut self, rmux_idx: u8, val: RMUX) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: RMUXRef {
                is_bram: false,
                i: rmux_idx,
            },
            _d: PhantomData,
        };
        val.set(ref_);
    }
}
