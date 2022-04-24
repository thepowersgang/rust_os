
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

    (::fs_fat::S_MODULE.init)();
    (::fs_ext_n::S_MODULE.init)();
    
    // 1. Load disks (physical volumes)
    let disks: [(&str, &::std::path::Path); 1] = [
        ("virt0", "data/hda.img".as_ref()),
        ];
    let mut volumes = vec![];
    for (name, disk) in disks.iter()
    {
        match crate::virt_storage::add_volume(name, disk, virt_storage::OverlayType::None)
        {
        Ok(h) => volumes.push(h),
        Err(e) => panic!("Unable to open {} as {}: {:?}", disk.display(), name, e),
        }
    }

    // 2. Mount
    let volumes: [(&str, &str, &str, &[&str]); 1] = [
        ("/system", "virt0p0", "", &[]),
        ];
    for (mount, volname, fs, opts) in volumes.iter()
    {
        let vh = match ::kernel::metadevs::storage::VolumeHandle::open_named(volname)
            {
            Ok(vh) => vh,
            Err(e) => {
                panic!("Unable to open {}: {}", volname, e);
                },
            };
        match ::kernel::vfs::mount::mount(mount.as_ref(), vh, fs, opts)
        {
        Ok(_) => {},
        Err(e) => {
            panic!("Unable to mount {} from {}: {:?}", mount, volname, e);
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
        cmd => todo!("Command {}", cmd),
        }
    }

    // TODO: Unmount all volumes
}