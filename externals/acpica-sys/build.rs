
fn main() {
	/*
	let url = "https://acpica.org/sites/acpica/files/acpica-unix2-20150410.tar.gz";
	let src_dir = ::std::path::Path::new("acpica-src/acpica-unix2-20150410");
	let patch_dir = ::std::path::Path::new("patches");

	if ! ::std::fs::exists(src_dir).expect("Unable to check existence of source folder") {
		let source_file = data_downloader::DownloadRequest {
				url,
				sha256_hash: &hex_literal::hex!("9cd298a370335d6b0271a320a220e88e787aecfaa039a3b67ea627ee27972e60"),
			};
		let source_archive_path = data_downloader::get_path(&source_file).expect("Failed to download source");

		::std::fs::create_dir_all("acpica-src").unwrap();
		let mut a = ::tar::Archive::new( ::std::fs::File::open(&source_archive_path).expect("Unable to open downloaded archive") );
		a.unpack(::std::path::Path::new("acpica-src")).expect("Unable to extract source");
	}

	let files = ["source/include/platform/acrust.h"];
	let patches = ["source/include/platform/acenv.h"];
	for f in files {
		let src = patch_dir.join(f);
		let dst = src_dir.join(f);
		if ! ::std::fs::exists(&dst).unwrap_or(false) {
			::std::fs::copy(src, dst).expect("Unable to copy");
		}
	}
	for f in patches {
		let src = patch_dir.join(format!("{}.patch", f));
		let backup = src_dir.join(format!("{}.orig", f));
		let tmp = src_dir.join(format!("{}.tmp", f));
		let dst = src_dir.join(f);
		if ! ::std::fs::exists(&backup).unwrap_or(false) {
			let patch = ::std::fs::read_to_string(src).unwrap();
			let patch = ::patch_apply::Patch::from_single(&patch).expect("Malformed patch");
			let s = ::std::fs::read_to_string(&dst).unwrap();
			::std::fs::write(&tmp, ::patch_apply::apply(s, patch)).unwrap();
			::std::fs::rename(&dst, backup).expect("Unable to rename to backup");
			::std::fs::rename(tmp, dst).expect("Unable to rename to actual");
		}
	}
	*/
}