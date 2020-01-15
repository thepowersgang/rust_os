
pub trait Descriptor
{
	const TYPE: u16;
	fn from_bytes(_: &[u8]) -> Option<Self> where Self: Sized;
}

#[repr(C)]
#[derive(Debug)]
pub struct Descriptor_Device
{
	pub length: u8,
	pub desc_type: u8,

	pub usb_version: u16,
	pub device_class: u8,
	pub device_sub_class: u8,
	pub device_protocol: u8,
	pub max_packet_size: u8,

	pub vendor_id: u16,
	pub device_id: u16,

	pub manufacturer_str: u8,
	pub product_str: u8,
	pub serial_number_str: u8,

	pub num_configurations: u8,
}
impl Descriptor for Descriptor_Device
{
	const TYPE: u16 = 1;
	fn from_bytes(b: &[u8]) -> Option<Self> {
		if b.len() != core::mem::size_of::<Self>() {
			None
		}
		else {
			use ::kernel::lib::PodHelpers;
			let mut rv: Self = PodHelpers::zeroed();
			rv.as_byte_slice_mut().copy_from_slice( b );
			Some(rv)
		}
	}
}

#[repr(C)]
pub struct Descriptor_String
{
	pub length: u8,
	pub desc_type: u8,	// = 3

	pub utf16: [u16; 127],
}
impl Descriptor for Descriptor_String
{
	const TYPE: u16 = 3;
	fn from_bytes(b: &[u8]) -> Option<Self> {
		if b.len() < 4 || b.len() % 2 != 0 {
			None
		}
		else {
			use ::kernel::lib::PodHelpers;
			let mut rv: Self = PodHelpers::zeroed();
			rv.as_byte_slice_mut()[..b.len()].copy_from_slice( b );
			Some(rv)
		}
	}
}

pub struct DeviceRequest
{
	pub req_type: u8,
	pub req_num: u8,

	pub value: u16,
	pub index: u16,
	pub length: u16,
}
impl DeviceRequest
{
	pub fn to_bytes(&self) -> [u8; 8] {
		[
			self.req_type,
			self.req_num,
			(self.value >> 0) as u8, (self.value >> 8) as u8,
			(self.index >> 0) as u8, (self.index >> 8) as u8,
			(self.length >> 0) as u8, (self.length >> 8) as u8,
			]
	}
}
