
use ::kernel::{log,log_error,log_log};

mod virt_storage;

fn main()
{
    ::kernel::threads::init();
    ::kernel::memory::phys::init();
    ::kernel::memory::page_cache::init();
    (::kernel::metadevs::storage::S_MODULE.init)();
    (::kernel::hw::mapper_mbr::S_MODULE.init)();
    (::kernel::vfs::S_MODULE.init)();

    // --
    (::fs_fat::S_MODULE.init)();
    (::fs_ext_n::S_MODULE.init)();
    // --
    
    // 1. Load disks (physical volumes)
    struct DiscDesc<'a> {
        name: &'a str,
        path: &'a ::std::path::Path,
        overlay: virt_storage::OverlayType,
    }
    struct VolumeDesc<'a> {
        mountpoint: &'a str,
        volume_name: &'a str,
        filesystem: &'a str,
        options: Vec<&'a str>,
    }
    let disk_defs = [
        DiscDesc { name: "virt0", path: "data/hda.img".as_ref(), overlay: virt_storage::OverlayType::None },
        ];
    let volume_defs = [
        VolumeDesc { mountpoint: "/system", volume_name: "virt0p0", filesystem: "", options: vec![] },
        ];
    

    let _volume_handles: Vec<_> = disk_defs.iter()
        .map(|d|
            match crate::virt_storage::add_volume(d.name, d.path, d.overlay)
            {
            Ok(h) => h,
            Err(e) => panic!("Unable to open {} as {}: {:?}", d.path.display(), d.name, e),
            })
        .collect();

    // 2. Mount
    for mount_desc in volume_defs.iter()
    {
        let vh = match ::kernel::metadevs::storage::VolumeHandle::open_named(mount_desc.volume_name)
            {
            Ok(vh) => vh,
            Err(e) => {
                panic!("Unable to open {}: {}", mount_desc.volume_name, e);
                },
            };
        match ::kernel::vfs::mount::mount(mount_desc.mountpoint.as_ref(), vh, mount_desc.filesystem, &mount_desc.options)
        {
        Ok(_) => {},
        Err(e) => {
            panic!("Unable to mount {} from {}: {:?}", mount_desc.mountpoint, mount_desc.volume_name, e);
            },
        }
    }

    // 3. Run commands
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
            match ::kernel::vfs::handle::Dir::open(dir)
            {
            Err(e) => log_error!("'{:?}' cannot be opened: {:?}", dir, e),
            Ok(h) =>
                for name in h.iter() {
                    println!("{:?}", name);
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
                    Ok(h) => match h.to_file(::kernel::vfs::handle::FileOpenMode::ExclRW)
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
        cmd => todo!("Command {}", cmd),
        }
    }

    // TODO: Unmount all volumes
}

