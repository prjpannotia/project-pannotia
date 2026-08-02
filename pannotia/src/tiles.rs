//! Code for accessing every kind of FPGA tile
//!
//! In order to allow API users to refer to a specific tile
//! as a first-class object (rather than passing in coordinates every time),
//! every kind of tile has its own `SomeKindOfTileRef` struct
//! containing a reference to the bitstream itself plus the tile's coordinate.
//! This is abstracted over mutability by using the [Borrow]/[BorrowMut](std::borrow::BorrowMut) traits.
//!
//! The "generic" tile reference is [TileRef], and it is constructed by calling
//! [Bitstream::tile{_mut}](Bitstream::tile)
//!
//! To understand the FPGA's architecture, it is recommended
//! to read the documentation in the following order:
//!
//! 1. [generic_routing]
//! 2. [logic]
//! 3. [bram9k]
//! 4. [io]
//! 5. everything else

use std::borrow::Borrow;
use std::fmt::Display;
use std::marker::PhantomData;

use crate::container::{Bitstream, DebugTracer};
use crate::coordinates::*;

/// The kind of tile that exists at a given position
///
///
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum TileType {
    /// There is no tile here
    ///
    /// Places where there might not be a tile include the corners
    /// and holes left by embedded microcontroller hard blocks.
    ///
    /// This also includes "logical" tiles used by the vendor software
    /// which do not have any actual configuration bits,
    /// such as a "clock distribution" tile.
    None,

    /// A logic tile, containing LUTs
    Logic,

    /// A tile which only contains routing
    ///
    /// This is found on the right-hand side of the chip.
    RoutingOnly,

    /// A block RAM tile
    BRAM,

    /// IO, on the top and bottom sides
    TopBottomIO,
    /// IO, on the left and right sides
    LeftRightIO,

    /// Special function interface, on the top side
    ///
    /// Note that "top" is relative to "the rest of the logic fabric"
    /// and is not at the top of the entire tile grid.
    ///
    /// This tile type is used to connect the MCU to the logic fabric.
    /// In all cases seen so far, the MCU is in the upper-left corner,
    /// so the _bottom_ side of the MCU is interfaced with `TopIP` tiles.
    /// These tiles are found in a row in the middle of the tile grid,
    /// but at the "top" of the valid logic region.
    ///
    /// There is no "bottom" IP tile encountered anywhere.
    TopIP,
    /// Special function interface, on the left and right sides
    ///
    /// See the note for [TopIP](Self::TopIP).
    /// These tiles can be found both in the middle (for interfacing with the MCU)
    /// and on the actual tile grid boundary (for interfacing with analog IP).
    /// In all cases seen so far, "left" IP tiles connect to the MCU
    /// (which is in the upper-left), and "right" IP tiles connect to analog IP
    /// (replacing some right-hand-side IO tiles).
    LeftRightIP,

    /// Tile containing a PLL
    PLL,

    /// Tile controlling global clock distribution
    GCLKSW,
}

/// Functions common to all tile references
pub trait TileRefTrait<D: DebugTracer, Ref: Borrow<Bitstream<D>>> {
    /// Get the type of the current tile
    fn tile_type(&self) -> TileType;
    /// Get the position of the current tile
    fn pos(&self) -> TilePos;
    /// Downcast this back to a generic tile reference
    fn as_base_tile(self) -> TileRef<D, Ref>;
}

/// Generic reference to a tile
///
/// This can be coerced to a more-specific reference type
/// using the `as_*` functions. These functions all panic if the
/// tile type is not as expected. The tile type can be validated
/// by first calling [tile_type](Self::tile_type).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct TileRef<D: DebugTracer, Ref: Borrow<Bitstream<D>>> {
    r: Ref,
    p: TilePos,
    _d: PhantomData<D>,
}
impl<D: DebugTracer, Ref: Borrow<Bitstream<D>>> TileRefTrait<D, Ref> for TileRef<D, Ref> {
    fn tile_type(&self) -> TileType {
        let family = self.r.borrow().family();
        family.get_tile_type(self.p)
    }
    fn pos(&self) -> TilePos {
        self.p
    }
    fn as_base_tile(self) -> TileRef<D, Ref> {
        self
    }
}

impl<D: DebugTracer, Ref: Borrow<Bitstream<D>>> TileRef<D, Ref> {
    pub(crate) fn new(r: Ref, p: TilePos) -> Self {
        Self {
            r,
            p,
            _d: PhantomData,
        }
    }

