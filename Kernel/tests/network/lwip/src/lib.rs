
pub use lwip_sys as sys;

pub mod os_mode;


pub mod pbuf {
}
pub mod netconn;


pub struct Error(::lwip_sys::err_t);
impl Error {
    pub fn check<T>(v: T) -> Result<T,Self>
    where
        T: Copy,
        Self: ::std::convert::TryFrom<T>
    {
        match <Self as ::std::convert::TryFrom<T>>::try_from(v)
        {
        Ok(e) => Err(e),
        Err(_) => Ok(v),
        }
    }
    pub fn check_unit(v: ::lwip_sys::err_t) -> Result<(),Self> {
        if v >= ::lwip_sys::err_enum_t_ERR_OK as _ {
            Ok( () )
        }
        else {
            Err(Error(v))
        }
    }
}
macro_rules! impl_tryfrom {
    ( $($t:ty),+) => { $(
        impl ::std::convert::TryFrom<$t> for Error {
            type Error = $t;
            fn try_from(v: $t) -> Result<Self,Self::Error> {
                if v >= ::lwip_sys::err_enum_t_ERR_OK as _ {
                    Err(v)
                }
                else {
                    Ok(Error(v as _))
                }
            }
        })+
    }
}
impl_tryfrom!{ i32, ::lwip_sys::ssize_t }
macro_rules! fmt_error {
    ( $( $id:ident $desc:expr,)* ) => {
        impl ::core::fmt::Debug for Error {
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                let s = match self.0 as _
                    {
                    $( ::lwip_sys::$id => &stringify!($id)[4+5+2..], )*
                    _ => return write!(f, "LwipError({})", self.0),
                    };
                f.write_str("LwipError(")?;
                f.write_str(s)?;
                f.write_str(")")
            }
        }
        impl ::core::fmt::Display for Error {
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                let s = match self.0 as _
                    {
                    $( ::lwip_sys::$id => $desc, )*
                    _ => return write!(f, "Unknown lwip error {}", self.0),
                    };
                f.write_str(s)
            }
        }
    }
}
fmt_error! {
    err_enum_t_ERR_OK           "Ok",
    err_enum_t_ERR_MEM          "Out of memory error",
    err_enum_t_ERR_BUF          "Buffer error",
    err_enum_t_ERR_TIMEOUT      "Timeout",
    err_enum_t_ERR_RTE          "Routing problem",
    err_enum_t_ERR_INPROGRESS   "Operation in progress",
    err_enum_t_ERR_VAL          "Illegal value",
    err_enum_t_ERR_WOULDBLOCK   "Operation would block",
    err_enum_t_ERR_USE          "Address in use",
    err_enum_t_ERR_ALREADY      "Already connecting",
    err_enum_t_ERR_ISCONN       "Connection already established",
    err_enum_t_ERR_CONN         "Not connected",
    err_enum_t_ERR_IF           "Low-level netif error",
    err_enum_t_ERR_ABRT         "Connection aborted",
    err_enum_t_ERR_RST          "Connection reset",
    err_enum_t_ERR_CLSD         "Connection closed",
    err_enum_t_ERR_ARG          "Illegal argument",
}