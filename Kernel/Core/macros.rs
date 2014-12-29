//
//
//
#![macro_escape]

macro_rules! is
{
	($val:expr, $p:pat) => ( match $val { $p => true, _ => false } );
}

macro_rules! while_let
{
	($p:pat = $e:expr $code:block) => ( loop { match $e { $p => {$code}, _ => break } } );
}
macro_rules! if_let
{
	($p:pat = $e:expr $code:block) => ( match $e { $p => $code, _ =>{}} );
}

macro_rules! _count
{
	() => {0};
	($a:ident) => {1};
	($a:ident, $($b:ident)+) => {1+_count!($($b),+)};
}

macro_rules! module_define_int
{
	($name:ident, $count:expr, $deps:expr, $init:path) => (
		//#[assume_reachable]
		#[link_section = ".MODULE_LIST"]
		pub static mut _s_module: ::modules::ModuleInfo = ::modules::ModuleInfo {
			name: stringify!($name),
			init: $init,
			deps: &S_DEPS,
			_rsvd: 0,
		};
		static S_DEPS: [&'static str, ..($count)] = $deps;
	);
}

macro_rules! module_define
{
	($name:ident, [], $init:path) => (module_define_int!{$name, 0, [], $init});
	($name:ident, [$($deps:ident),+], $init:path) => (module_define_int!{$name, _count!($($deps),+), [$(stringify!($deps)),+], $init});
}

// Workaround for Any not working with trait inheritance
macro_rules! any_for_trait{
	($trait_:path) => {
		impl<'a> ::core::any::AnyRefExt<'a> for &'a $trait_ {
			#[inline]
			fn is<T: 'static>(self) -> bool {
			    // Get TypeId of the type this function is instantiated with
			    let t = ::core::intrinsics::TypeId::of::<T>();

			    // Get TypeId of the type in the trait object
			    let boxed = self.get_type_id();

			    // Compare both TypeIds on equality
			    t == boxed
			}

			#[inline]
			fn downcast_ref<T: 'static>(self) -> Option<&'a T> {
			    if self.is::<T>() {
				unsafe {
				    // Get the raw representation of the trait object
				    let to: ::core::raw::TraitObject = ::core::mem::transmute(self);

				    // Extract the data pointer
				    Some(::core::mem::transmute(to.data))
				}
			    } else {
				None
			    }
			}
		}
	}
}

// vim: ft=rust
