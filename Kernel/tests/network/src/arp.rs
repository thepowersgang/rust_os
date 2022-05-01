
pub struct ArpPacket {
    pub hwtype: u16,
    pub swtype: u16,
    pub hwsize: u8,
    pub swsize: u8,
    pub op: u16,
    pub src_hwaddr: [u8; 6],
    pub src_swaddr: [u8; 4],
    pub dst_hwaddr: [u8; 6],
    pub dst_swaddr: [u8; 4],
}
impl ::std::fmt::Debug for ArpPacket {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "ArpPacket {{")?;
        write!(f, " HW: 0x{:04x} {},", self.hwtype, self.hwsize)?;
        write!(f, " SW: 0x{:04x} {},", self.swtype, self.swsize)?;
        write!(f, " Op: 0x{:04x},", self.op)?;
        write!(f, " Src HW:{:x?} SW:{:x?},", &self.src_hwaddr[..self.hwsize as usize], &self.src_swaddr[..self.swsize as usize])?;
        write!(f, " Dst HW:{:x?} SW:{:x?},", &self.dst_hwaddr[..self.hwsize as usize], &self.dst_swaddr[..self.swsize as usize])?;
        write!(f, " }}")
    }
}
impl ArpPacket
{
    pub fn parse(mut buf: &[u8]) -> (Self, &[u8])
    {
        fn read_bytes<const N: usize>(buf: &mut &[u8], len: usize) -> [u8; N] {
            let mut rv = [0; N];
            for i in 0..len {
                rv[i] = buf[i];
            }
            *buf = &buf[len..];
            rv
        }
        fn read_u16(buf: &mut &[u8]) -> u16 {
            let rv = u16::from_be_bytes([buf[0], buf[1]]);
            *buf = &buf[2..];
            rv
        }
        let hwtype = read_u16(&mut buf);
        let swtype = read_u16(&mut buf);
        let hwsize = buf[0]; buf = &buf[1..];
        let swsize = buf[0]; buf = &buf[1..];
        let op = read_u16(&mut buf);
        let rv = ArpPacket {
            hwtype,
            swtype,
            hwsize,
            swsize,
            op,
            src_hwaddr: read_bytes(&mut buf, hwsize as usize),
            src_swaddr: read_bytes(&mut buf, swsize as usize),
            dst_hwaddr: read_bytes(&mut buf, hwsize as usize),
            dst_swaddr: read_bytes(&mut buf, swsize as usize),
            };
        (rv, buf)
    }
    pub fn encode(&self) -> super::ArrayBuf<28> {
        let mut rv = super::ArrayBuf::new();

        rv.extend(self.hwtype.to_be_bytes());
        rv.extend(self.swtype.to_be_bytes());
        rv.extend(self.hwsize.to_be_bytes());
        rv.extend(self.swsize.to_be_bytes());
        rv.extend(self.op.to_be_bytes());

        rv.extend(self.src_hwaddr[..self.hwsize as usize].iter().copied());
        rv.extend(self.src_swaddr[..self.swsize as usize].iter().copied());
        rv.extend(self.dst_hwaddr[..self.hwsize as usize].iter().copied());
        rv.extend(self.dst_swaddr[..self.swsize as usize].iter().copied());

        rv
    }
}

pub struct ArpHandler
{
    my_ip: crate::ipv4::Addr,
}
impl ArpHandler {
    pub fn new(my_ip: crate::ipv4::Addr) -> Self {
        ArpHandler { my_ip }
    }
}
impl super::PacketHandler for ArpHandler
{
    fn check_packet(&mut self, fw: &super::TestFramework, data: &[u8]) -> bool {
        let (eh, data) = crate::ethernet::EthernetHeader::parse(data);
        if eh.proto != 0x0806 {
            return false;
        }

        let (pkt,_data) = ArpPacket::parse(data);
        println!("ArpHandler: RECV {:?}", pkt);
        assert!( pkt.hwtype == 1 ); // Ethernet
        assert!( pkt.swtype == 0x0800 ); // IPv4
        assert!( pkt.swsize == 4 );
        if pkt.op == 1 {    // Request?
            if pkt.dst_swaddr[..4] == self.my_ip.0 {    // For our IP?
                // Send a reply
                let reply = ArpPacket {
                    hwtype: 1,
                    swtype: 0x0800,
                    hwsize: 6, swsize: 4,
                    op: 2,  // Reply
                    dst_swaddr: pkt.src_swaddr,
                    dst_hwaddr: pkt.src_hwaddr,
                    src_hwaddr: crate::LOCAL_MAC,
                    src_swaddr: self.my_ip.0,
                    };
                println!("ArpHandler: SEND {:?}", reply);
                fw.send_ethernet_direct(0x0806, &[&reply.encode()]);
            }
            else {
                println!("ArpHandler: Ignoring ARP dest IP {}", crate::ipv4::Addr(pkt.dst_swaddr));
            }
        }
        else {
            println!("ArpHandler: Ignoring ARP op #{}", pkt.op);
        }

        true
    }
}
