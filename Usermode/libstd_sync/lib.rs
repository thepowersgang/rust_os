//
//
//
#![feature(no_std,core,core_prelude)]
#![no_std]
#![feature(const_fn)]

#[macro_use]
extern crate core;

pub use mutex::Mutex;

pub mod mutex;

pub use core::atomic;


