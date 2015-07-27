// Tifflin OS - Process management library
// - By John Hodge (thePowersGang)
//
// Process management support (between syscalls and std)
#![feature(no_std)]
#![no_std]
#![feature(core,core_prelude)]
use core::prelude::*;

#[macro_use]
extern crate core;
extern crate loader;
extern crate syscalls;

pub struct Process(::syscalls::threads::Process);

impl Process
{
	pub fn spawn<S: AsRef<[u8]>>(path: S) -> Process {
		match loader::new_process(path.as_ref(), &[])
		{
		Ok(v) => Process(v),
		Err(_) => panic!(""),
		}
	}
}
impl ::core::ops::Deref for Process {
    type Target = ::syscalls::threads::Process;
    fn deref(&self) -> &Self::Target { &self.0 }
}
