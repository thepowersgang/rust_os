#[allow(non_camel_case_types)]
pub type Elf32_Half = u16;
#[allow(non_camel_case_types)]
pub type Elf32_Addr = u32;
#[allow(non_camel_case_types)]
pub type Elf32_Off = u32;
#[allow(non_camel_case_types)]
pub type Elf32_Sword = i32;
#[allow(non_camel_case_types)]
pub type Elf32_Word = u32;

#[repr(C)]
pub struct ElfHeader {
	pub e_ident: [u8; 16],
	pub e_object_type: Elf32_Half,
	pub e_machine_type: Elf32_Half,
	pub e_version: Elf32_Word,

	pub e_entry: Elf32_Addr,
	pub e_phoff: Elf32_Off,
	pub e_shoff: Elf32_Off,

	pub e_flags: Elf32_Word,
	pub e_ehsize: Elf32_Half,

	pub e_phentsize: Elf32_Half,
	pub e_phnum: Elf32_Half,

	pub e_shentsize: Elf32_Half,
	pub e_shnum: Elf32_Half,
	pub e_shstrndx: Elf32_Half,
}
#[repr(C,packed)]
#[derive(Copy,Clone)]
pub struct PhEnt {
	pub p_type: Elf32_Word,
	pub p_offset: Elf32_Off,
	pub p_vaddr: Elf32_Addr,
	pub p_paddr: Elf32_Addr,	// aka load
	pub p_filesz: Elf32_Word,
	pub p_memsz: Elf32_Word,
	pub p_flags: Elf32_Word,
	pub p_align: Elf32_Word,
}
#[repr(C)]
#[derive(Copy,Clone)]
pub struct ShEnt {
	pub sh_name: Elf32_Word,
	pub sh_type: Elf32_Word,
	pub sh_flags: Elf32_Word,
	pub sh_addr: Elf32_Addr,
	pub sh_offset: Elf32_Off,
	pub sh_size: Elf32_Word,
	pub sh_link: Elf32_Word,
	pub sh_info: Elf32_Word,
	pub sh_addralign: Elf32_Word,
	pub sh_entsize: Elf32_Word,
}
#[derive(Copy,Clone,Debug)]
pub struct SymEnt {
	pub st_name: u32,
	pub st_value: u32,
	pub st_size: u32,
	pub st_info: u8,
	pub st_other: u8,
	pub st_shndx: u16,
}
