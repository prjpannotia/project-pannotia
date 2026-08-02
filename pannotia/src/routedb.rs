//! Routing database

use std::{
    fmt::Display,
    ops::{Add, AddAssign},
};

use crate::coordinates::TilePos;

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
impl From<WireType> for u8 {
    fn from(value: WireType) -> Self {
        match value {
            WireType::T1 => 1,
            WireType::T4 => 4,
        }
    }
}
impl From<WireType> for u32 {
    fn from(value: WireType) -> Self {
        match value {
            WireType::T1 => 1,
            WireType::T4 => 4,
        }
    }
}
impl From<WireType> for usize {
    fn from(value: WireType) -> Self {
        match value {
            WireType::T1 => 1,
            WireType::T4 => 4,
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
impl Direction {
    pub fn flip(self) -> Self {
        match self {
            Self::N => Self::S,
            Self::S => Self::N,
            Self::E => Self::W,
            Self::W => Self::E,
        }
    }
}
impl Add<Direction> for crate::coordinates::TilePos {
    type Output = Self;
    fn add(self, rhs: Direction) -> Self::Output {
        match rhs {
            Direction::N => TilePos {
                x: self.x,
                y: self.y + 1,
            },
            Direction::S => TilePos {
                x: self.x,
                y: self.y - 1,
            },
            Direction::E => TilePos {
                x: self.x + 1,
                y: self.y,
            },
            Direction::W => TilePos {
                x: self.x - 1,
                y: self.y,
            },
        }
    }
}
impl AddAssign<Direction> for crate::coordinates::TilePos {
    fn add_assign(&mut self, rhs: Direction) {
        *self = *self + rhs;
    }
}

/// A general-purpose routing wire, tile-relative numbering
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct RoutingWire {
    pub ty: WireType,
    pub going_dir: Direction,
    pub bundle: u8,
    pub wire_idx: u8,
}
impl RoutingWire {
    pub fn to_absolute(
        self,
        family: crate::chips::Family,
        tile_pos: TilePos,
    ) -> AbsoluteRoutingWire {
        let max_bundle = match self.ty {
            WireType::T1 => match self.going_dir {
                Direction::N | Direction::S => 0, // not valid at all
                Direction::E | Direction::W => 1,
            },
            WireType::T4 => 4,
        };
        assert!(self.bundle < max_bundle, "invalid wire bundle");

        let mut src_pos = tile_pos;
        let mut dir = self.going_dir.flip();
        let (tile_w, tile_h) = family.tile_dims();
        let mut flip_dir = false;
        for _ in 0..self.bundle + 1 {
            // handle flipping wires around the edges
            let flip_at_edge = match dir {
                Direction::N if src_pos.y == tile_h - 1 => true,
                Direction::S if src_pos.y == 0 => true,
                Direction::E if src_pos.x == tile_w - 1 => true,
                Direction::W if src_pos.x == 0 => true,
                _ => false,
            };
            if flip_at_edge {
                debug_assert!(!flip_dir, "cannot flip twice???");
                flip_dir = true;
                dir = dir.flip();
                // because the signals don't go "through" the edge tiles,
                // we have to do an additional add in order to take that into account
                src_pos += dir;
            }

            src_pos += dir;
        }

        AbsoluteRoutingWire {
            tile: src_pos,
            ty: self.ty,
            going_dir: if flip_dir {
                self.going_dir.flip()
            } else {
                self.going_dir
            },
            wire_idx: self.wire_idx,
        }
    }
}

/// A general-purpose routing wire, absolute numbering
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct AbsoluteRoutingWire {
    pub tile: TilePos,
    pub ty: WireType,
    pub going_dir: Direction,
    pub wire_idx: u8,
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
    RoutingWire(RoutingWire),
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
            } => Self::RoutingWire(RoutingWire {
                ty,
                going_dir,
                bundle,
                wire_idx,
            }),
        }
    }
}

/// Map of RMUX inputs
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

/// What a given RMUX actually does
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum RMUXPurpose {
    /// A self-wire into the tile's logic
    SelfWire,
    /// A T1 left-going neighbor wire
    LeftNeighbor,
    /// A span-4 wire
    Span4 { going_dir: Direction, wire_idx: u8 },
}

/// Map of RMUX index to what it actually does
pub use rmux::RMUX_PURPOSE;

/// Map of span-4 wire to the RMUX index that controls it
pub use rmux::rmux_idx_for_span4;

mod rmux;

/// Possible sources to drive a (LUT/BRAM) IMUX
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum IMUXSource {
    /// The output of an RMUX in this tile
    ///
    /// This will be a T0 self-wire
    RMUX(u8),
    /// A T1 left-going wire from the right neighbor
    RightNeighborWire(u8),
    /// The output of an LE in this tile (only for logic tiles)
    LEOutput(u8),
}

/// Map of IMUX inputs for a logic tile
pub fn logic_imux_input(le_idx: u8, le_inp_idx: u8, imux_inp_idx: u8) -> IMUXSource {
    assert!(le_idx < 16, "LE index out of range");
    assert!(le_inp_idx < 4, "LE input index out of range");
    assert!(imux_inp_idx < 27, "IMUX input index out of range");

    // An IMUX has 27 inputs, which are divided up as follows:
    // [0-8]    a LE output
    // [9-10]   a neighbor wire
    // [11-26]  a RMUX self-wire
    match imux_inp_idx {
        0..=8 => match le_inp_idx {
            0 | 2 => {
                // LUT inputs A/C have access to even LE outputs, with one extra odd output
                let xtra_idx = le_idx / 2 + 1;
                if imux_inp_idx == xtra_idx {
                    IMUXSource::LEOutput(le_idx / 2 * 2 + 1)
                } else if imux_inp_idx > xtra_idx {
                    IMUXSource::LEOutput((imux_inp_idx - 1) * 2)
                } else {
                    IMUXSource::LEOutput(imux_inp_idx * 2)
                }
            }
            1 | 3 => {
                // LUT inputs B/D have access to odd LE outputs, with one extra odd output
                let xtra_idx = le_idx / 2;
                if imux_inp_idx == xtra_idx {
                    IMUXSource::LEOutput(le_idx / 2 * 2)
                } else if imux_inp_idx > xtra_idx {
                    IMUXSource::LEOutput((imux_inp_idx - 1) * 2 + 1)
                } else {
                    IMUXSource::LEOutput(imux_inp_idx * 2 + 1)
                }
            }
            _ => unreachable!(),
        },
        // Input 9 has access to the "top" half of the neighbor wires
        9 => IMUXSource::RightNeighborWire(le_idx % 2 * 4 + le_inp_idx),
        // Input 10 has access to the "bottom" half of the neighbor wires
        10 => IMUXSource::RightNeighborWire(le_idx % 2 * 4 + le_inp_idx + 8),
        11..=26 => match le_inp_idx {
            // LUT inputs A/C have access to the "first of the two in the group"
            0 | 2 => IMUXSource::RMUX((imux_inp_idx - 11) * 6 + 4),
            // LUT inputs B/D have access to the second
            1 | 3 => IMUXSource::RMUX((imux_inp_idx - 11) * 6 + 5),
            _ => unreachable!(),
        },
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rmux_consistency_check() {
        for dir in [Direction::N, Direction::S, Direction::E, Direction::W] {
            for i in 0..12 {
                let rmux_i = rmux_idx_for_span4(dir, i);
                assert_eq!(
                    RMUX_PURPOSE[rmux_i],
                    RMUXPurpose::Span4 {
                        going_dir: dir,
                        wire_idx: i
                    }
                )
            }
        }
    }
}
