

pub struct ClientSocket {
    conn: ::lwip::netconn::TcpConnection,
    cur_buf: Option<::lwip::netconn::Netbuf>,
    cur_ofs: usize,
}
impl ClientSocket {
    pub(super) fn from_conn(conn: ::lwip::netconn::TcpConnection) -> Self {
        ClientSocket { conn, cur_buf: None, cur_ofs: 0 }
    }
    pub fn connect(ip: ::lwip::sys::ip4_addr, port: u16) -> Result<Self,::lwip::Error> {
        let ip = ::lwip::sys::ip_addr {
            type_: ::lwip::sys::lwip_ip_addr_type_IPADDR_TYPE_V4 as u8,
            u_addr: ::lwip::sys::ip_addr__bindgen_ty_1 {
                ip4: ip,
            }
        };
        Ok(Self::from_conn( ::lwip::netconn::TcpConnection::connect(ip, port)? ))
    }

    pub fn send_data(&self, bytes: &[u8]) -> Result<usize,::lwip::Error> {
        self.conn.send(bytes)
    }
    pub fn recv_data(&mut self, buf: &mut [u8]) -> Result<usize,::lwip::Error> {

        fn partial_read(dst: &mut [u8], dst_ofs: &mut usize, src: &::lwip::netconn::Netbuf, src_ofs: &mut usize) -> Result<(), ::lwip::Error>
        {
            let src = src.get_slice()?;
            assert!(*src_ofs < src.len());
            assert!(*dst_ofs <= dst.len());
            let l = ::std::cmp::Ord::min(src.len() - *src_ofs, dst.len());

            dst[*dst_ofs..][..l].copy_from_slice(&src[*src_ofs..][..l]);
            *src_ofs += l;
            *dst_ofs += l;
            assert!(*src_ofs <= src.len());
            assert!(*dst_ofs <= dst.len());
            Ok( () )
        }

        let mut buf_ofs = 0;

        // If there's data from a previous read attempt, read from there first.
        if let Some(ref src) = self.cur_buf
        {
            partial_read(buf, &mut buf_ofs, src, &mut self.cur_ofs)?;
            
            if buf_ofs == buf.len() {
                self.cur_buf = None;
                self.cur_ofs = 0;
            }
            else {
                return Ok(buf_ofs);
            }
        }

        let inbuf = self.conn.recv()?;
        partial_read(buf, &mut buf_ofs, &inbuf, &mut self.cur_ofs)?;
        
        // If the output buffer consumed fully, then there is still data in the input buffer (or it's empty)
        if buf_ofs == buf.len() {
            self.cur_buf = Some(inbuf);
        }

        Ok(buf_ofs)
    }
}
