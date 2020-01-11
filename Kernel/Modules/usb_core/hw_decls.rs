
pub trait Descriptor
{
	const TYPE: u16;
	fn from_bytes(_: &[u8]) -> Option<Self> where Self: Sized;
}

#[repr(C)]
pub struct DeviceDescriptor
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
impl Descriptor for DeviceDescriptor
{
	const TYPE: u16 = 1;
	fn from_bytes(_: &[u8]) -> Option<Self> {
		None
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
