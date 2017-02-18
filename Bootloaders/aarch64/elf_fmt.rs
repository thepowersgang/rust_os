#[allow(non_camel_case_types)]
pub type Elf64_Half = u16;
#[allow(non_camel_case_types)]
pub type Elf64_Addr = u64;
#[allow(non_camel_case_types)]
pub type Elf64_Off = u64;
#[allow(non_camel_case_types)]
pub type Elf64_Sword = i32;
#[allow(non_camel_case_types)]
pub type Elf64_Word = u32;
#[allow(non_camel_case_types)]
pub type Elf64_Sxword = i32;
#[allow(non_camel_case_types)]
pub type Elf64_Xword = u64;

#[repr(C)]
pub struct ElfHeader {
	pub e_ident: [u8; 16],
	pub e_object_type: Elf64_Half,
	pub e_machine_type: Elf64_Half,
	pub e_version: Elf64_Word,

	pub e_entry: Elf64_Addr,
	pub e_phoff: Elf64_Off,
	pub e_shoff: Elf64_Off,

	pub e_flags: Elf64_Word,
	pub e_ehsize: Elf64_Half,

	pub e_phentsize: Elf64_Half,
	pub e_phnum: Elf64_Half,

	pub e_shentsize: Elf64_Half,
	pub e_shnum: Elf64_Half,
	pub e_shstrndx: Elf64_Half,
}
#[repr(C,packed)]
#[derive(Copy,Clone)]
pub struct Elf64_PhEnt {
	pub p_type: Elf64_Word,
	pub p_flags: Elf64_Word,
	pub p_offset: Elf64_Off,
	pub p_vaddr: Elf64_Addr,
	pub p_paddr: Elf64_Addr,	// aka load
	pub p_filesz: Elf64_Xword,
	pub p_memsz: Elf64_Xword,
	pub p_align: Elf64_Xword,
}
#[repr(C)]
#[derive(Copy,Clone)]
pub struct Elf64_ShEnt {
	pub sh_name: Elf64_Word,
	pub sh_type: Elf64_Word,
	pub sh_flags: Elf64_Xword,
	pub sh_addr: Elf64_Addr,
	pub sh_offset: Elf64_Off,
	pub sh_size: Elf64_Xword,
	pub sh_link: Elf64_Word,
	pub sh_info: Elf64_Word,
	pub sh_addralign: Elf64_Xword,
	pub sh_entsize: Elf64_Xword,
}
#[derive(Copy,Clone,Debug)]
pub struct SymEnt {
	pub st_name: Elf64_Word,
	pub st_info: u8,
	pub st_other: u8,
	pub st_shndx: Elf64_Half,
	pub st_value: Elf64_Addr,
	pub st_size: Elf64_Xword,
}

#[derive(Copy,Clone,Debug)]
#[allow(non_camel_case_types)]
pub struct Elf32_SymEnt {
	pub st_name: u32,
	pub st_value: u32,
	pub st_size: u32,
	pub st_info: u8,
	pub st_other: u8,
	pub st_shndx: u16,
}
