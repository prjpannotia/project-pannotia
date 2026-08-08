//! Imports a bunch of generally "useful" names

pub use crate::chips::Family;
pub use crate::container::{Bitstream, BitstreamContainerError};
pub use crate::coordinates::TilePos;
pub use crate::packages;
pub use crate::padring::{self, PadRingExt};
pub use crate::routedb::{self, AbsoluteRoutingWire, Direction, RoutingWire, WireType};
pub use crate::tiles::{
    self, TileRefTrait, TileType,
    generic_routing::{GenericRoutingRefMutTrait, GenericRoutingRefTrait},
    io::{IOTileCommon, IOTileCommonMut},
    logic::LUT,
};

/// Types that you most likely only need if you want to implement a custom debug tracer
pub mod debug {
    pub use crate::container::DebugTracer;
    pub use crate::coordinates::{GlobalBitPos, TileRelativeBitPos};
}

/// All the mux types in one place
pub mod mux {
    pub use crate::tiles::bram9k::{KMUX, TMUX};
    pub use crate::tiles::clocking::{GCLKMux4, InvertedMux17Inv};
    pub use crate::tiles::generic_routing::RMUX;
    pub use crate::tiles::hard_ip::{Mux13Inv, Mux17Inv};
    pub use crate::tiles::io::{
        IOClockMux, IOLocalToClockMux, LeftRightIOLocalMux, LocalToIOMux, TopBottomIOLocalMux,
    };
    pub use crate::tiles::local_lines::{CtrlMux, IMUX};
    pub use crate::tiles::logic::OMUX;
    pub use crate::tiles::{GlobalToLocalMux, Mux2, Mux2Inv, Mux3Inv};
}

/// All the tile types in one place
pub mod tile {
    pub use crate::tiles::TileRef as GenericTileRef;
    pub use crate::tiles::bram9k::BRAMTileRef;
    pub use crate::tiles::clocking::{GCLKSWTileRef, PLLTileRef};
    pub use crate::tiles::generic_routing::GenericRoutingRef;
    pub use crate::tiles::hard_ip::{LeftRightIPTileRef, TopIPTileRef};
    pub use crate::tiles::io::{LeftRightIOTileRef, TopBottomIOTileRef};
    pub use crate::tiles::logic::LogicTileRef;
    pub use crate::tiles::routing_only::RoutingOnlyTileRef;
}
