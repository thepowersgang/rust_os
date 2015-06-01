
use kernel::prelude::*;
use kernel::lib::byteorder::{ByteOrder,BigEndian};

macro_rules! def_cmd {
	($name:ident[$size:expr] $opcode:expr, ($($n:ident: $t:ty),*) => [$($values:expr),+]) => {
		pub struct $name([u8; $size]);
		impl AsRef<[u8]> for $name { fn as_ref(&self) -> &[u8] { &self.0 } }
		impl $name {
			const OPCODE: u8 = $opcode;
			pub fn new($($n:$t),*) -> Self {
				$name([ Self::OPCODE, $($values),+ ])
			}
		}
	};
}
macro_rules! def_rsp {
	($name:ident[$size:expr]) => {
		pub struct $name([u8;$size]);
		impl AsRef<[u8]> for $name { fn as_ref(&self) -> &[u8] { &self.0 } }
		impl AsMut<[u8]> for $name { fn as_mut(&mut self) -> &mut [u8] { &mut self.0 } }
		impl $name {
			pub fn new() -> Self { $name([0xFF; $size]) }
			pub fn len(&self) -> usize { $size }
		}
	};
}

pub struct Read6([u8; 6]);
impl AsRef<[u8]> for Read6 { fn as_ref(&self) -> &[u8] { &self.0 } }
impl Read6
{
	const CMD: u8 = 0x08;
	pub fn new(lba: u32, count: u8) -> Self {
		assert!(lba < 1<<24);
		Read6([
			Self::CMD,
			((lba >> 16) & 0xFF) as u8,
			((lba >>  8) & 0xFF) as u8,
			((lba >>  0) & 0xFF) as u8,
			count,
			0
			])
	}
	pub fn set_control(&mut self, control: u8) {
		self.0[5] = control;
	}
}

def_cmd!{ Read10[10] 0x28,
	(lba: u32, count: u16) => [
		0,	// 1: flags
		((lba >> 24) & 0xFF) as u8,
		((lba >> 16) & 0xFF) as u8,
		((lba >>  8) & 0xFF) as u8,
		((lba >>  0) & 0xFF) as u8,
		0,	// 6: group number
		((count >> 8) & 0xFF) as u8,
		((count >> 0) & 0xFF) as u8,
		0	// 9: control
	] }
impl Read10
{
	pub fn set_control(&mut self, control: u8) {
		self.0[9] = control;
	}
}

def_cmd!{ Read12[12] 0xA8,
	(lba: u32, count: u32) => [
		0,	// 1: flags
		((lba >> 24) & 0xFF) as u8,
		((lba >> 16) & 0xFF) as u8,
		((lba >>  8) & 0xFF) as u8,
		((lba >>  0) & 0xFF) as u8,
		((count >> 24) & 0xFF) as u8,
		((count >> 16) & 0xFF) as u8,
		((count >>  8) & 0xFF) as u8,
		((count >>  0) & 0xFF) as u8,
		0,	// 10: group number
		0	// 11: control
	] }
impl Read12
{
	pub fn set_control(&mut self, control: u8) {
		self.0[11] = control;
	}
}

def_cmd!{ Read16[16] 0x88,
	(lba: u64, count: u32) => [
		0,	// 1: flags
		((lba >> 56) & 0xFF) as u8,
		((lba >> 48) & 0xFF) as u8,
		((lba >> 40) & 0xFF) as u8,
		((lba >> 32) & 0xFF) as u8,
		((lba >> 24) & 0xFF) as u8,
		((lba >> 16) & 0xFF) as u8,
		((lba >>  8) & 0xFF) as u8,
		((lba >>  0) & 0xFF) as u8,
		0,	// 10: group number
		((count >> 24) & 0xFF) as u8,
		((count >> 16) & 0xFF) as u8,
		((count >>  8) & 0xFF) as u8,
		((count >>  0) & 0xFF) as u8,
		0	// 15: control
	] }
impl Read16
{
	pub fn set_control(&mut self, control: u8) {
		self.0[15] = control;
	}
}

def_cmd!{ Inquiry[6] 0x12,
	(alloc: u16) => [
		0,	// 1: EPVD
		0,	// 2: page code
		((alloc >> 8) & 0xFF) as u8,
		((alloc >> 0) & 0xFF) as u8,
		0	// 5: control
	] }
impl Inquiry
{
	pub fn set_epvd(&mut self, page: u8) {
		self.0[1] = 1;
		self.0[2] = page;
	}
}
def_rsp!{ InquiryRsp[255] }
impl InquiryRsp
{
	pub fn prehipheral_type(&self) -> u8 {
		self.0[0]
	}
	pub fn removable(&self) -> bool {
		self.0[1] & 0x80 != 0
	}
}


def_cmd!{ ReadCapacity10[10] 0x25,
	() => [
		0,	// reserved
		0,0,0,0,	// LBA
		0,0,	// reserved
		0,	// flags
		0	// control
	] }

def_rsp!{ ReadCapacity10Rsp[8] }
impl ReadCapacity10Rsp
{
	pub fn maxlba(&self) -> u32 {
		BigEndian::read_u32(&self.0[0..4])
	}
	pub fn block_length(&self) -> u32 {
		BigEndian::read_u32(&self.0[4..8])
	}
}

def_cmd!{ GetConfiguration[10] 0x46,
	(alloc: u16) => [
		0,	// mode (bottom two bits)
		0,0,	// starting feature
		0,0,0,	// reserved
		((alloc >> 8) & 0xFF) as u8,
		((alloc >> 0) & 0xFF) as u8,
		0	// control
	] }
impl GetConfiguration
{
	pub fn set_start(&mut self, feature: u16) {
		self.0[2] = ((feature >> 8) & 0xFF) as u8;
		self.0[3] = ((feature >> 0) & 0xFF) as u8;
	}
}

