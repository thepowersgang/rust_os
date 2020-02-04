
use ::kernel::{log,log_error};

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
    (::fs_extN::S_MODULE.init)();
    
    let disks: [(&str, &::std::path::Path); 1] = [
        ("virt0", "data/hda.img".as_ref()),
        ];
    for (name, disk) in disks.iter()
    {
        match crate::virt_storage::add_volume(name, disk)
        {
        Ok( () ) => (),
        Err(e) => panic!("Unable to open {} as {}: {:?}", disk.display(), name, e),
        }
    }

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

    let mut cmd_stream = ::std::io::stdin();
    loop
    {
        let mut s = String::new();
        cmd_stream.read_line(&mut s).expect("Reading user input");

        // Parse using cmdline_words_parser
        let mut args = ::cmdline_words_parser::StrExt::parse_cmdline_words(&mut s);
        let cmd = match args.next()
            {
            None => break,
            Some("") => break,
            Some(v) => v,
            };
        match cmd
        {
        "ls" => {
            let dir = ::kernel::vfs::Path::new( args.next().expect("ls dir") );
            match ::kernel::vfs::handle::Dir::open(dir)
            {
            Err(e) => log_error!("'{:?}' cannot be opened: {:?}", dir, e),
            Ok(h) =>
                for name in h.iter() {
                    println!("{:?}", name);
                },
            }
            },
        _ => todo!("Command {}", cmd),
        }
    }
}