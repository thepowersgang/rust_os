//! Evil hacky "unix" emulation for cmdline_words_parser

pub mod ffi
{
	pub trait OsStrExt
	{
		fn as_bytes(&self) -> &[u8];
	}

	impl OsStrExt for ::ffi::OsStr {
		fn as_bytes(&self) -> &[u8] {
			self.as_ref()
		}
	}
}
