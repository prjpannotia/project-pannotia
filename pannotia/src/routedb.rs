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

        let mut dir = self.going_dir.flip();
        let mut cur_pos = tile_pos + dir;
        let mut did_flip_dir = false;
        let (tile_w, tile_h) = family.tile_dims();
        for _ in 0..self.bundle {
            // handle flipping wires around the edges
            if family.get_tile_type(cur_pos).is_boundary()
                // Apparently, there are loop wires in the empty tile below the BRAM
                || cur_pos.x == 0
                || cur_pos.x == tile_w - 1
                || cur_pos.y == 0
                || cur_pos.y == tile_h - 1
            {
                debug_assert!(!did_flip_dir, "cannot flip twice???");
                did_flip_dir = true;
                dir = dir.flip();
                // because the signals don't go "through" the edge tiles,
                // we have to do an additional add in order to take that into account
                cur_pos += dir;
            }

            cur_pos += dir;
        }

        // Handle looping through buffers
        let tile_type = family.get_tile_type(cur_pos);
        if tile_type.is_boundary() {
            let via_loop = match self.ty {
                WireType::T1 => tile_type.has_loop1(),
                WireType::T4 => tile_type.has_loop4(),
            };
            if via_loop {
                return AbsoluteRoutingWire {
                    tile: cur_pos + dir.flip(), // back up one tile
                    ty: self.ty,
                    going_dir: dir,
                    wire_idx: self.wire_idx,
                };
            }
        }

        AbsoluteRoutingWire {
            tile: cur_pos,
            ty: self.ty,
            going_dir: dir.flip(), // flip again to get the original going direction
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
impl RMUXSourceInternal {
    pub const fn into_rmux_source(self) -> RMUXSource {
        match self {
            Self::SpecialCaseInput => unreachable!(),
            Self::CellOutput(i) => RMUXSource::CellOutput(i),
            Self::RoutingWire {
                ty,
                going_dir,
                bundle,
                wire_idx,
            } => RMUXSource::RoutingWire(RoutingWire {
                ty,
                going_dir,
                bundle,
                wire_idx,
            }),
        }
    }
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
        value.into_rmux_source()
    }
}

/// Map of RMUX inputs
pub const fn rmux_input(rmux_idx: u8, inp_idx: u8, is_bram: bool) -> RMUXSource {
    if inp_idx != 4 {
        let mut ret = rmux::RMUX_MAP[rmux_idx as usize][inp_idx as usize].into_rmux_source();

        // In a BRAM tile, outputs [0-15] go to neighbor wires.
        // RMUXes get outputs [16-31] where the logic tile has LE outputs.
        // The final wires are special.
        if is_bram && let RMUXSource::CellOutput(i) = &mut ret {
            *i += 16;
        }

        ret
    } else {
        if let RMUXSourceInternal::SpecialCaseInput =
            rmux::RMUX_MAP[rmux_idx as usize][inp_idx as usize]
        {
        } else {
            panic!("RMUX table not as expected, should never happen!")
        }
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
    ///
    /// Index is arbitrary-ish but contiguous, and ranges up to 32 for the currently-supported families
    SelfWire(u8),
    /// A T1 left-going neighbor wire
    LeftNeighbor(u8),
    /// A span-4 wire
    Span4 { going_dir: Direction, wire_idx: u8 },
}

/// Map of RMUX index to what it actually does
pub use rmux::RMUX_PURPOSE;

/// Map of neighbor wire to the RMUX index that controls it
pub const fn rmux_idx_for_neighbor(i: u8) -> usize {
    i as usize * 6
}

/// Map of self-wire to the RMUX index that controls it
pub const fn rmux_idx_for_self(i: u8) -> usize {
    ((i / 2 * 6) + (i % 2) + 4) as usize
}

/// Map of span-4 wire to the RMUX index that controls it
pub const fn rmux_idx_for_span4(dir: Direction, wire_idx: u8) -> usize {
    (match dir {
        Direction::N => {
            (const { [25u8, 2, 75, 55, 32, 9, 85, 62, 39, 19, 92, 69] }[wire_idx as usize])
        }
        Direction::S => {
            (const { [73, 50, 27, 7, 80, 57, 37, 14, 87, 67, 44, 21] }[wire_idx as usize])
        }
        Direction::E => {
            (const { [1, 74, 51, 31, 8, 81, 61, 38, 15, 91, 68, 45] }[wire_idx as usize])
        }
        Direction::W => {
            (const { [49, 26, 3, 79, 56, 33, 13, 86, 63, 43, 20, 93] }[wire_idx as usize])
        }
    }) as usize
}

mod rmux;

/// Possible sources to drive a (LUT/BRAM) input signal
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum FunctionInputSource {
    /// The output of an RMUX in this tile
    ///
    /// This will be a T0 self-wire
    RMUX(u8),
    /// A T1 left-going wire from the right neighbor
    RightNeighborWire(u8),
    /// A T1 right-going wire from the left neighbor (only for BRAM tiles)
    LeftNeighborWire(u8),
    /// The output of an LE in this tile (only for logic tiles)
    LEOutput(u8),
    /// Dummy VCC input (only for BRAM tiles)
    Unused,
}

/// Map of IMUX inputs for a logic tile
pub const fn logic_imux_input(le_idx: u8, le_inp_idx: u8, imux_inp_idx: u8) -> FunctionInputSource {
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
                    FunctionInputSource::LEOutput(le_idx / 2 * 2 + 1)
                } else if imux_inp_idx > xtra_idx {
                    FunctionInputSource::LEOutput((imux_inp_idx - 1) * 2)
                } else {
                    FunctionInputSource::LEOutput(imux_inp_idx * 2)
                }
            }
            1 | 3 => {
                // LUT inputs B/D have access to odd LE outputs, with one extra odd output
                let xtra_idx = le_idx / 2;
                if imux_inp_idx == xtra_idx {
                    FunctionInputSource::LEOutput(le_idx / 2 * 2)
                } else if imux_inp_idx > xtra_idx {
                    FunctionInputSource::LEOutput((imux_inp_idx - 1) * 2 + 1)
                } else {
                    FunctionInputSource::LEOutput(imux_inp_idx * 2 + 1)
                }
            }
            _ => unreachable!(),
        },
        // Input 9 has access to the "top" half of the neighbor wires
        9 => FunctionInputSource::RightNeighborWire(le_idx % 2 * 4 + le_inp_idx),
        // Input 10 has access to the "bottom" half of the neighbor wires
        10 => FunctionInputSource::RightNeighborWire(le_idx % 2 * 4 + le_inp_idx + 8),
        11..=26 => match le_inp_idx {
            // LUT inputs A/C have access to the "first of the two in the group"
            0 | 2 => FunctionInputSource::RMUX((imux_inp_idx - 11) * 6 + 4),
            // LUT inputs B/D have access to the second
            1 | 3 => FunctionInputSource::RMUX((imux_inp_idx - 11) * 6 + 5),
            _ => unreachable!(),
        },
        _ => unreachable!(),
    }
}