    /// Coerce to a reference to a generic routing tile
    #[inline]
    pub fn as_generic_routing_tile(self) -> generic_routing::GenericRoutingRef<D, Ref> {
        let tile_type = self.tile_type();
        assert!(
            tile_type == TileType::Logic
                || tile_type == TileType::RoutingOnly
                || tile_type == TileType::BRAM
        );
        generic_routing::GenericRoutingRef {
            r: self.r,
            p: self.p,
            _d: PhantomData,
        }
    }

    /// Coerce to a reference to a logic tile
    #[inline]
    pub fn as_logic_tile(self) -> logic::LogicTileRef<D, Ref> {
        assert!(self.tile_type() == TileType::Logic);
        logic::LogicTileRef {
            r: self.r,
            p: self.p,
            _d: PhantomData,
        }
    }

    /// Coerce to a reference to a routing-only tile
    #[inline]
    pub fn as_routing_only_tile(self) -> routing_only::RoutingOnlyTileRef<D, Ref> {
        assert!(self.tile_type() == TileType::RoutingOnly);
        routing_only::RoutingOnlyTileRef {
            r: self.r,
            p: self.p,
            _d: PhantomData,
        }
    }

    /// Coerce to a reference to a block RAM (9216 bits) tile
    #[inline]
    pub fn as_bram9k_tile(self) -> bram9k::BRAMTileRef<D, Ref> {
        assert!(self.tile_type() == TileType::BRAM);
        bram9k::BRAMTileRef {
            r: self.r,
            p: self.p,
            _d: PhantomData,
        }
    }

    /// Coerce to a reference to a hard IP tile
    #[inline]
    pub fn as_top_ip_tile(self) -> hard_ip::TopIPTileRef<D, Ref> {
        assert!(self.tile_type() == TileType::TopIP);
        hard_ip::TopIPTileRef {
            r: self.r,
            p: self.p,
            _d: PhantomData,
        }
    }
    /// Coerce to a reference to a hard IP tile
    #[inline]
    pub fn as_leftright_ip_tile(self) -> hard_ip::LeftRightIPTileRef<D, Ref> {
        assert!(self.tile_type() == TileType::LeftRightIP);
        hard_ip::LeftRightIPTileRef {
            r: self.r,
            p: self.p,
            _d: PhantomData,
        }
    }

    /// Coerce to a reference to an I/O tile
    #[inline]
    pub fn as_topbottom_io_tile(self) -> io::TopBottomIOTileRef<D, Ref> {
        assert!(self.tile_type() == TileType::TopBottomIO);
        io::TopBottomIOTileRef {
            r: self.r,
            p: self.p,
            _d: PhantomData,
        }
    }
    /// Coerce to a reference to an I/O tile
    #[inline]
    pub fn as_leftright_io_tile(self) -> io::LeftRightIOTileRef<D, Ref> {
        assert!(self.tile_type() == TileType::LeftRightIO);
        io::LeftRightIOTileRef {
            r: self.r,
            p: self.p,
            _d: PhantomData,
        }
    }

    /// Coerce to a reference to a PLL tile
    #[inline]
    pub fn as_pll_tile(self) -> clocking::PLLTileRef<D, Ref> {
        assert!(self.tile_type() == TileType::PLL);
        clocking::PLLTileRef {
            r: self.r,
            p: self.p,
            _d: PhantomData,
        }
    }

    /// Coerce to a reference to a GCLKSW tile
    #[inline]
    pub fn as_gclksw_tile(self) -> clocking::GCLKSWTileRef<D, Ref> {
        assert!(self.tile_type() == TileType::GCLKSW);
        clocking::GCLKSWTileRef {
            r: self.r,
            p: self.p,
            _d: PhantomData,
        }
    }
}

/// A generic 2-choice mux
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[bitmux::bitenum]
pub enum Mux2 {
    _0 = "0",
    _1 = "1",
}
impl Display for Mux2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::_0 => write!(f, "0"),
            Self::_1 => write!(f, "1"),
        }
    }
}
impl Default for Mux2 {
    fn default() -> Self {
        Self::_0
    }
}
impl From<bool> for Mux2 {
    fn from(value: bool) -> Self {
        match value {
            false => Self::_0,
            true => Self::_1,
        }
    }
}
impl From<Mux2> for bool {
    fn from(value: Mux2) -> Self {
        match value {
            Mux2::_0 => false,
            Mux2::_1 => true,
        }
    }
}

