// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/acpi/aml.rs
//! ACPI Machine Language parser (VM)
use prelude::*;

struct Error;
struct AmlStream<'a>(&'a [u8],usize);

#[derive(Debug)]
enum FieldElement<'a>
{
	Named(&'a str, usize),
}

impl<'a> AmlStream<'a>
{
	pub fn new(s: &[u8]) -> AmlStream {
		AmlStream(s, 0)
	}
	
	pub fn empty(&self) -> bool {
		self.0.len() == 0
	}
	
	fn slice(&mut self, size: usize) -> Result<&'a [u8], Error> {
		if self.0.len() < size {
			Err( Error )
		}
		else {
			let res = &self.0[..size];
			self.0 = &self.0[size..];
			self.1 += size;
			Ok( res )
		}
	}
	fn take(&mut self, size: usize) -> Result<AmlStream<'a>,Error> {
		Ok( AmlStream::new( try!(self.slice(size)) ) )
	}
	fn read_byte(&mut self) -> Result<u8,Error> {
		Ok( try!(self.slice(1))[0] )
	}
	fn read_pkglength(&mut self) -> Result<usize,Error> {
		let lead = try!(self.read_byte());
		match lead >> 6
		{
		0 => Ok(lead as usize),
		count @ 1 ... 3 => {
			let mut rv = lead as usize & 0xF;
			for ofs in 0 .. count {
				let b = try!(self.read_byte()) as usize;
				rv |= b << (4 + ofs*8);
			}
			Ok( rv )
			},
		_ => unreachable!(),
		}
	}
	fn read_namestring(&mut self) -> Result<&'a str,Error> {
		let base = self.0;
		let mut len = 0;
		if len >= base.len() { return Err( Error ); }
		
		if base[len] == b'\\' {
			len += 1;
			if len >= base.len() { return Err( Error ); }
		}
		else {
			while base[len] == b'^' {
				len += 1;
				if len >= base.len() { return Err( Error ); }
			}
		}
		
		let ignore_last = match base[len]
			{
			0x00 => { len += 1; true },
			// TODO: Handle these with a custom format string that understands the decomposition
			//0x2E => { len += 8; false },
			//0x2F => {
			//	len += 1;
			//	if len >= base.len() { return Err( Error ); }
			//	let c = base[len];
			//	len += 4*c;
			//	false
			//	},
			b'A' ... b'Z' | b'_' => { len += 4; false },
			v @ _ => todo!("read_namestring - non-trivial path types ({:#02x})", v),
			};
		
		try!(self.slice(len));
		
		let rv_bytes = &base[0 .. len-(if ignore_last { 1 } else { 0 })];
		Ok( ::core::str::from_utf8(rv_bytes).unwrap() )
	}
	
	fn read_uint(&mut self, bytes: usize) -> Result<u64, Error> {
		let mut rv = 0;
		for i in 0 .. bytes {
			rv |= (try!(self.read_byte()) as u64) << i*8;
		}
		Ok( rv )
	}
	
	pub fn read_termarg(&mut self) -> Result<u64, Error> {
		match try!(self.read_byte())
		{
		// Type2Opcode
		0x00 => Ok( 0 ),
		0x01 => Ok( 1 ),
		// Data Object
		// - ComputationalData
		//  - ByteConst
		0x0A => Ok( try!(self.read_byte()) as u64 ), 
		0x0B => self.read_uint(2),
		0x0C => self.read_uint(4),
		0x0D => todo!("read_termarg - string"),
		0x0E => self.read_uint(8),
		// - DefPackage
		// - DefVarPackage
		// ArgObj
		// LocalObj
		v @ _ => todo!("read_termarg - {:#02x}", v),
		}
	}
	pub fn read_fieldelement(&mut self) -> Result<FieldElement,Error> {
		unimplemented!();
	}
}

fn dump_aml_termobj(data: &mut AmlStream) -> Result<usize,Error>
{
	match try!(data.read_byte())
	{
	// TermObj
	// -> NameSpaceModifierObj
	//  -> DefAlias
	0x06 => {
		let dst = try!(data.read_namestring());
		let src = try!(data.read_namestring());
		log_trace!("DefAlias {} {}", dst, src);
		},
	//  -> DefName
	0x08 => {
		// NameString
		let name = try!(data.read_namestring());
		// DataRefObject
		todo!("DefName DataRefObject (name={})", name);
		},
	//  -> DefScope
	0x10 => {
		let pkg_length = try!(data.read_pkglength());
		let name_string = try!(data.read_namestring());
		log_trace!("Node '{}' ({} bytes)", name_string, pkg_length);
		// TermList
		let mut subdata = try!(data.take(pkg_length));
		while !subdata.empty()
		{
			try!(dump_aml_termobj(&mut subdata));
		}
		log_trace!("CLOSE '{}'", name_string);
		},
	// -> NamedObj, -> Type1Opcode, -> Type2Opcode
	0x5B => match try!(data.read_byte())
		{
		// -> DefOpRegion
		0x80 => {
			let name = try!(data.read_namestring());
			let space = try!(data.read_byte());
			let ofs = try!(data.read_termarg());
			let len = try!(data.read_termarg());
			log_debug!("Region '{}' {} {:#x}+{}", name, space, ofs, len);
			},
		// -> DefField
		0x81 => {
			let len = try!(data.read_pkglength());
			let name = try!(data.read_namestring());
			let flags = try!(data.read_byte());	// FieldFlags
			// FieldList
			let mut subdata = try!(data.take(len));
			while !subdata.empty()
			{
				let e = try!(data.read_fieldelement());
				log_debug!("- {:?}", e);
			}
			},
		v @ _ => {
			log_warning!("Unknown byte 0x5B {:#02x} encountered in AML", v);
			return Err( Error );
			},
		},
	
	v @ _ => {
		log_warning!("Unknown byte {:#02x} encountered in AML", v);
		return Err( Error );
		},
	}
	Ok(0)
}

pub fn dump_aml(data: &[u8])
{
	::logging::hex_dump( "AML", data );
	
	match dump_aml_termobj( &mut AmlStream::new(data) )
	{
	Ok(s) => assert_eq!(s, data.len()),
	Err(_) => {},
	}
}

