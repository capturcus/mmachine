#![feature(variant_count)]

pub mod bits;
pub mod bus;
pub mod cpu_component;
pub mod microcodes;

extern crate lazy_static;
extern crate num;
#[macro_use]
extern crate num_derive;

#[cfg(test)]
mod tests;

use crate::cpu_component::*;
