//! Custom parsing
//!
// FIXME: Can we make this nicer somehow?

use std::str::FromStr;

use pannotia::prelude::*;

pub trait PannotiaParse {
    fn parse(s: &str) -> Result<Self, ()>
    where
        Self: Sized;
}

impl PannotiaParse for bool {
    fn parse(s: &str) -> Result<Self, ()> {
        parse_bool(s)
    }
}
impl PannotiaParse for ::bitmux::InvertedBool {
    fn parse(s: &str) -> Result<Self, ()> {
        Ok(bitmux::InvertedBool(parse_bool(s)?))
    }
}

macro_rules! impl_pannotia_parse {
    ($ty:ty) => {
        impl PannotiaParse for $ty {
            fn parse(s: &str) -> Result<Self, ()> {
                FromStr::from_str(s).map_err(|_| {})
            }
        }
    };
}
impl_pannotia_parse!(mux::Mux2);
impl_pannotia_parse!(mux::Mux2Inv);
impl_pannotia_parse!(mux::Mux3Inv);
impl_pannotia_parse!(mux::Mux13Inv);
impl_pannotia_parse!(mux::Mux17Inv);
impl_pannotia_parse!(mux::InvertedMux17Inv);
impl_pannotia_parse!(mux::GlobalToLocalMux);
impl_pannotia_parse!(mux::RMUX);
impl_pannotia_parse!(mux::TMUX);
impl_pannotia_parse!(mux::KMUX);
impl_pannotia_parse!(mux::OMUX);
impl_pannotia_parse!(mux::IMUX);
impl_pannotia_parse!(mux::CtrlMux);
impl_pannotia_parse!(mux::LocalToIOMux);
impl_pannotia_parse!(mux::IOLocalToClockMux);
impl_pannotia_parse!(mux::IOClockMux);
impl_pannotia_parse!(mux::TopBottomIOLocalMux);
impl_pannotia_parse!(mux::LeftRightIOLocalMux);
impl_pannotia_parse!(mux::GCLKMux4);
impl_pannotia_parse!(tiles::io::RegCtrlMode);
impl_pannotia_parse!(tiles::logic::InputCMode);
impl_pannotia_parse!(tiles::bram9k::ClockMode);
impl_pannotia_parse!(tiles::bram9k::PortWidth);

macro_rules! impl_parse_int {
    ($ty:ty) => {
        impl PannotiaParse for $ty {
            fn parse(s: &str) -> Result<Self, ()> {
                parse_int::parse::<Self>(s).map_err(|_| {})
            }
        }
    };
}
impl_parse_int!(u8);
impl_parse_int!(u16);
impl_parse_int!(u32);

pub fn parse_bool(s: &str) -> Result<bool, ()> {
    match s.to_ascii_lowercase().as_str() {
        "0" | "f" | "false" => Ok(false),
        "1" | "t" | "true" => Ok(true),
        _ => Err(()),
    }
}
