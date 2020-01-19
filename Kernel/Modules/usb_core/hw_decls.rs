
pub struct ParseError;
impl_fmt! {
	Debug(self, f) for ParseError {
		f.write_str("ParseError")
	}
}

pub trait Descriptor
{
	const TYPE: u16;
	fn from_bytes(_: &[u8]) -> Result<Self,ParseError> where Self: Sized;
}
macro_rules! pod_descriptor {
	($t:ty, $ty_val:expr) => {
		impl Descriptor for $t {
			const TYPE: u16 = $ty_val;
			fn from_bytes(b: &[u8]) -> Result<Self,ParseError> {
				if b.len() != core::mem::size_of::<Self>() {
					Err(ParseError)
				}
				else {
					use ::kernel::lib::PodHelpers;
					let mut rv: Self = PodHelpers::zeroed();
					rv.as_byte_slice_mut().copy_from_slice( b );
					Ok(rv)
				}
			}
		}
	}
}
	
pub struct IterDescriptors<'a>(pub &'a [u8]);
impl<'a> Iterator for IterDescriptors<'a>
{
	type Item = &'a [u8];
	fn next(&mut self) -> Option<Self::Item>
	{
		if self.0 .len() == 0 {
			None
		}
		else {
			let len = self.0[0] as usize;
			if len > self.0 .len() {
				return None;
			}
			let rv = &self.0[..len];
			self.0 = &self.0[len..];
			Some(rv)
		}
	}
}
#[derive(Debug)]
pub enum DescriptorAny<'a>
{
	Unknown(&'a [u8]),
	Configuration(Descriptor_Configuration),
	Interface(Descriptor_Interface),
	Endpoint(Descriptor_Endpoint),
}
impl<'a> DescriptorAny<'a>
{
	pub fn from_bytes(b: &'a [u8]) -> Result<Self, ParseError> {
		if b.len() < 2 {
			return Err(ParseError);
		}
		Ok(match b[1] as u16
		{
		Descriptor_Configuration::TYPE => DescriptorAny::Configuration(Descriptor_Configuration::from_bytes(b)?),
		Descriptor_Interface::TYPE => DescriptorAny::Interface(Descriptor_Interface::from_bytes(b)?),
		Descriptor_Endpoint::TYPE => DescriptorAny::Endpoint(Descriptor_Endpoint::from_bytes(b)?),
		_ => DescriptorAny::Unknown(b),
		})
	}
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
	/// Device release number (binary-coded-decimal)
	pub bcd_device: u16,

	pub manufacturer_str: u8,
	pub product_str: u8,
	pub serial_number_str: u8,

	pub num_configurations: u8,
}
pod_descriptor! { Descriptor_Device, 1 }

#[repr(C)]
#[derive(Debug,Copy,Clone)]
pub struct Descriptor_Configuration
{
	pub length: u8,
	pub desc_type: u8,

	pub total_length: u16,
	pub num_interfaces: u8,
	pub configuration_value: u8,
	pub configuration_str: u8,
	pub attributes_bitmap: u8,
	/// Units: 2mA
	pub max_power: u8,
}
pod_descriptor! { Descriptor_Configuration, 2 }

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
	fn from_bytes(b: &[u8]) -> Result<Self,ParseError> {
		if b.len() < 4 || b.len() % 2 != 0 {
			Err(ParseError)
		}
		else {
			use ::kernel::lib::PodHelpers;
			let mut rv: Self = PodHelpers::zeroed();
			rv.as_byte_slice_mut()[..b.len()].copy_from_slice( b );
			Ok(rv)
		}
	}
}

#[repr(C)]
#[derive(Debug,Copy,Clone)]
pub struct Descriptor_Interface
{
	pub length: u8,
	pub desc_type: u8,	// = 4

	pub interface_num: u8,
	pub alternate_setting: u8,
	pub num_endpoints: u8,

	pub interface_class: u8,
	pub interface_sub_class: u8,
	pub interface_protocol: u8,

	pub interface_str: u8,
}
pod_descriptor!{ Descriptor_Interface, 4 }

#[repr(C)]
#[derive(Debug,Copy,Clone)]
pub struct Descriptor_Endpoint
{
	pub length: u8,
	pub desc_type: u8,	// = 5

	/// Endpoint address
	/// 0-3: Endpoint number
	/// 4-6: Reserved (zero)
	/// 7- Direction (0: OUT, 1: IN, x: Control)
	pub address: u8,
	/// 0-1: Transfer type (00: Control, 01: Isoch, 10: Bulk, 11: Interrupt)
	/// 2-3: Synchonosiation type (00: None, 01: Asynch, 10: Adaptive, 11: Synch)
	/// 4-5: Usage type (00: Data, 01: Feedback, 10: "Implicit feedback Data", 11: reserved)
	/// 7-7: reserved (zero)
	pub attributes: u8,
	/// Maximum packet size (little endian u16)
	/// 0-10: Packet size (in bytes)
	/// 11-12: (Isoch) extra transaction chances (00: 1 transaction/uFrame, 01: 2, 10: 3, 11: reserved)
	/// 13-15: reserved (zero)
	pub max_packet_size: (u8, u8),	// Avoid needing 2 byte alignment
	/// Max polling interval (frames, or uFrames - depends on device speed)
	/// NOTE: Encoding depends on endpoint type and device speed
	pub max_polling_interval: u8,
}
pod_descriptor!{ Descriptor_Endpoint, 5 }

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
