//! Routing database

use std::fmt::Display;

#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum WireType {
    T1,
    T4,
}
impl Display for WireType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::T1 => write!(f, "T1"),
            Self::T4 => write!(f, "T4"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Direction {
    N,
    S,
    E,
    W,
}
impl Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::N => write!(f, "N"),
            Self::S => write!(f, "S"),
            Self::E => write!(f, "E"),
            Self::W => write!(f, "W"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
enum RMUXSourceInternal {
    SpecialCaseInput,
    CellOutput(u8),
    RoutingWire {
        ty: WireType,
        going_dir: Direction,
        bundle: u8,
        wire_idx: u8,
    },
}

/// Possible sources to drive a RMUX
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum RMUXSource {
    /// One (of 4) global-to-local muxes in this tile
    GlobalToLocal(u8),
    /// The output of another RMUX in this tile
    RMUX(u8),
    /// The output of the internals of this tile
    CellOutput(u8),
    /// A routing wire coming into this tile
    RoutingWire {
        ty: WireType,
        going_dir: Direction,
        bundle: u8,
        wire_idx: u8,
    },
}
impl From<RMUXSourceInternal> for RMUXSource {
    fn from(value: RMUXSourceInternal) -> Self {
        match value {
            RMUXSourceInternal::SpecialCaseInput => unreachable!(),
            RMUXSourceInternal::CellOutput(i) => Self::CellOutput(i),
            RMUXSourceInternal::RoutingWire {
                ty,
                going_dir,
                bundle,
                wire_idx,
            } => Self::RoutingWire {
                ty,
                going_dir,
                bundle,
                wire_idx,
            },
        }
    }
}

pub fn rmux_input(rmux_idx: u8, inp_idx: u8, is_bram: bool) -> RMUXSource {
    if inp_idx != 4 {
        let mut ret = rmux::RMUX_MAP[rmux_idx as usize][inp_idx as usize].into();

        // In a BRAM tile, outputs [0-15] go to neighbor wires.
        // RMUXes get outputs [16-31] where the logic tile has LE outputs.
        // The final wires are special.
        if is_bram && let RMUXSource::CellOutput(i) = &mut ret {
            *i += 16;
        }

        ret
    } else {
        assert_eq!(
            rmux::RMUX_MAP[rmux_idx as usize][inp_idx as usize],
            RMUXSourceInternal::SpecialCaseInput
        );
        // RMUXes have a repeating pattern of 6
        let rmux_within_group = rmux_idx % 6;

        if !is_bram {
            // logic/routing tile
            match rmux_within_group {
                0 => RMUXSource::RMUX(rmux_idx + 1),
                1 | 2 | 3 => RMUXSource::GlobalToLocal(rmux_idx / 24),
                4 | 5 => RMUXSource::RMUX(rmux_idx - 2),
                _ => unreachable!(),
            }
        } else {
            // BRAM tile
            match rmux_within_group {
                0 => RMUXSource::GlobalToLocal(rmux_idx / 24),
                // here is where the special [32-35] outputs go
                1 | 2 | 3 => RMUXSource::CellOutput(32 + rmux_idx / 24),
                4 | 5 => RMUXSource::GlobalToLocal(rmux_idx / 24),
                _ => unreachable!(),
            }
        }
    }
}

mod rmux;
