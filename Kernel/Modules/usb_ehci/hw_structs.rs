
#[repr(C,align(32))]
pub struct TransferDesc
{
    pub link: u32,
	pub link2: u32, // Used when there's a short packet
	pub token: u32,
	pub pages: [u32; 5],    //First has offset in low 12 bits
}

pub const QTD_TOKEN_DATATGL	    : u32 = 1<<31;
pub const QTD_TOKEN_IOC   	    : u32 = 1<<15;
pub const QTD_TOKEN_STS_ACTIVE	: u32 = 1<< 7;
pub const QTD_TOKEN_STS_HALT	: u32 = 1<< 6;

#[repr(C,align(32))]
pub struct QueueHead
{
    // 16 bytes
    pub hlink: u32,
    pub endpoint: u32,
    pub endpoint_ext: u32,
    pub current_td: u32,
    // 32 bytes - can't use TransferDesc as it's aligned
    pub overlay_link: u32,
	pub overlay_link2: u32, // Used when there's a short packet
	pub overlay_token: u32,
	pub overlay_pages: [u32; 5],    //First has offset in low 12 bits
    // Dead space: 16 bytes
}
/// H - Head of Reclamation List
pub const QH_ENDPT_H: u32 = 1<<15;