/// (Helper) access to global2local, only in the core
pub(crate) struct GlobalToLocalMuxRef {
    pub(crate) is_bram: bool,
    pub(crate) i: u8,
}
impl FieldPositionCalculator for GlobalToLocalMuxRef {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        if !self.is_bram {
            assert!(self.i < 4, "GlobalToLocalMux index out of range");
        } else {
            assert!(self.i < 6, "GlobalToLocalMux index out of range");
        }

        if self.i < 4 {
            // There are 4 GlobalToLocalMux which fit into the "gap" between the
            // top half and the bottom half of a tile's control bits.
            let y = match self.i {
                0 | 2 => 32,
                1 | 3 => 35,
                _ => unreachable!(),
            };

            let mut x = 2
                + biti as u32
                + match self.i {
                    0 | 1 => 0,
                    2 | 3 => 6,
                    _ => unreachable!(),
                };

            // For a BRAM tile, this is scooted right by 1
            if self.is_bram {
                x += 1;
            }

            TileRelativeBitPos { y, x }
        } else {
            // BRAM tiles have 2 additional global-to-local muxes.
            // They sit in the leftmost column (the one which causes everything else to need to scoot over).

            let y = match self.i {
                4 => biti as u32 + 26,
                5 => 41 - biti as u32,
                _ => unreachable!(),
            };

            TileRelativeBitPos { y, x: 0 }
        }
    }
}

/// A mux for getting a global wire into a tile
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum GlobalToLocalMux {
    None,
    I(u8),
}
impl Default for GlobalToLocalMux {
    fn default() -> Self {
        Self::None
    }
}
impl Display for GlobalToLocalMux {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "<unset>"),
            Self::I(i) => write!(f, "#{i}"),
        }
    }
}
impl bitmux::BitstreamField for GlobalToLocalMux {
    fn get(b: impl bitmux::BitGetter) -> Self {
        let bits = b.get_bits::<6>();
        match bits {
            0b000001 => Self::I(0),
            0b000010 => Self::I(1),
            0b000100 => Self::I(2),
            0b001000 => Self::I(3),
            0b010000 => Self::I(4),
            0b100000 => Self::I(5),
            0b000000 => Self::None,
            _ => panic!("invalid GlobalToLocalMux {bits:06b}"),
        }
    }
    fn set(&self, mut b: impl bitmux::BitSetter) {
        let bits = match self {
            Self::I(0) => 0b000001,
            Self::I(1) => 0b000010,
            Self::I(2) => 0b000100,
            Self::I(3) => 0b001000,
            Self::I(4) => 0b010000,
            Self::I(5) => 0b100000,
            Self::None => 0b000000,
            _ => panic!("invalid GlobalToLocalMux {}", self),
        };
        b.set_bits::<6>(bits);
    }
}