/// Map of CtrlMUX inputs for a logic tile
pub const fn logic_ctrl_preselect_input(
    ctrlmux_idx: u8,
    ctrlmux_inp_idx: u8,
) -> FunctionInputSource {
    assert!(ctrlmux_idx < 4, "CtrlMux index out of range");
    assert!(ctrlmux_inp_idx < 32, "CtrlMux input index out of range");

    match ctrlmux_inp_idx {
        0..=7 => FunctionInputSource::LEOutput(ctrlmux_inp_idx * 2 + ctrlmux_idx % 2),
        8..=15 => {
            FunctionInputSource::RightNeighborWire((ctrlmux_inp_idx - 8) * 2 + ctrlmux_idx % 2)
        }
        _ => FunctionInputSource::RMUX((ctrlmux_inp_idx - 16) * 6 + 4 + ctrlmux_idx % 2),
    }
}

const BRAM_IMUX_EVEN_RMUX: [[u8; 18]; 4] = [
    [
        0, 4, 6, 10, 16, 22, 28, 34, 40, 46, 52, 58, 64, 70, 76, 82, 88, 94,
    ],
    [
        4, 10, 16, 22, 24, 28, 30, 34, 40, 46, 52, 58, 64, 70, 76, 82, 88, 94,
    ],
    [
        4, 10, 16, 22, 28, 34, 40, 46, 48, 52, 54, 58, 64, 70, 76, 82, 88, 94,
    ],
    [
        4, 10, 16, 22, 28, 34, 40, 46, 52, 58, 64, 70, 72, 76, 78, 82, 88, 94,
    ],
];

const BRAM_IMUX_ODD_RMUX: [[u8; 18]; 4] = [
    [
        5, 11, 12, 17, 18, 23, 29, 35, 41, 47, 53, 59, 65, 71, 77, 83, 89, 95,
    ],
    [
        5, 11, 17, 23, 29, 35, 36, 41, 42, 47, 53, 59, 65, 71, 77, 83, 89, 95,
    ],
    [
        5, 11, 17, 23, 29, 35, 41, 47, 53, 59, 60, 65, 66, 71, 77, 83, 89, 95,
    ],
    [
        5, 11, 17, 23, 29, 35, 41, 47, 53, 59, 65, 71, 77, 83, 84, 89, 90, 95,
    ],
];

