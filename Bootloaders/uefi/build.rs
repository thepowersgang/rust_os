fn main() {
	println!("cargo:rustc-link-lib=static=stubs");
	println!("cargo:rustc-link-search=.obj");
}
