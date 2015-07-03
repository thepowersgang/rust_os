
#[macro_export]
macro_rules! impl_fmt
{
	( $( <$($g:ident),+> $tr:ident ($s:ident, $f:ident) for $ty:ty { $code:expr } )+ ) => { $(
		impl<$($g),+> ::std::fmt::$tr for $ty {
			fn fmt(&$s, $f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
				$code
			}
		}
		)+
		};
	
	( $( $tr:ident ($s:ident, $f:ident) for $ty:ty { $code:expr } )+ ) => { $(
		impl ::std::fmt::$tr for $ty {
			fn fmt(&$s, $f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
				$code
			}
		}
		)+
		};
}

#[macro_export]
macro_rules! impl_from {
	($(From<$src:ty>($v:ident) for $t:ty { $($code:stmt)*} )+) => {
		$(
			impl ::std::convert::From<$src> for $t {
				fn from($v: $src) -> $t {
					$($code)*
				}
			}
		)+
	}
}
