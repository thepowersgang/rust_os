
use ::kernel::{log,log_error,log_log};
use ::kernel::vfs::handle as vfs_handle;

mod virt_storage;

fn main()
{
    ::kernel::threads::init();
    ::kernel::memory::phys::init();
    ::kernel::memory::page_cache::init();
    (::kernel::metadevs::storage::S_MODULE.init)();
    (::kernel::hw::mapper_mbr::S_MODULE.init)();
    (::kernel::vfs::S_MODULE.init)();

    // -- TODO: use `my_dependencies`?
    (::fs_fat::S_MODULE.init)();
    (::fs_ext_n::S_MODULE.init)();
    // --

    let cmd_stream = ::std::io::stdin();
    loop
    {
        let mut s = String::new();
        cmd_stream.read_line(&mut s).expect("Reading user input");

        // Parse using cmdline_words_parser
        let mut args = ::cmdline_words_parser::parse_posix(&mut s);
        let cmd = match args.next()
            {
            None => break,
            Some("") => break,  // Blank means a space? (or a leading space)
            Some(v) => v,
            };
        match cmd
        {
        "add_disk" => {
            let Some(name) = args.next() else { panic!("`add_disk`: missing `name` argument") };
            let Some(path) = args.next() else { panic!("`add_disk`: missing `path` argument") };
            let Some(overlay) = args.next() else { panic!("`add_disk`: missing `overlay` argument") };
            let overlay = match overlay
                {
                "write-through"|"none" => virt_storage::OverlayType::None,
                "transient"|"temporary" => virt_storage::OverlayType::Temporary,
                "persistent" => virt_storage::OverlayType::Persistent,
                _ => panic!("`add_disk`: Invalid `overlay` argument"),
                };
            log_log!("CMD add_disk {} := {} {:?}", name, path, overlay);
            match crate::virt_storage::add_volume(name, path.as_ref(), overlay)
            {
            Ok(()) => {},
            Err(e) => panic!("`add_disk`: Unable to open {} as {}: {:?}", path, name, e),
            }
            },
        "mount" => {
            let Some(mountpt) = args.next() else { panic!("`mount`: missing `mountpt` argument") };
            let Some(volume) = args.next() else { panic!("`mount`: missing `volume` argument") };
            let filesystem = args.next().unwrap_or("");
            let options = args.next().map(|v| v.split(",").collect::<Vec<_>>()).unwrap_or_default();
            log_log!("COMMAND: mount {mountpt:?} := {volume:?} fs={filesystem:?} options={options:?}");

            let vh = match ::kernel::metadevs::storage::VolumeHandle::open_named(volume)
                {
                Ok(vh) => vh,
                Err(e) => panic!("`mount`: Unable to open {}: {}", volume, e),
                };
            match ::kernel::vfs::mount::mount(mountpt.as_ref(), vh, filesystem, &options)
            {
            Ok(_) => {},
            Err(e) => panic!("`mount`: Unable to mount {} from {}: {:?}", mountpt, volume, e),
            }
            },
        // List directory
        "ls" => {
            let dir = ::kernel::vfs::Path::new( args.next().expect("ls dir") );
            log_log!("COMMAND: ls {:?}", dir);
            match vfs_handle::Dir::open(dir)
            {
            Err(e) => log_error!("'{:?}' cannot be opened: {:?}", dir, e),
            Ok(h) => {
                let mut count = 0;
                for name in h.iter() {
                    let child_h = match h.open_child(&name)
                        {
                        Ok(child_h) => child_h,
                        Err(e) => panic!("`ls` failed to open child {:?} of {:?}", name, dir),
                        };
                    println!("{:?}: {:?}", name, child_h.get_class());
                    count += 1;
                }
                println!("{:?}: {} entries", dir, count);
                },
            }
            },
        // Create a directory
        "mkdir" => {
            let dir = ::kernel::vfs::Path::new( args.next().expect("mkdir dir") );
            let dirname = args.next().expect("mkdir newname");
            log_log!("COMMAND: mkdir {:?} {:?}", dir, dirname);
            let h = match ::kernel::vfs::handle::Dir::open(dir)
                {
                Ok(h) => h,
                Err(e) => {
                    log_error!("'{:?}' cannot be opened: {:?}", dir, e);
                    continue
                    },
                };
            match h.mkdir(dirname)
            {
            Ok(_) => {},
            Err(e) => log_error!("cannot create {:?} in '{:?}': {:?}", dirname, dir, e),
            }
            },
        // Copy a file from local to remote
        "store" => {
            let src: &::std::path::Path = args.next().expect("`store` src").as_ref();
            let dst: &::kernel::vfs::Path = args.next().expect("`store` dst").as_ref();
            let (dst_dir,dst_name) = dst.split_off_last().expect("`store` dst invalid");

            let mut src_handle = match ::std::fs::File::open(src)
                {
                Ok(h) => h,
                Err(e) => panic!("`store`: Cannot open source file {}: {:?}", src.display(), e),
                };
            let parent_handle = match ::kernel::vfs::handle::Dir::open(dst_dir)
                {
                Ok(h) => h,
                Err(e) => panic!("`store`: Cannot open parent directory of {:?}: {:?}", dst, e),
                };
            let dst_handle = match parent_handle.create_file(dst_name)
                {
                Ok(h) => h,
                Err(::kernel::vfs::Error::AlreadyExists) => match parent_handle.open_child(dst_name)
                    {
                    Ok(h) => match h.into_file(::kernel::vfs::handle::FileOpenMode::ExclRW)
                        {
                        Ok(h) => {
                            h.truncate();
                            h
                            },
                        Err(e) => panic!("`store`: Cannot create {:?}: {:?}", dst, e),
                        },
                    Err(e) => panic!("`store`: Cannot open existing {:?}: {:?}", dst, e),
                    },
                Err(e) => panic!("`store`: Cannot create {:?}: {:?}", dst, e),
                };

            let mut ofs = 0;
            let mut buf = vec![0; 1024];
            loop
            {
                use std::io::Read;
                match src_handle.read(&mut buf)
                {
                Ok(0) => break,
                Ok(l) => {
                    match dst_handle.write(ofs, &buf[..l])
                    {
                    Ok(v) if v == l => {},
                    Ok(v) => panic!("`store`: Failed to write to {:?}: Truncated? {} != exp {}", dst, v, l),
                    Err(e) => panic!("`store`: Failed to write to {:?}: {:?}", dst, e),
                    }
                    ofs += l as u64;
                    },
                Err(e) => panic!("`store`: IO failure reading from local: {:?}", e),
                }
            }
            },
        // Read a file and check that it's identical to the on-system version
        "readback" => {
            let local: &::std::path::Path = args.next().expect("`readback` local").as_ref();
            let remote: &::kernel::vfs::Path = args.next().expect("`readback` remote").as_ref();

            let mut local_handle = match ::std::fs::File::open(local)
                {
                Ok(h) => h,
                Err(e) => panic!("`readback`: Cannot open local file {}: {:?}", local.display(), e),
                };
            let remote_handle = match vfs_handle::File::open(remote, vfs_handle::FileOpenMode::SharedRO)
                {
                Ok(h) => h,
                Err(e) => panic!("`readback`: Cannot open remote file {:?}: {:?}", remote, e),
                };
            let mut ofs = 0;
            let mut buf_l = vec![0; 1024];
            let mut buf_r = vec![0; 1024];
            loop
            {
                use std::io::Read;
                let len_l = match local_handle.read(&mut buf_l)
                    {
                    Ok(l) => l,
                    Err(e) => panic!("`readback`: IO failure reading from local: {:?}", e),
                    };
                let len_r = match remote_handle.read(ofs, &mut buf_r)
                    {
                    Ok(l) => l,
                    Err(e) => panic!("`readback`: IO failure reading from remote: {:?}", e),
                    };
                assert_eq!(len_l, len_r);
                if len_l == 0 {
                    break;
                }
                assert_eq!(buf_l[..len_l], buf_r[..len_r]);
                ofs += len_l as u64;
            }
            },
        cmd => todo!("Command {}", cmd),
        }
    }

    // TODO: Unmount all volumes
}

