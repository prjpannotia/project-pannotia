//! `pannotia`&mdash;handle bitstreams for [AGM32](https://www.agm-micro.com/) FPGAs, codenamed `rodinia`.

pub mod chips;
pub mod container;
pub mod coordinates;
pub mod packages;
pub mod padring;
pub mod routedb;
pub mod tiles;

#[inline]
pub(crate) const fn divroundup(x: u32, divisor: u32) -> u32 {
    (x + divisor - 1) / divisor
}
