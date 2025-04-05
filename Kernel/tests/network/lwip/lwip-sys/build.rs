fn main() {
	let src_dir = ::std::path::Path::new("lwip-src/lwip-2.2.0/src");
	let os = ::std::env::var("CARGO_CFG_TARGET_OS").unwrap();
	if cfg!(feature="from_source") || os == "windows" {
		if ! ::std::fs::exists(src_dir).expect("Unable to check existence of lwip source folder") {
			let source_file = data_downloader::DownloadRequest {
					url: "http://download.savannah.nongnu.org/releases/lwip/lwip-2.2.0.zip",
					sha256_hash: &hex_literal::hex!("E12C769BE5A1DA9A1EDF1B8F38D645C6C87D52A26A636172CEA4B8C63EC04994"),
				};
			let source_zip_path = data_downloader::get_path(&source_file).expect("Failed to download source");
	
			::std::fs::create_dir_all("lwip-src").unwrap();
			::zip_extract::extract(
				::std::fs::File::open(&source_zip_path).expect("Unable to open downloaded zip"), 
				::std::path::Path::new("lwip-src"),
				false 
				).expect("Unable to extract source");
		}

		let mut build = cc::Build::new();
		build
			.file(src_dir.join("core/init.c"))
			.file(src_dir.join("core/def.c"))
			//.file(src_dir.join("core/dns.c"))
			.file(src_dir.join("core/inet_chksum.c"))
			.file(src_dir.join("core/ip.c"))
			.file(src_dir.join("core/mem.c"))
			.file(src_dir.join("core/memp.c"))
			.file(src_dir.join("core/netif.c"))
			.file(src_dir.join("core/pbuf.c"))
			.file(src_dir.join("core/raw.c"))
			.file(src_dir.join("core/stats.c"))
			//.file(src_dir.join("core/sys.c"))
			.file(src_dir.join("core/tcp.c"))
			.file(src_dir.join("core/tcp_in.c"))
			.file(src_dir.join("core/tcp_out.c"))
			.file(src_dir.join("core/timeouts.c"))
			.file(src_dir.join("core/udp.c"))
			//.file(src_dir.join("core/ipv4/autoip.c"))
			//.file(src_dir.join("core/ipv4/dhcp.c"))
			.file(src_dir.join("core/ipv4/etharp.c"))
			.file(src_dir.join("core/ipv4/icmp.c"))
			// .file(src_dir.join("core/ipv4/igmp.c")
			.file(src_dir.join("core/ipv4/ip4_frag.c"))
			.file(src_dir.join("core/ipv4/ip4.c"))
			.file(src_dir.join("core/ipv4/ip4_addr.c"))
			//.file(src_dir.join("core/ipv6/dhcp6.c"))
			//.file(src_dir.join("core/ipv6/ethip6.c"))
			.file(src_dir.join("core/ipv6/icmp6.c"))
			//.file(src_dir.join("core/ipv6/inet6.c"))
			.file(src_dir.join("core/ipv6/ip6.c"))
			.file(src_dir.join("core/ipv6/ip6_addr.c"))
			.file(src_dir.join("core/ipv6/ip6_frag.c"))
			.file(src_dir.join("core/ipv6/mld6.c"))
			.file(src_dir.join("core/ipv6/nd6.c"))
			//.file(src_dir.join("custom/sys_arch.c"))

			.file(src_dir.join("netif/ethernet.c"))

			//.file(src_dir.join("api/err.c"))
			.file(src_dir.join("api/api_lib.c"))
			.file(src_dir.join("api/api_msg.c"))
			.file(src_dir.join("api/tcpip.c"))
			.file(src_dir.join("api/netbuf.c"))

			.include(src_dir.join("custom"))
			.include(src_dir.join("include"))
			.warnings(false)
			.flag_if_supported("-Wno-everything")
			.include("src")	// For lwipopts.h
			;
		if os == "windows" {
			build
				.include(src_dir.join("../contrib/ports/win32/port/include"))
				;
		}
		else {
			build
				.include(src_dir.join("../contrib/ports/unix/port/include"))
				.file(src_dir.join("../contrib/ports/unix/port/sys_arch.c"))
				;
		}
		build.debug(true);
		build.compile("lwip");

		let mut builder = bindgen::Builder::default()
			.header("template.h")
			.clang_arg(format!("-I{}", "src"))
			.clang_arg(format!("-I{}", src_dir.join("include").display()))
			.clang_arg(format!("-I{}", src_dir.join("../contrib/ports/unix/port/include").display()))
			//.clang_arg("-I./old-src/custom")
			.clang_arg("-Wno-everything")
			.layout_tests(false)
			.parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
			;

		builder = builder
			.allowlist_type("err_enum_t")
			// tcpip_* - Hosted/OS mode
			.allowlist_function("tcpip_.*")
			// // LWIP BSD socket functions
			// .allowlist_function("lwip_.*")
			// .allowlist_type("lwip_.*")
			// .allowlist_var("SOCK_.*")
			// .allowlist_var("AF_.*")
			// .allowlist_type("sockaddr*")
			// netconn (native sockets)
			.allowlist_function("netconn_.*")
			.allowlist_var("NETCONN_FLAG_.*")
			// low-level APIs
			.allowlist_function("netif_.*").allowlist_var("NETIF_FLAG_.*")
			.allowlist_function("pbuf_.*").allowlist_var("PBUF_.*")
			.allowlist_function("netbuf_.*")
			.allowlist_function("etharp_.*")
			.allowlist_function("ip[46]addr_.*")
			;

		if os == "windows" {
			builder = builder.size_t_is_usize(false);
		}
	
		let bindings = builder.generate().expect("Unable to generate bindings");
	
		let out_path = ::std::path::PathBuf::from(::std::env::var("OUT_DIR").unwrap());
		bindings
			.write_to_file(out_path.join("bindings.rs"))
			.expect("Couldn't write bindings!");
	}
}