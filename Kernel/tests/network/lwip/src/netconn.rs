use ::lwip_sys::*;

const TCP_DEFAULT_LISTEN_BACKLOG: u8 = 0xFF;

pub struct Tcp {
    conn: *mut ::lwip_sys::netconn,
}
impl ::core::ops::Drop for Tcp {
    fn drop(&mut self) {
        unsafe { crate::Error::check_unit(netconn_delete(self.conn)).expect("Error deleting a proto connection"); }
    }
}
impl Tcp {
    /// Create a new TCP `netconn`
    pub fn new() -> Result<Tcp,crate::Error> {
        let conn = unsafe { netconn_new_with_proto_and_callback(netconn_type_NETCONN_TCP, 0, None) };
        if conn == ::core::ptr::null_mut() {
            return Err(crate::Error(err_enum_t_ERR_MEM as _));
        }
        Ok(Tcp { conn })
    }
    /// Bind the local address of the connection
    pub fn bind(&mut self, addr: &ip_addr, port: u16) -> Result<(),crate::Error> {
        crate::Error::check_unit(unsafe { netconn_bind(self.conn, addr, port) })?;
        Ok( () )
    }
    /// Connect to a remote host
    pub fn connect(self, addr: &ip_addr, port: u16) -> Result<TcpConnection,crate::Error> {
        let conn = self.conn;
        crate::Error::check_unit(unsafe { netconn_connect(conn, addr, port) })?;
        ::core::mem::forget(self);
        Ok(TcpConnection { conn })
    }
    /// Listen for incoming connections
    pub fn listen(self) -> Result<TcpServer,crate::Error> {
        self.listen_with_backlog(TCP_DEFAULT_LISTEN_BACKLOG)
    }
    /// Listen for incoming connections (overriding the default backlog size of 255)
    pub fn listen_with_backlog(self, backlog: u8) -> Result<TcpServer,crate::Error> {
        let conn = self.conn;
        unsafe { crate::Error::check_unit(netconn_listen_with_backlog(conn, backlog))?; }
        ::core::mem::forget(self);
        Ok(TcpServer { conn })
    }
}

/// A TCP server socket
pub struct TcpServer {
    conn: *mut ::lwip_sys::netconn,
}
impl TcpServer {
    /// Shortcut to listen on [*]:<port>
    pub fn listen_with_backlog(port: u16, backlog: u8) -> Result<Self,crate::Error> {
        let mut rv = Tcp::new()?;
        let addr = unsafe { ::core::mem::zeroed::<::lwip_sys::ip_addr>() };
        rv.bind(&addr, port)?;
        rv.listen_with_backlog(backlog)
    }
    pub fn accept(&self) -> Option<TcpConnection> {
        unsafe {
            let mut new_conn = ::core::ptr::null_mut();
            match crate::Error::check_unit(::lwip_sys::netconn_accept(self.conn, &mut new_conn))
            {
            Ok( () ) => Some(TcpConnection { conn: new_conn }),
            Err(_) => None,
            }
        }
    }
}
impl ::core::ops::Drop for TcpServer {
    fn drop(&mut self) {
        unsafe {
            crate::Error::check_unit(netconn_close(self.conn)).expect("Error closing a sever connection");
            crate::Error::check_unit(netconn_delete(self.conn)).expect("Error deleting a sever connection");
        }
    }
}

pub struct TcpConnection {
    conn: *mut ::lwip_sys::netconn,
}
impl TcpConnection {
    /// Shortcut to connect to a remote host from an unspecified source
    pub fn connect(ip: ::lwip_sys::ip_addr, port: u16) -> Result<Self,crate::Error> {
        unsafe {
            let conn = ::lwip_sys::netconn_new_with_proto_and_callback(::lwip_sys::netconn_type_NETCONN_TCP, 0, None);

            crate::Error::check_unit(::lwip_sys::netconn_connect(conn, &ip, port))?;

            Ok(TcpConnection{ conn })
        }
    }
    
    /// Send a set of bytes to the connection
    pub fn send(&self, bytes: &[u8]) -> Result<usize,crate::Error> {
        unsafe {
            let mut res_len = 0;
            crate::Error::check_unit(::lwip_sys::netconn_write_partly(self.conn, bytes.as_ptr() as *const _, bytes.len() as _, 0, &mut res_len))?;
            Ok(res_len as usize)
        }
    }
    
    /// Receive data from the connection
    pub fn recv(&mut self) -> Result<Netbuf,crate::Error> {
        unsafe {
            let mut inbuf = ::core::ptr::null_mut();
            crate::Error::check_unit(::lwip_sys::netconn_recv(self.conn, &mut inbuf))?;
            let rv = Netbuf(inbuf);
            rv.get_slice()?;
            Ok(rv)
        }
    }
}
impl ::core::ops::Drop for TcpConnection {
    fn drop(&mut self) {
        unsafe {
            crate::Error::check_unit(netconn_disconnect(self.conn)).expect("Error deleting a socket connection");
            crate::Error::check_unit(netconn_close(self.conn)).expect("Error deleting a socket connection");
            crate::Error::check_unit(netconn_delete(self.conn)).expect("Error deleting a socket connection");
        }
    }
}


pub struct Netbuf(*mut ::lwip_sys::netbuf);
impl Netbuf {
    pub fn get_slice(&self) -> Result<&[u8],crate::Error> {
        unsafe {
            let mut buf_ptr = ::core::ptr::null();
            let mut buflen = 0;
            crate::Error::check_unit(::lwip_sys::netbuf_data(self.0, &mut buf_ptr as *mut _ as *mut _, &mut buflen))?;
            Ok( ::core::slice::from_raw_parts(buf_ptr, buflen as usize) )
        }
    }
}
impl ::core::ops::Drop for Netbuf {
    fn drop(&mut self) {
        unsafe {
            ::lwip_sys::netbuf_delete(self.0);
        }
    }
}