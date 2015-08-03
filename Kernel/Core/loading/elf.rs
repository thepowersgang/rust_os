// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/loading/elf.rs
//! Executable and Linking Format (ELF) support
use prelude::*;
use lib::borrow::Cow;
use memory::virt::SliceAllocHandle;
use memory::PAddr;

#[repr(C,packed)]
#[derive(Clone)]
struct Elf32_Shent
{
	sh_name: u32,
	sh_type: u32,
	sh_flags: u32,
	sh_address: u32,
	sh_offset: u32,
	sh_size: u32,
	sh_link: u32,
	sh_info: u32,
	sh_addralign: u32,
	sh_entsize: u32,
}

#[repr(C,packed)]
#[derive(Clone,Debug)]
struct Elf32_Sym
{
	st_name: u32,
	st_value: u32,
	st_size: u32,
	st_info: u8,
	st_other: u8,
	st_shndx: u16,
}
unsafe impl ::lib::POD for Elf32_Sym {}

pub struct SectionHeader<'a>
{
	data: Cow<'a, [Elf32_Shent]>,
	_string_table_idx: usize,
}

pub struct StringTable<'a>
{
	data: SliceAllocHandle<u8>,
	_l: ::core::marker::PhantomData<&'a[u8]>
}

pub struct SymbolTable<'a>
{
	data: SliceAllocHandle<Elf32_Sym>,
	string_table: StringTable<'a>,
}

impl<'a> SectionHeader<'a>
{
	pub fn from_ref(buffer: &[u8], ent_size: usize, shstridx: usize) -> SectionHeader
	{
		assert_eq!(ent_size, ::core::mem::size_of::<Elf32_Shent>());
		// SAFE: POD transmute
		let buf = unsafe { ::lib::unsafe_cast_slice::<Elf32_Shent>(buffer) };
		let rv = SectionHeader {
			data: Cow::Borrowed(buf),
			_string_table_idx: shstridx,
			};
		rv.data[shstridx].dump();
		rv
	}
	pub fn dump(&self)
	{
		for (i,ent) in self.data.iter().enumerate()
		{
			log_debug!("[{}] {:?}", i, ent);
		}
	}
	
	pub fn string_table<'b>(&'b self, idx: usize) -> Result<StringTable<'b>,()>
	{
		let strtab = &self.data[idx];
		// SAFE: Assuming that sh_address is not currently mapped RW
		let alloc = match unsafe { ::memory::virt::map_hw_slice::<u8>(
				strtab.sh_address as PAddr,
				strtab.sh_size as usize
				) }
			{
			Ok(a) => a,
			Err(_) => return Err( () ),
			};
		Ok( StringTable {
			data: alloc,
			_l: ::core::marker::PhantomData,
			} )
	}
	
	pub fn symbol_table<'b>(&'b self) -> Result<SymbolTable<'b>,()>
	{
		let symtab = match self.data.iter().find(|e| e.sh_type == 2)
			{
			Some(e) => e,
			None => return Err( () ),
			};
		
		let count = symtab.sh_size as usize / ::core::mem::size_of::<Elf32_Sym>();
		// SAFE: (uncheckable) Assumes that sh_address does point to the symbol table (in physical space)
		let alloc = match unsafe { ::memory::virt::map_hw_slice::<Elf32_Sym>(symtab.sh_address as u64, count) }
			{
			Ok(v) => v,
			Err(e) => panic!("symbol table address invalid: {}", e),
			};
		Ok( SymbolTable {
			data: alloc,
			string_table: try!( self.string_table(symtab.sh_link as usize) ),
			} )
	}
	
	pub fn address_to_symbol(&self, address: usize) -> Option<(&str,usize)>
	{
		let symtab = self.symbol_table().unwrap();
		for (i,sym) in symtab.data.iter().enumerate()
		{
			log_debug!("sym [{}] {:?}", i, sym);
			log_debug!(" - {}", symtab.string_table.get(sym.st_name as usize));
			if sym.st_value as usize <= address && address < sym.st_value as usize + sym.st_size as usize {
				//let ofs = address - sym.st_value as usize;
				//return Some( (symtab.string_table.get(sym.st_name as usize), ofs) );
			}
		}
		
		None
	}
	
}

impl<'a> StringTable<'a>
{
	fn get(&self, ofs: usize) -> &str {
		let bytes = match self.data[ofs..].iter().position(|&x| x == 0)
			{
			Some(len) => &self.data[ofs .. ofs+len],
			None => &self.data[ofs..],
			};
		::core::str::from_utf8(bytes).unwrap()
	}
}

impl Elf32_Shent
{
	fn dump(&self)
	{
		// HACK: Assumes 64-bit kernel, and that kernel is loaded to -2GB
		// UNSAFE: (ish) References loaded kernel, should really check correctness.
		let buf: &[u8] = unsafe { ::core::slice::from_raw_parts( (0xFFFFFFFF_80000000 + self.sh_address as usize) as *const u8, self.sh_size as usize) };
		log_debug!("Elf32_Shent {:?}", ::lib::RawString(buf));
	}
}

impl ::core::fmt::Debug for Elf32_Shent
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result
	{
		write!(f, "Elf32_Shent {{ name: @{}, type:{}, flags:{:#x}, address:{:#x}, size:{:#x}, link:{}, info:{}, entsize:{} }}",
			self.sh_name, self.sh_type, self.sh_flags, self.sh_address, self.sh_size, self.sh_link, self.sh_info, self.sh_entsize)
	}
}

