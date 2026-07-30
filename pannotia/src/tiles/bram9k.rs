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

    pub fn addr_a(&self, bit: u8) -> IMUX {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: IMUXRef {
                is_bram: true,
                i: 12 - bit,
            },
            _d: PhantomData,
        };
        IMUX::get(ref_)
    }
    pub fn addr_b(&self, bit: u8) -> IMUX {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: IMUXRef {
                is_bram: true,
                i: 51 + bit,
            },
            _d: PhantomData,
        };
        IMUX::get(ref_)
    }
    pub fn data_in_a(&self, bit: u8) -> IMUX {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: IMUXRef {
                is_bram: true,
                i: 30 - bit,
            },
            _d: PhantomData,
        };
        IMUX::get(ref_)
    }
    pub fn data_in_b(&self, bit: u8) -> IMUX {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: IMUXRef {
                is_bram: true,
                i: 33 + bit,
            },
            _d: PhantomData,
        };
        IMUX::get(ref_)
    }
    pub fn imux_xtra(&self, idx: u8) -> IMUX {
        assert!(idx < 2, "invalid extra IMUX index");
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: IMUXRef {
                is_bram: true,
                i: 31 + idx,
            },
            _d: PhantomData,
        };
        IMUX::get(ref_)
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

    pub fn set_addr_a(&mut self, bit: u8, val: IMUX) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: IMUXRef {
                is_bram: true,
                i: 12 - bit,
            },
            _d: PhantomData,
        };
        val.set(ref_);
    }
    pub fn set_addr_b(&mut self, bit: u8, val: IMUX) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: IMUXRef {
                is_bram: true,
                i: 51 + bit,
            },
            _d: PhantomData,
        };
        val.set(ref_);
    }
    pub fn set_data_in_a(&mut self, bit: u8, val: IMUX) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: IMUXRef {
                is_bram: true,
                i: 30 - bit,
            },
            _d: PhantomData,
        };
        val.set(ref_);
    }
    pub fn set_data_in_b(&mut self, bit: u8, val: IMUX) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: IMUXRef {
                is_bram: true,
                i: 33 + bit,
            },
            _d: PhantomData,
        };
        val.set(ref_);
    }
    pub fn set_imux_xtra(&mut self, idx: u8, val: IMUX) {
        assert!(idx < 2, "invalid extra IMUX index");
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: IMUXRef {
                is_bram: true,
                i: 31 + idx,
            },
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
