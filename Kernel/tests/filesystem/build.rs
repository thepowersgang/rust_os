//
// Write a module list (so you only need to update Cargo.toml to enable a new module)
//

fn main()
{
	use std::io::Write;
	
	// Enumerate all of this crate's dependencies
	let deps = my_dependencies::enumerate();
	
	let filename = {
		let mut p = std::path::PathBuf::from( std::env::var_os("OUT_DIR").unwrap() );
		p.push("modules.rs");
		p
		};
	let mut outfile = std::fs::File::create(&filename).expect("Cannot open modules.rs output file");
	writeln!(outfile, "{{").unwrap();
	for (dep_name, _dep_info) in deps.iter()
	{
		// Explicitly ignore `syscalls` (main already links to it)
		if !dep_name.starts_with("fs_") {
			continue ;
		}
		
		let mangled_name = dep_name.replace('-',"_");
		writeln!(outfile, "extern crate {dep_name}; use_mod(&{dep_name}::S_MODULE); rv+=1;", dep_name=mangled_name).unwrap();
	}
	writeln!(outfile, "}}").unwrap();
}
