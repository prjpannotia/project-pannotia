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

        // Now we can look up the main 4x2 bloc
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
