
#[repr(C,align(32))]
pub struct TransferDesc
{
    pub link: u32,
	pub link2: u32, // Used when there's a short packet
	pub token: u32,
	pub pages: [u32; 5],    //First has offset in low 12 bits
}
impl TransferDesc {
    pub fn token_len(token: u32) -> usize {
        ((token >> 16) & 0x7FFF) as usize
    }
}
impl ::core::fmt::Debug for TransferDesc {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        struct V<'a>(&'a [u32; 5]);
        impl ::core::fmt::Debug for V<'_> {
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                let mut l = f.debug_list();
                let mut l = &mut l;
                for &v in self.0 {
                    if v == 0 { break; }
                    l = l.entry(&format_args!("{:#x}", v));
                }
                l.finish()
            }
        }
        f.debug_struct("TransferDesc")
            .field("link", &format_args!("{:#x}{}",
                self.link & !0xF,
                ["","/T"][ (self.link & 1) as usize ]
                ))
            .field("link2", &format_args!("{:#x}{}",
                self.link2 & !0xF,
                ["","/T"][ (self.link2 & 1) as usize ]
                ))
            .field("token", &format_args!("{nb}b{ioc}{dt}/{c_page}/{cerr}/{pid}/{status:02x}",
                nb = (self.token >> 16) & 0x7FFF,
                ioc = ["","/IOC"][ (self.token >> 15) as usize & 1 ],
                c_page = (self.token >> 12) & 7, // C_Page
                cerr = (self.token >> 10) & 3, // CERR
                pid = ["OUT","IN","SETUP","resv"][ (self.token >> 8) as usize & 3 ],
                status = self.token & 0xFF,
                dt = if self.token & QTD_TOKEN_DATATGL != 0 { "/DT" } else { "" },
                ))
            .field("pages", &V(&self.pages))
            .finish()
    }
}

pub const QTD_TOKEN_DATATGL	    : u32 = 1<<31;
pub const QTD_TOKEN_IOC   	    : u32 = 1<<15;
pub const QTD_TOKEN_STS_ACTIVE	: u32 = 1<< 7;
pub const QTD_TOKEN_STS_HALT	: u32 = 1<< 6;

#[repr(C,align(32))]
pub struct QueueHead    // sizeof = 64 = 0x40
{
    // 16 bytes
    /// Horizontal link:
    /// - 31:5 = Address
    /// - 4:3 = Reserved
    /// - 2:1 = Type (00=iTD, 01=QH, 10=siTD, 11=FSTN)
    /// - 0 = Terminate (only used for non-async, see spec 4.8.2)
    pub hlink: u32,
    pub endpoint: u32,
    pub endpoint_ext: u32,
    /// Pointer to the current descriptor (note: this is written by hardware)
    pub current_td: u32,
    // 32 bytes - can't use TransferDesc as it's aligned
    pub overlay_link: u32,
	pub overlay_link2: u32, // Used when there's a short packet
	pub overlay_token: u32,
	pub overlay_pages: [u32; 5],    //First has offset in low 12 bits
    // Dead space: 16 bytes
    // TODO: Store the interrupt metadata here
}
impl ::core::fmt::Debug for QueueHead {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        f.debug_struct("QueueHead")
            .field("hlink", &format_args!("{addr:#x}/{ty}{t}",
                addr = self.hlink & !0xF,
                ty = ["iTD","QH","siTD","FSTN"][ ((self.hlink >> 1) & 3) as usize ],
                t = ["","/T"][ (self.hlink & 1) as usize ]
                ))
            .field("endpoint", &format_args!("{:#x}", self.endpoint))
            .field("endpoint_ext", &format_args!("{:#x}", self.endpoint_ext))
            .field("current_td", &format_args!("{:#x}", self.current_td))
            .field("overlay_*", &TransferDesc {
                link: self.overlay_link,
                link2: self.overlay_link2,
                token: self.overlay_token,
                pages: self.overlay_pages,
                })
            .finish()
    }
}
//pub const QH_HLINK_TERMINATE: u32 = 1<<0;
pub const QH_HLINK_TY_QH: u32 = 1<<1;
/// H - Head of Reclamation List
pub const QH_ENDPT_H: u32 = 1<<15;

#[repr(u32)]
pub enum Pid
{
    Out = 0,
    In = 1,
    Setup = 2,
}