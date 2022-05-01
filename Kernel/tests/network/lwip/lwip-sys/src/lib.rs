
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(deref_nullptr)]
include!("bindgen.rs");

#[link(name="lwip")]
extern "C" {
}

/// Custom `Display` impl for IP addresses
impl ::core::fmt::Display for ip4_addr {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        let mut buf = [0u8; 4*4];
        unsafe { ip4addr_ntoa_r(self, buf.as_mut_ptr() as *mut _, buf.len() as i32) };
        let len = buf.iter().position(|&v| v == 0).unwrap_or(0);
        f.write_str(std::str::from_utf8(&buf[..len]).unwrap())
    }
}
/// Custom `Display` impl for IP addresses
impl ::core::fmt::Display for ip6_addr {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        let mut buf = [0u8; 5*8+1+3];
        unsafe { ip6addr_ntoa_r(self, buf.as_mut_ptr() as *mut _, buf.len() as i32) };
        let len = buf.iter().position(|&v| v == 0).unwrap_or(0);
        f.write_str(std::str::from_utf8(&buf[..len]).unwrap())
    }
}
impl ::core::fmt::Display for ip_addr {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match self.type_ as u32
        {
        lwip_ip_addr_type_IPADDR_TYPE_ANY => write!(f, "*"),
        lwip_ip_addr_type_IPADDR_TYPE_V4 => unsafe { self.u_addr.ip4.fmt(f) },
        lwip_ip_addr_type_IPADDR_TYPE_V6 => unsafe { self.u_addr.ip6.fmt(f) },
        _ => write!(f, "ip_addr#INVALID{}", self.type_),
        }
    }
}

impl ::core::fmt::Debug for ip_addr {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match self.type_ as u32
        {
        lwip_ip_addr_type_IPADDR_TYPE_ANY => write!(f, "IPADDR_TYPE_ANY"),
        lwip_ip_addr_type_IPADDR_TYPE_V4 => unsafe { self.u_addr.ip4.fmt(f) },
        lwip_ip_addr_type_IPADDR_TYPE_V6 => unsafe { self.u_addr.ip6.fmt(f) },
        _ => write!(f, "ip_addr#INVALID{}", self.type_),
        }
    }
}