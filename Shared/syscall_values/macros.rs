
macro_rules! expand_expr { ($e:expr) => {$e}; }

// Define the non-object system calls (broken into groups)
macro_rules! def_groups {
	(
		$(
			$(#[$group_attrs:meta])*
			=$group_idx:tt: $group_name:ident = {
					$(
						$(? $cfg:meta)*
						$(#[$a:meta])*
						=$v:tt: $n:ident$(<$lft:lifetime>)? ( $($aname:ident : $aty:ty),* )$(-> $ret:ty)?,
					)*
				}
		),*
		$(,)*
	) => {
		#[repr(u32)]
		#[allow(non_camel_case_types,dead_code)]
		enum Groups {
			$($group_name = expand_expr!($group_idx)),*
		}
		mod group_calls { $(
			#[repr(u32)]
			#[allow(non_camel_case_types,dead_code)]
			pub enum $group_name {
				$(
					$(#[cfg($cfg)])*
					$n = expand_expr!($v),
				)*
			}
		)* }
		$( $(#[$group_attrs])* pub const $group_name: u32 = Groups::$group_name as u32; )*
		$( $(
			$(#[cfg($cfg)])*
			$(#[$a])*
			pub const $n: u32 = ($group_name << GRP_OFS) | (self::group_calls::$group_name::$n as u32);
		)* )*
		//pub mod group_args {
		//	use super::*;
			$( $(
			$(#[cfg($cfg)])*
			//$(#[$a])*
			def_args_struct!{$n $(< $lft >)? ( $( $aname: $aty,)* ) }
			)* )*
		//}
		//pub const GROUP_NAMES: &'static [&'static str] = &[
		//	$(stringify!($group_name),)*
		//	]; 
		};
}

// Define all classes, using c-like enums to ensure that values are not duplicated
macro_rules! def_classes {
	(
		$(
			$(#[$class_attrs:meta])*
			=$class_idx:tt: $class_name:ident = {
					// By-reference (non-moving) methods
					$( $(#[$va:meta])* =$vv:tt: $vn:ident$(<$v_lft:lifetime>)? ( $($v_aname:ident : $v_aty:ty),* ) $(-> $v_ret:ty)?, )*
					--
					// By-value (moving) methods
					$( $(#[$ma:meta])* =$mv:tt: $mn:ident$(<$m_lft:lifetime>)? ( $($m_aname:ident : $m_aty:ty),* ) $(-> $m_ret:ty)?, )*
				}|{
					// Events
					$( $(#[$ea:meta])* =$ev:tt: $en:ident, )*
				}
		),*
		$(,)*
	) => {
		#[repr(u16)]
		#[allow(non_camel_case_types,dead_code)]
		enum Classes {
			$($class_name = expand_expr!($class_idx)),*
		}
		mod calls { $(
			//#[repr(u16)]
			#[allow(non_camel_case_types,dead_code)]
			pub enum $class_name {
				$($vn = expand_expr!($vv),)*
				$($mn = expand_expr!($mv)|0x400),*
			}
		)* }
		//pub mod class_args {
		//	use super::*;
			$(
			$( def_args_struct!{$vn $(< $v_lft >)? ( $( $v_aname: $v_aty,)* ) } )*
			$( def_args_struct!{$mn $(< $m_lft >)? ( $( $m_aname: $m_aty,)* ) } )*
			)*
		//}
		mod masks { $(
			#[allow(non_camel_case_types,dead_code)]
			pub enum $class_name { $($en = expand_expr!($ev)),* }
		)* }
		$( $(#[$class_attrs])* pub const $class_name: u16 = Classes::$class_name as u16; )*
		$( $( $(#[$va])* pub const $vn: u16 = self::calls::$class_name::$vn as u16; )* )*
		$( $( $(#[$ma])* pub const $mn: u16 = self::calls::$class_name::$mn as u16; )* )*
		$( $( $(#[$ea])* pub const $en: u32 = 1 << self::masks::$class_name::$en as usize; )* )*
		pub const CLASS_NAMES: &'static [&'static str] = &[
			$(stringify!($class_name),)*
			]; 
		};
}

macro_rules! def_args_struct {
	($n:ident$(<$lft:lifetime>)? ( $($aname:ident : $aty:ty,)* )) => {
		#[allow(non_camel_case_types)]
		pub struct $n< $($lft,)? > {
			$( pub $aname: $aty ),*
		}
		impl< $($lft,)? > crate::Args for $n< $($lft,)? > {
			const CALL: u32 = $n as u32;
			type Tuple = ( $($aty,)* );
			fn from_tuple(t: ( $($aty,)* )) -> Self {
				def_args_struct!(@from_tuple t $n ($($aname)*))
			}
			fn into_tuple(self) -> Self::Tuple {
				($(self.$aname,)*)
			}
		}
	};
	(@from_tuple $t:ident $n:ident ()) => {{ let _ = $t; $n {} }};
	(@from_tuple $t:ident $n:ident ($a1:ident)) => {{ $n { $a1: $t.0 } }};
	(@from_tuple $t:ident $n:ident ($a1:ident $a2:ident)) => {{ $n { $a1: $t.0, $a2: $t.1 } }};
	(@from_tuple $t:ident $n:ident ($a1:ident $a2:ident $a3:ident)) => {{ $n { $a1: $t.0, $a2: $t.1, $a3: $t.2 } }};
	(@from_tuple $t:ident $n:ident ($a1:ident $a2:ident $a3:ident $a4:ident)) => {{
		$n { $a1: $t.0, $a2: $t.1, $a3: $t.2, $a4: $t.3 }
	}};
	(@from_tuple $t:ident $n:ident ($a1:ident $a2:ident $a3:ident $a4:ident $a5:ident)) => {{
		$n { $a1: $t.0, $a2: $t.1, $a3: $t.2, $a4: $t.3, $a5: $t.4 }
	}};
}


macro_rules! enum_to_from {
	($(#[$a_o:meta])* $enm:ident => $ty:ty : $( $(#[$a:meta])* $n:ident = $v:expr,)*) => {
		$(#[$a_o])*
		#[derive(Debug)]
		pub enum $enm
		{
			$(
			$(#[$a])*
			$n = $v,
			)*
		}
		impl $enm {
			#[allow(dead_code)]
			pub fn try_from(v: $ty) -> Result<Self,$ty> {
				match v
				{
				$($v => Ok($enm::$n),)*
				_ => Err(v),
				}
			}
		}
		impl crate::ToUsizeArray for $enm {
			const LEN: usize = 1;
			fn into_array(self) -> [usize; 1] {
				[self as $ty as usize]
			}
		}
		impl ::core::convert::Into<$ty> for $enm {
			fn into(self) -> $ty {
				match self
				{
				$($enm::$n => $v,)*
				}
			}
		}
	}
}