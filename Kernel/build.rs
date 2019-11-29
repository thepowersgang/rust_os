
//
// Write a module list (so you only need to update Cargo.toml to enable a new module)
//

fn main()
{
	use std::io::Write;
	
	// Enumerate all of this crate's dependencies
	let deps = crate::get_deps::enumerate();
	
	
	let filename = {
		let mut p = std::path::PathBuf::from( std::env::var_os("OUT_DIR").unwrap() );
		p.push("modules.rs");
		p
		};
	let mut outfile = std::fs::File::create(&filename).expect("Cannot open modules.rs output file");
	writeln!(outfile, "{{").unwrap();
	for (dep_name, dep_info) in deps.iter()
	{
		// Explicitly ignore `syscalls` (main already links to it)
		if dep_name == "syscalls" {
			continue ;
		}
		
		let mangled_name = dep_name.replace('-',"_");
		match dep_info.source
		{
		get_deps::DepSource::Path(ref p) if p.starts_with("Modules/") => {
			writeln!(outfile, "extern crate {dep_name}; use_mod(&{dep_name}::S_MODULE); rv+=1;", dep_name=mangled_name).unwrap();
			},
		_ => {},
		}
	}
	writeln!(outfile, "}}").unwrap();
}

/// 
mod get_deps {
	
	use std::collections::{HashMap, HashSet};

	#[derive(Debug)]
	pub struct ActiveDependency
	{
		/// Source for the dependency code
		pub source: DepSource,
		/// Are default features included
		pub include_default_features: bool,
		/// Set of explicitly enabled features
		pub features: HashSet<String>,
	}
	#[derive(Debug)]
	pub enum DepSource
	{
		/// From git
		Git {
			url: String,
			branch: Option<String>,
			tag: Option<String>,
			rev: Option<String>,
			},
		/// Explicit path
		Path(String),
		/// A crates.io depdencency
		CratesIo(String),
	}
	
	pub fn enumerate() -> HashMap<String, ActiveDependency>
	{
		let manifest_path = {
			let mut p = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
			p.push("Cargo.toml");
			p
			};
		let content = match std::fs::read(&manifest_path)
			{
			Ok(v) => v,
			Err(e) => panic!("Unable to open {}: {:?}", manifest_path.display(), e),
			};
		let m = match cargo_toml::Manifest::from_slice(&content)
			{
			Ok(v) => v,
			Err(e) => panic!("Unable to parse {}: {:?}", manifest_path.display(), e),
			};
		//println!("{:?}", m);
		
		// Enumerate which of the declared features are active (activates dependency features)
		let mut dep_features = HashMap::<String, HashSet<String>>::new();
		for (feat_name, subfeats) in m.features
		{
			if std::env::var_os(format!("CARGO_FEATURE_{}", feat_name)).is_some()
			{
				for subfeat_desc in subfeats
				{
					let (dep, f) = {
						let mut it = subfeat_desc.split('/');
						( it.next().unwrap(), it.next().unwrap(), )
						};
					dep_features.entry(dep.to_string()).or_default().insert(f.to_string());
				}
			}
		}
		
		let mut rv = HashMap::new();
		for (depname, dep_info) in m.dependencies
		{
			if let Some(ad) = get_activedep(&dep_features, &depname, &dep_info)
			{
				rv.insert(depname.clone(), ad);
			}
		}
		
		let current_target = std::env::var("TARGET").unwrap();
		for (target_name, target_info) in m.target
		{
			println!("{:?}", target_name);
			// If the target begins with 'cfg', parse it as a cfg fragment
			let active =
				if target_name.starts_with("cfg") {
					let ml: syn::MetaList = match syn::parse_str(&target_name)
						{
						Ok(v) => v,
						Err(e) => panic!("Failed to parse target cfg {:?} - {:?}", target_name, e),
						};
					match check_cfg_root(&ml)
					{
					Some(v) => v,
					None => panic!("Failed to parse target cfg - {:?}", target_name),
					}
				}
				else {
					target_name == current_target
				};
			// If this target applies, enumerate dependencies
			if active
			{
				for (depname, dep_info) in target_info.dependencies
				{
					if let Some(ad) = get_activedep(&dep_features, &depname, &dep_info)
					{
						rv.insert(depname.clone(), ad);
					}
				}
			}
		}
		
		rv
	}

	/// Get an "ActiveDependency" for this `cargo_toml` dependency
	fn get_activedep(dep_features: &HashMap<String, HashSet<String>>, depname: &str, dep_info: &cargo_toml::Dependency) -> Option<ActiveDependency>
	{
		Some(match dep_info
		{
		cargo_toml::Dependency::Simple(version_str) => {
			ActiveDependency {
				source: DepSource::CratesIo(version_str.clone()),
				include_default_features: true,
				features: dep_features.get(depname).cloned().unwrap_or(HashSet::new()),
				}
			},
		cargo_toml::Dependency::Detailed(details) => {
			if details.optional && std::env::var_os(format!("CARGO_FEATURE_{}", depname)).is_none() {
				return None;
			}
			let source = 
				if let Some(ref version_str) = details.version {
					DepSource::CratesIo(version_str.clone())
				}
				else if let Some(ref path) = details.path {
					DepSource::Path(path.clone())
				}
				else if let Some(ref url) = details.git {
					DepSource::Git {
						url: url.clone(),
						branch: details.branch.clone(),
						tag: details.tag.clone(),
						rev: details.rev.clone(),
						}
				}
				else {
					panic!("?");
				};
			let mut features = dep_features.get(depname).cloned().unwrap_or(HashSet::new());
			for f in &details.features
			{
				features.insert(f.clone());
			}
			ActiveDependency {
				source: source,
				include_default_features: details.default_features.unwrap_or(true),
				features: features,
				}
			},
		})
	}
	
	/// Check `cfg()`-style targets
	fn check_cfg_root(ml: &syn::MetaList) -> Option<bool>
	{
		if ml.nested.len() != 1 {
			println!("Unexpected cfg(...) takes a single argument, {} provided", ml.nested.len());
			return None;
		}
		check_cfg( ml.nested.first().unwrap() )
	}
	fn check_cfg(m: &syn::NestedMeta) -> Option<bool>
	{
		let m = match m
			{
			syn::NestedMeta::Meta(m) => m,
			_ => return None,
			};
		Some(match m
		{
		syn::Meta::Path(_) => return None,
		syn::Meta::List(ml) => {
			let i = ml.path.get_ident()?;
			if i == "any" {
				for e in &ml.nested {
					if check_cfg(e)? {
						return Some(true);
					}
				}
				false
			}
			else if i == "not" {
				if ml.nested.len() != 1 {
					println!("Unexpected not(...) takes a single argument, {} provided", ml.nested.len());
					return None;
				}
				let e = ml.nested.first().unwrap();
				! check_cfg(e)?
			}
			else if i == "all" {
				for e in &ml.nested {
					if ! check_cfg(e)? {
						return Some(false);
					}
				}
				true
			}
			else {
				println!("Unexpected cfg fragment: {}", i);
				return None;
			}
			},
		syn::Meta::NameValue(nv) => {
			let i = nv.path.get_ident()?;
			let v = match &nv.lit
				{
				syn::Lit::Str(s) => s.value(),
				_ => {
					println!("cfg options require strings, got {:?}", nv.lit);
					return None;
					},
				};
			let ev = std::env::var(format!("CARGO_CFG_{}", i));
			ev == Ok(v)
			},
		})
	}
}
