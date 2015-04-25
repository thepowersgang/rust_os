// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/loading/elf.rs
//! Executable and Linking Format (ELF) support
use _common::*;
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
	
	pub fn string_table(&self, idx: usize) -> Result<StringTable<'static>,()>
	{
		let strtab = &self.data[idx];
		let alloc = match ::memory::virt::map_hw_slice::<u8>(strtab.sh_address as PAddr, strtab.sh_size as usize)
			{
			Ok(a) => a,
			Err(_) => return Err( () ),
			};
		Ok( StringTable {
			data: alloc,
			_l: ::core::marker::PhantomData,
			} )
	}
	
	pub fn symbol_table(&self) -> Result<SymbolTable<'static>,()>
	{
		let symtab = match self.data.iter().find(|e| e.sh_type == 2)
			{
			Some(e) => e,
			None => return Err( () ),
			};
		
		let count = symtab.sh_size as usize / ::core::mem::size_of::<Elf32_Sym>();
		let alloc = ::memory::virt::map_hw_slice::<Elf32_Sym>(symtab.sh_address as u64, count).unwrap();
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
		let buf: &[u8] = unsafe { ::core::mem::transmute( ::core::raw::Slice { data: (0xFFFFFFFF_80000000 + self.sh_address as usize) as *const u8, len: self.sh_size as usize } ) };
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