/// A mux with two choices and an optional invert
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Mux2Inv {
    VCC,
    GND,
    I { invert: bool, i: u8 },
}
impl Default for Mux2Inv {
    fn default() -> Self {
        Self::VCC
    }
}
impl Display for Mux2Inv {
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
impl bitmux::BitstreamField for Mux2Inv {
    fn get(b: impl bitmux::BitGetter) -> Self {
        let bits = b.get_bits::<3>();
        let invert = bits & 0b100 != 0;
        match bits & 0b11 {
            0b01 => Self::I { invert, i: 0 },
            0b10 => Self::I { invert, i: 1 },
            0b00 if invert => Self::GND,
            0b00 if !invert => Self::VCC,
            _ => panic!("invalid Mux2Inv {bits:03b}"),
        }
    }
    fn set(&self, mut b: impl bitmux::BitSetter) {
        let bits = match self {
            Self::VCC => 0b000,
            Self::GND => 0b100,
            Self::I { invert, i: 0 } => 0b01 | if *invert { 0b100 } else { 0 },
            Self::I { invert, i: 1 } => 0b10 | if *invert { 0b100 } else { 0 },
            _ => panic!("invalid Mux2Inv {}", self),
        };
        b.set_bits::<3>(bits);
    }
}

/// A mux with three choices and an optional invert
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Mux3Inv {
    VCC,
    GND,
    I { invert: bool, i: u8 },
}
impl Default for Mux3Inv {
    fn default() -> Self {
        Self::VCC
    }
}
impl Display for Mux3Inv {
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
impl bitmux::BitstreamField for Mux3Inv {
    fn get(b: impl bitmux::BitGetter) -> Self {
        let bits = b.get_bits::<4>();
        let invert = bits & 0b1000 != 0;
        match bits & 0b11 {
            0b001 => Self::I { invert, i: 0 },
            0b010 => Self::I { invert, i: 1 },
            0b100 => Self::I { invert, i: 2 },
            0b000 if invert => Self::GND,
            0b000 if !invert => Self::VCC,
            _ => panic!("invalid Mux3Inv {bits:04b}"),
        }
    }
    fn set(&self, mut b: impl bitmux::BitSetter) {
        let bits = match self {
            Self::VCC => 0b0000,
            Self::GND => 0b1000,
            Self::I { invert, i: 0 } => 0b001 | if *invert { 0b1000 } else { 0 },
            Self::I { invert, i: 1 } => 0b010 | if *invert { 0b1000 } else { 0 },
            Self::I { invert, i: 2 } => 0b100 | if *invert { 0b1000 } else { 0 },
            _ => panic!("invalid Mux3Inv {}", self),
        };
        b.set_bits::<4>(bits);
    }
}

/// This trait is used for getting BRAM init data
pub trait BitSink {
    fn set(&mut self, biti: usize, val: bool);
}
impl<T: bitvec::store::BitStore, O: bitvec::order::BitOrder> BitSink
    for &mut bitvec::slice::BitSlice<T, O>
{
    fn set(&mut self, biti: usize, val: bool) {
        bitvec::slice::BitSlice::set(self, biti, val);
    }
}
impl BitSink for &mut [bool] {
    fn set(&mut self, biti: usize, val: bool) {
        self[biti] = val;
    }
}

/// This trait is used for setting BRAM init data
pub trait BitSource {
    fn get(&self, biti: usize) -> bool;
}
impl<T: bitvec::store::BitStore, O: bitvec::order::BitOrder> BitSource
    for &mut bitvec::slice::BitSlice<T, O>
{
    fn get(&self, biti: usize) -> bool {
        self[biti]
    }
}
impl BitSource for &[bool] {
    fn get(&self, biti: usize) -> bool {
        self[biti]
    }
}

macro_rules! _magic_replace_set_redirect_fn {
    // shift one statement at a time
    ($self:ident $val:ident $s0:stmt; $($rest:tt)*) => {
        $s0
        _magic_replace_set_redirect_fn! { $self $val $($rest)* }
    };
    // this should only apply to the last statement
    ($self:ident $val:ident self.$redir_to:ident($($args:tt)*)) => {
        mident::mident! {
            $self.#concat(set_ $redir_to)($($args)*, $val);
            panic!("test test {:?}", $val);
        }
    };
}

macro_rules! _magic_tile_impl_gen_one_item {
    // as a BitstreamField
    (read $self:ident $r:ty $body:block) => {
        let field_pos = $body;
        let ref_ = crate::coordinates::GenericFieldRef {
            bitstream: $self.r.borrow(),
            tile_pos: $self.p,
            field_pos,
            _d: std::marker::PhantomData,
        };
        ::bitmux::BitstreamField::get(ref_)
    };
    (write $self:ident $val:ident $r:ty $body:block) => {
        let field_pos = $body;
        let ref_ = crate::coordinates::GenericFieldRef {
            bitstream: $self.r.borrow_mut(),
            tile_pos: $self.p,
            field_pos,
            _d: std::marker::PhantomData,
        };
        ::bitmux::BitstreamField::set(&$val, ref_);
    };

    // as a raw integer
    (read $self:ident $nbits:literal bits in $r:ty $body:block) => {
        let field_pos = $body;
        let ref_ = crate::coordinates::GenericFieldRef {
            bitstream: $self.r.borrow(),
            tile_pos: $self.p,
            field_pos,
            _d: std::marker::PhantomData,
        };
        ::bitmux::BitGetter::get_bits::<$nbits>(&ref_) as $r
    };
    (write $self:ident $val:ident $nbits:literal bits in $r:ty $body:block) => {
        assert!(
            $val & !(((1u64 << $nbits) - 1) as $r) == 0,
            "invalid setting"
        );
        let field_pos = $body;
        let mut ref_ = crate::coordinates::GenericFieldRef {
            bitstream: $self.r.borrow_mut(),
            tile_pos: $self.p,
            field_pos,
            _d: std::marker::PhantomData,
        };
        ::bitmux::BitSetter::set_bits::<$nbits>(&mut ref_, $val as u32)
    };

    // redirect to another function
    (read $self:ident $r:ty = redir $($inside:tt)*) => {
        $($inside)*
    };
    (write $self:ident $val:ident $r:ty = redir $($inside:tt)*) => {
        _magic_replace_set_redirect_fn! { $self $val $($inside)* }
    };
}

macro_rules! _magic_tile_impl_gen_items {
    (read $($(#[$attr:meta])* $v:vis fn $f:ident(&$self:ident $($args:tt)* ) -> $($nbits:literal bits in)? $r:ty $(= { $($redir_tt:tt)* })? $($body:block)?)* ) => {
        $(
            #[doc = concat!("Read the field `", stringify!($f), "`\n\n")]
            $(#[$attr])*
            $v fn $f(&$self $($args)* ) -> $r {
                _magic_tile_impl_gen_one_item! { read $self $($nbits bits in)? $r $(= redir $($redir_tt)*)? $($body)? }
            }
        )*
    };
    (write $($(#[$attr:meta])* $v:vis fn $f:ident(&$self:ident $($args:tt)* ) -> $($nbits:literal bits in)? $r:ty $(= { $($redir_tt:tt)* })? $($body:block)?)* ) => {
        mident::mident! {
            $(
                #[doc = concat!("Write the field `", stringify!($f), "`\n\n")]
                $(#[$attr])*
                $v fn #concat(set_ $f)(&mut $self $($args)*, val: $r ) {
                    _magic_tile_impl_gen_one_item! { write $self val $($nbits bits in)? $r $(= redir $($redir_tt)*)? $($body)? }
                }
            )*
        }
    };
}

macro_rules! magic_tile_impl_gen {
    // impl a trait
    (impl on $impl_on:ident trait $trait:ty, $trait_mut:ty $(, get { $($get_item:item)* })? $(, set { $($set_item:item)* })? { $($inside:tt)* }) => {
        impl<D: DebugTracer, Ref: std::borrow::Borrow<Bitstream<D>>> $trait for $impl_on<D, Ref> {
            _magic_tile_impl_gen_items!{ read $($inside)* }
            $($( $get_item )*)?
        }
        impl<D: DebugTracer, Ref: std::borrow::BorrowMut<Bitstream<D>>> $trait_mut for $impl_on<D, Ref> {
            _magic_tile_impl_gen_items!{ write $($inside)* }
            $($( $set_item )*)?
        }
    };

    // impl on the struct itself
    (impl $impl_on:ident { $($inside:tt)* }) => {
        impl<D: DebugTracer, Ref: std::borrow::Borrow<Bitstream<D>>> $impl_on<D, Ref> {
            _magic_tile_impl_gen_items!{ read $($inside)* }
        }
        impl<D: DebugTracer, Ref: std::borrow::BorrowMut<Bitstream<D>>> $impl_on<D, Ref> {
            _magic_tile_impl_gen_items!{ write $($inside)* }
        }
    };
}

macro_rules! make_tile_ref {
    // handle the need to refer to self
    ($(#[$attr:meta])* $name:ident = $self:ident $override_ty:block) => {
        $(#[$attr])*
        #[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
        pub struct $name<D: DebugTracer, Ref: Borrow<Bitstream<D>>> {
            pub(super) r: Ref,
            pub(super) p: TilePos,
            pub(super) _d: PhantomData<D>,
        }
        impl<D: DebugTracer, Ref: Borrow<Bitstream<D>>> TileRefTrait<D, Ref> for $name<D, Ref> {
            fn tile_type(&$self) -> TileType {
                $override_ty
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
    };

    ($(#[$attr:meta])* $name:ident = $override_ty:expr) => {
        $(#[$attr])*
        #[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
        pub struct $name<D: DebugTracer, Ref: Borrow<Bitstream<D>>> {
            pub(super) r: Ref,
            pub(super) p: TilePos,
            pub(super) _d: PhantomData<D>,
        }
        impl<D: DebugTracer, Ref: Borrow<Bitstream<D>>> TileRefTrait<D, Ref> for $name<D, Ref> {
            fn tile_type(&self) -> TileType {
                $override_ty
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
    };
}

pub mod bram9k;
pub mod clocking;
pub mod generic_routing;
pub mod hard_ip;
pub mod io;
pub mod local_lines;
pub mod logic;
pub mod routing_only;
