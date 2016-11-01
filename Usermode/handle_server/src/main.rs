// Tifflin OS - handle_server
// - By John Hodge (thePowersGang)
//
// handle_server/src/main.rs
//! Top level logic

#[macro_use]
extern crate syscalls;

extern crate loader;
extern crate handle_server;

use handle_server::protocol;

struct Connection
{
	name: String,
	channel: ::syscalls::ipc::RpcChannel,
}

fn main()
{
	// handle_server gets the read-write root handle for the session user
	let filesystem_root: ::syscalls::vfs::Dir = ::syscalls::threads::S_THIS_PROCESS.receive_object("RwRoot").expect("Failed to receive FS root");

	// Active handle set - pre-populated with connection to leader
	let mut handles = vec![
		Connection {
			name: String::from("Leader"),
			channel: ::syscalls::threads::S_THIS_PROCESS.receive_object("HsChan").expect("Failed to receive leader channel"),
		}
		];

	let mut waits: Vec<_> = handles.iter().map(|x| x.channel.wait_rx()).collect();

	loop
	{
		::syscalls::threads::wait(&mut waits, !0);
		for conn in handles.iter()
		{
			let (buffer, obj) = match conn.channel.try_receive()
				{
				Ok(v) => v,
				Err(::syscalls::ipc::RxError::NoMessage) => continue,
				Err(::syscalls::ipc::RxError::ConnectionClosed) => panic!("TODO: Handle connection loss"),
				};
			match protocol::Request::try_from(buffer)
			{
			// Request to open an executable
			Ok(protocol::Request::OpenExecutable(req)) => {
				// TODO: Search a set of paths and registered applications.
				let path = match req.name()
					{
					b"fileviewer" => b"/system/bin/fileviewer",
					_ => {
						conn.channel.send( protocol::RspError::new(0, "Unknown name").into() );
						continue
						},
					};
				match filesystem_root.open_child_path(path).and_then(|x| x.into_file(::syscalls::vfs::FileOpenMode::Execute))
				{
				Ok(fh) => {
					conn.channel.send_obj( protocol::RspOpenedFile::new(path).into(), fh );
					},
				Err(_) => {
					conn.channel.send( protocol::RspError::new(0, "Could not open executable file").into() );
					continue
					},
				}
				},
			// Request the user pick a file to open
			Ok(protocol::Request::PickFile(req)) => {
				// TODO: Spawn a "open file" dialog linked to the calling process
				unimplemented!();
				},
			Err(e) => {
				kernel_log!("NOTICE: Unknown request from '{}' - {}", conn.name, buffer[0]);
				conn.channel.send( protocol::RspError::new(0, "Unknown request").into() );
				},
			}
		}
	}
}