/// Map of IMUX inputs for a BRAM tile
pub const fn bram_imux_input(imux_idx: u8, imux_inp_idx: u8) -> FunctionInputSource {
    assert!(imux_idx < 64, "IMUX index out of range");
    assert!(imux_inp_idx < 27, "IMUX input index out of range");

    // An IMUX has 27 inputs, which are divided up as follows:
    // [0-3]    a neighbor wire from the left (T1_E)
    // [4-7]    a neighbor wire from the right (T1_W)
    // [8-25]   a RMUX self-wire
    // 26       not used
    match imux_inp_idx {
        // for the T1 wires, there is a difference between the "bottom half" [0-31] IMUXes vs the "top half" [32-63]
        // in the bottom half, even IMUXes have wires 0/4/8/12 and odd IMUXes have wires 1/5/9/13
        // in the top half, even IMUXes have wires 2/6/10/14 and odd IMUXes have 3/7/11/15
        0..=3 => {
            if imux_idx < 32 {
                FunctionInputSource::LeftNeighborWire((imux_inp_idx % 4) * 4 + (imux_idx % 2))
            } else {
                FunctionInputSource::LeftNeighborWire((imux_inp_idx % 4) * 4 + 2 + (imux_idx % 2))
            }
        }
        4..=7 => {
            if imux_idx < 32 {
                FunctionInputSource::RightNeighborWire((imux_inp_idx % 4) * 4 + (imux_idx % 2))
            } else {
                FunctionInputSource::RightNeighborWire((imux_inp_idx % 4) * 4 + 2 + (imux_idx % 2))
            }
        }
        8..=25 => {
            // for the RMUXes, each group of 16 has access to the same mix of inputs
            let group_of_16 = imux_idx / 16;
            // and within this group of 16, the evens/odds have access to the same mix of inputs
            let rmux_even_odd = if imux_idx % 2 == 0 {
                BRAM_IMUX_EVEN_RMUX
            } else {
                BRAM_IMUX_ODD_RMUX
            };
            let rmux_inp_set = rmux_even_odd[group_of_16 as usize];
            // and after this we finally have a few small tables of entries
            FunctionInputSource::RMUX(rmux_inp_set[imux_inp_idx as usize - 8])
        }
        26 => FunctionInputSource::Unused,
        _ => unreachable!(),
    }
}

/// Map of CtrlMUX inputs for a BRAM tile
pub const fn bram_ctrl_preselect_input(
    ctrlmux_idx: u8,
    ctrlmux_inp_idx: u8,
) -> FunctionInputSource {
    assert!(ctrlmux_idx < 4, "CtrlMux index out of range");
    assert!(ctrlmux_inp_idx < 32, "CtrlMux input index out of range");

    match ctrlmux_inp_idx {
        0..=3 => FunctionInputSource::LeftNeighborWire(
            [0, 4, 10, 14][ctrlmux_inp_idx as usize] + ctrlmux_idx % 2,
        ),
        4..=7 => FunctionInputSource::RightNeighborWire(
            [0, 4, 10, 14][ctrlmux_inp_idx as usize - 4] + ctrlmux_idx % 2,
        ),
        8..=11 => FunctionInputSource::LeftNeighborWire(
            [2, 6, 8, 12][ctrlmux_inp_idx as usize - 8] + ctrlmux_idx % 2,
        ),
        12..=15 => FunctionInputSource::RightNeighborWire(
            [2, 6, 8, 12][ctrlmux_inp_idx as usize - 12] + ctrlmux_idx % 2,
        ),
        _ => FunctionInputSource::RMUX((ctrlmux_inp_idx - 16) * 6 + 4 + ctrlmux_idx % 2),
    }
}

/// Possible sources to drive a TMUX
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum TMUXSource {
    /// One (of 6) global-to-local muxes in this tile
    GlobalToLocal(u8),
    /// An unused input
    ///
    /// This shows up in place of a global-to-local wire sometimes
    Unused,
    /// The output of another RMUX in this tile
    RMUX(u8),
    /// A routing wire coming into this tile
    RoutingWire(RoutingWire),
}

mod tmux;

// Map of block RAM TMUX inputs
pub use tmux::TMUX_MAP;

/// Map of KMUX inputs
///
/// The input always comes from a TMUX
pub const fn kmux_input(kmux_idx: u8, mut inp_idx: u8) -> u8 {
    assert!(kmux_idx < 16, "KMUX index out of range");

    if kmux_idx != 0 {
        // for everything except the first one,
        // scoot over the output indices for inputs past a certain point
        if inp_idx >= kmux_idx - 1 {
            inp_idx += 1;
        }
    }

    inp_idx
}

