//! Routing-only tiles
//!
//! This tile behaves like a logic tile without any logic cells inside.
//! This means that there are no `IMUX` nor control signals.
//! Right-going neighbor wires instead select from the `RMUX` self-wires.

use std::borrow::{Borrow, BorrowMut};

use super::generic_routing::{GenericRoutingRefMutTrait, GenericRoutingRefTrait, RMUX, RMUXRef};

use super::*;

use bitmux::{BitGetter, BitSetter, BitstreamField};

/// Access to a routing-only tile
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct RoutingOnlyTileRef<D: DebugTracer, Ref: Borrow<Bitstream<D>>> {
    pub(super) r: Ref,
    pub(super) p: TilePos,
    pub(super) _d: PhantomData<D>,
}
impl<D: DebugTracer, Ref: Borrow<Bitstream<D>>> TileRefTrait<D, Ref>
    for RoutingOnlyTileRef<D, Ref>
{
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

struct OMUXRef(u8);
impl FieldPositionCalculator for OMUXRef {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 16, "output index out of range");
        TileRelativeBitPos {
            x: 15,
            y: self.0 as u32 * 4 + if self.0 >= 8 { 4 } else { 0 },
        }
    }
}

impl<D: DebugTracer, Ref: Borrow<Bitstream<D>>> RoutingOnlyTileRef<D, Ref> {
    pub fn global_to_local(&self, inp_idx: u8) -> GlobalToLocalMux {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: GlobalToLocalMuxRef {
                is_bram: false,
                i: inp_idx,
            },
            _d: PhantomData,
        };
        GlobalToLocalMux::get(ref_)
    }

    pub fn right_neighbor_output(&self, lc_idx: u8) -> Mux2 {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: OMUXRef(lc_idx),
            _d: PhantomData,
        };
        ref_.get_bit(0).into()
    }
}
impl<D: DebugTracer, Ref: BorrowMut<Bitstream<D>>> RoutingOnlyTileRef<D, Ref> {
    pub fn set_global_to_local(&mut self, inp_idx: u8, val: GlobalToLocalMux) {
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: GlobalToLocalMuxRef {
                is_bram: false,
                i: inp_idx,
            },
            _d: PhantomData,
        };
        val.set(ref_);
    }

    pub fn set_right_neighbor_output(&mut self, lc_idx: u8, val: Mux2) {
        let mut ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: OMUXRef(lc_idx),
            _d: PhantomData,
        };
        ref_.set_bit(0, val.into());
    }
}

impl<D: DebugTracer, Ref: Borrow<Bitstream<D>>> GenericRoutingRefTrait
    for RoutingOnlyTileRef<D, Ref>
{
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
    for RoutingOnlyTileRef<D, Ref>
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
