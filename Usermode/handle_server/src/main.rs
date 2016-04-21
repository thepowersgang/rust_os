// Tifflin OS - handle_server
// - By John Hodge (thePowersGang)
//
// handle_server/src/main.rs
//! Top level logic

#[macro_use]
extern crate syscalls;

extern crate handle_server;

use handle_server::protocol;

struct Connection
{
	name: String,
	channel: ::syscalls::ipc::RpcChannel,
}

fn main()
{
	let filesystem_root: ::syscalls::vfs::Dir = ::syscalls::threads::S_THIS_PROCESS.receive_object().expect("Failed to receive FS root");

	let mut handles = vec![
		Connection {
			name: String::from("Leader"),
			channel: ::syscalls::threads::S_THIS_PROCESS.receive_object().expect("Failed to receive leader channel"),
		}
		];

	let mut waits: Vec<_> = handles.iter().map(|x| x.channel.wait_rx()).collect();

	loop
	{
		::syscalls::threads::wait(&mut waits, !0);
		for conn in handles.iter()
		{
			if let Ok( (buffer, obj) ) = conn.channel.try_receive()
			{
				match protocol::RequestId::try_from(buffer[0])
				{
				Some(protocol::RequestId::OpenExecutable) => {
					let req: protocol::RequestExecutable = buffer.into();
					// TODO: Search a set of paths and registered applications.
					let path = match req.name()
						{
						b"fileviewer" => b"/system/bin/fileviewer",
						_ => {
							conn.channel.send( protocol::RspError::new(0, "Unknown name").into() );
							return
							},
						};
					let fh = match filesystem_root.open_child_path(path).and_then(|x| x.into_file(::syscalls::vfs::FileOpenMode::Execute))
						{
						Ok(v) => v,
						Err(_) => {
							conn.channel.send( protocol::RspError::new(0, "Could not open executable file").into() );
							return
							},
						};
					conn.channel.send_obj( protocol::RspFile::new(path).into(), fh );
					},
				Some(protocol::RequestId::PickFile) => {
					// TODO: Spawn a "open file" dialog linked to the calling process
					unimplemented!();
					},
				None => {
					kernel_log!("NOTICE: Unknown request from '{}' - {}", conn.name, buffer[0]);
					conn.channel.send( protocol::RspError::new(0, "Unknown request").into() );
					},
				}
			}
		}
	}
}