/// Possible sources to drive an IO tile local line
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum IOLocalLineSource {
    /// A routing wire coming into this tile
    ///
    /// The direction of this wire need not be specified
    /// because IO tiles only have routing wires entering from one side.
    RoutingWire {
        ty: WireType,
        bundle: u8,
        wire_idx: u8,
    },
    /// One (of 2× number-of-IOs) global-to-local muxes in this tile
    GlobalToLocal(u8),
}

mod io_rmux;

/// Map of top/bottom IO inputs to local lines
pub const fn top_bottom_io_local_line_input(rmux_idx: u8, inp_idx: u8) -> IOLocalLineSource {
    assert!(rmux_idx < 32, "RMUX index out of range");
    assert!(inp_idx < 7, "RMUX input index out of range");

    // These occur in groups of 4, where each item in the group of 4 is the same
    io_rmux::TOP_BOTTOM_IO_RMUX_LOOKUP[rmux_idx as usize / 4][inp_idx as usize]
}

/// Map of top/bottom IO inputs to clocks (from local lines)
pub const fn top_bottom_io_clock_input(clkmux_idx: u8, inp_idx: u8) -> u8 {
    assert!(clkmux_idx < 8, "CtrlMUX index out of range");
    assert!(inp_idx < 8, "CtrlMUX input index out of range");

    [3, 7, 11, 15, 19, 23, 27, 31][inp_idx as usize]
}

/// Map of top/bottom IO inputs to non-clocks (from local lines)
pub const fn top_bottom_io_signal_input(iomux_idx: u8, inp_idx: u8) -> u8 {
    assert!(iomux_idx < 24, "IOMUX index out of range");
    assert!(inp_idx < 8, "IOMUX input index out of range");

    // These occur in groups of 8, where each item in the group of 8 is the same
    [
        // IOMUX 0
        [0, 4, 8, 12, 16, 20, 24, 28],
        // IOMUX 8
        [1, 5, 9, 13, 17, 21, 25, 29],
        // IOMUX 16
        [2, 6, 10, 14, 18, 22, 26, 30],
    ][iomux_idx as usize / 8][inp_idx as usize]
}

/// Map of left/right IO inputs to local lines
pub const fn left_right_io_local_line_input(rmux_idx: u8, inp_idx: u8) -> IOLocalLineSource {
    assert!(rmux_idx < 48, "RMUX index out of range");
    assert!(inp_idx < 9, "RMUX input index out of range");

    if inp_idx == 8 {
        // This last input is special, and is always a global2local in groups of _4_
        IOLocalLineSource::GlobalToLocal(rmux_idx / 4)
    } else {
        // These occur in groups of _6_, where each item in the group of _6_ is the same
        io_rmux::LEFT_RIGHT_IO_RMUX_LOOKUP[rmux_idx as usize / 6][inp_idx as usize]
    }
}

/// Map of left/right IO inputs to clocks (from local lines)
pub const fn left_right_io_clock_input(clkmux_idx: u8, inp_idx: u8) -> u8 {
    assert!(clkmux_idx < 12, "CtrlMUX index out of range");
    assert!(inp_idx < 8, "CtrlMUX input index out of range");

    if clkmux_idx < 4 {
        [4, 10, 16, 22, 28, 34, 40, 46][inp_idx as usize]
    } else {
        [5, 11, 17, 23, 29, 35, 41, 47][inp_idx as usize]
    }
}

/// Map of left/right IO inputs to non-clocks (from local lines)
pub const fn left_right_io_signal_input(iomux_idx: u8, inp_idx: u8) -> u8 {
    assert!(iomux_idx < 36, "IOMUX index out of range");
    assert!(inp_idx < 8, "IOMUX input index out of range");

    // These occur in groups of 8, where each item in the group of 8 is the same
    [
        [0, 6, 12, 18, 24, 30, 36, 42],
        [1, 7, 13, 19, 25, 31, 37, 43],
        [2, 8, 14, 20, 26, 32, 38, 44],
        [3, 9, 15, 21, 27, 33, 39, 45],
        [4, 10, 16, 22, 28, 34, 40, 46],
    ][iomux_idx as usize / 8][inp_idx as usize]
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

        for i in 0..16 {
            let rmux_i = rmux_idx_for_neighbor(i);
            assert_eq!(RMUX_PURPOSE[rmux_i], RMUXPurpose::LeftNeighbor(i));
        }

        for i in 0..32 {
            let rmux_i = rmux_idx_for_self(i);
            assert_eq!(RMUX_PURPOSE[rmux_i], RMUXPurpose::SelfWire(i));
        }
    }
}
