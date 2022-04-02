/*
 * VirtIO input device support
 */
//use kernel::prelude::*;
use crate::interface::Interface;
use kernel::threads::WorkerThread;
use kernel::lib::byte_str::ByteStr;
use gui::input::keyboard as gui_keyboard;
use gui::input::keyboard::KeyCode;

/// Device instance (as stored by the device manager)
pub struct InputDevice<I>
where
	I: Interface + Send + Sync
{
	_pd: ::core::marker::PhantomData<I>,
	_worker: WorkerThread,
}
impl<I> ::kernel::device_manager::DriverInstance for InputDevice<I>
where
	I: Interface + Send + Sync
{
}

impl<I> InputDevice<I>
where
	I: 'static + Interface + Send + Sync
{
	pub fn new(mut int: I) -> Self
	{
		let mut cfg_buf = [0; 128];
		log_debug!("CFG Name   = {:?}", ByteStr::new(Self::read_config(&mut int, VIRTIO_INPUT_CFG_ID_NAME  , 0, &mut cfg_buf)));
		log_debug!("CFG Serial = {:?}", ByteStr::new(Self::read_config(&mut int, VIRTIO_INPUT_CFG_ID_SERIAL, 0, &mut cfg_buf)));
		log_debug!("CFG DevIDs = {:x?}", Self::read_config(&mut int, VIRTIO_INPUT_CFG_ID_DEVIDS, 0, &mut cfg_buf));
		log_debug!("CFG Props  = {:x?}", Self::read_config(&mut int, VIRTIO_INPUT_CFG_PROP_BITS, 0, &mut cfg_buf));
		log_debug!("CFG Events = {:x?}", Self::read_config(&mut int, VIRTIO_INPUT_CFG_EV_BITS, 0, &mut cfg_buf));
		// No features
		int.set_driver_ok();

		let guidev = gui_keyboard::Instance::new();
		let eventq = int.get_queue(0, 0).expect("Queue #0 'eventq' missing on virtio input device");
		int.bind_interrupt(eventq.check_interrupt_fn());
		//let statusq = int.get_queue(1, 0).expect("Queue #1 'statusq' missing on virtio input device");
		let worker = WorkerThread::new("virtio-input", move || {
			eventq.into_stream(&int, /*item_size*/8, /*count*/16, |ev| {
				log_debug!("ev = {:x?}", ev);
				let ty    = u16::from_le_bytes([ev[0], ev[1]]);
				let code  = u16::from_le_bytes([ev[2], ev[3]]);
				let value = u32::from_le_bytes(::core::convert::TryInto::try_into(&ev[4..8]).unwrap());
				match ty
				{
				0/*EV_SYN*/ => {},
				1/*EV_KEY*/ => if let Some(&kc) = KEYMAP.get(code as usize)
					{
						if kc == KeyCode::None {
						}
						else if value != 0 {
							guidev.press_key(kc);
						}
						else {
							guidev.release_key(kc);
						}
					},
				_ => {},
				}
				let _ = guidev;
				});
			});
		Self {
			_pd: Default::default(),
			_worker: worker
			}
	}

	fn read_config<'a>(int: &mut I, id: virtio_input_config_select, subsel: u8, buf: &'a mut [u8; 128]) -> &'a [u8]
	{
		// SAFE: Writing to writable fields, unique access
		unsafe {
			int.cfg_write_8(0, id as u8);
			int.cfg_write_8(1, subsel);
			let len = int.cfg_read_8(2) as usize;
			assert!(len <= 128);
			for i in 0 .. len
			{
				buf[i] = int.cfg_read_8(8 + i);
			}
			&buf[..len]
		}
	}
}

use self::virtio_input_config_select::*;
#[repr(u8)]
#[allow(non_camel_case_types,dead_code)]
enum virtio_input_config_select
{
	VIRTIO_INPUT_CFG_UNSET     = 0x00,
	VIRTIO_INPUT_CFG_ID_NAME   = 0x01,
	VIRTIO_INPUT_CFG_ID_SERIAL = 0x02,
	VIRTIO_INPUT_CFG_ID_DEVIDS = 0x03,
	VIRTIO_INPUT_CFG_PROP_BITS = 0x10,
	VIRTIO_INPUT_CFG_EV_BITS   = 0x11,
	VIRTIO_INPUT_CFG_ABS_INFO  = 0x12,
}

use ::gui::input::keyboard::KeyCode::*;
static KEYMAP: [KeyCode; 120] = [
	KeyCode::None,
	Esc, Kb1, Kb2, Kb3, Kb4, Kb5, Kb6, Kb7, Kb8, Kb9, Kb0, Minus, Equals, Backsp,
	Tab, Q, W, E, R, T, Y, U, I, O, P, SquareOpen, SquareClose, Return,
	LeftCtrl, A, S, D, F, G, H, J, K, L, Semicolon, Quote, GraveTilde,
	LeftShift, Backslash, Z, X, C, V, B, N, M, Comma, Period, Slash, RightShift,

	KpStar, LeftAlt, Space, Caps,
	F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, Numlock, ScrollLock,
	Kp7, Kp8, Kp9, KpMinus,
	Kp4, Kp5, Kp6, KpPlus,
	Kp1, Kp2, Kp3,
	Kp0, KpPeriod,
	KeyCode::None,	// 84 = <empty>
	KeyCode::None,	// KEY_ZENKAKUHANKAKU
	KeyCode::None,	// KEY_102ND
	F11, F12,
	KeyCode::None,	// KEY_RO
	KeyCode::None,	// KEY_KATAKANA
	KeyCode::None,	// KEY_HIRAGANA
	KeyCode::None,	// KEY_HENKAN
	KeyCode::None,	// KEY_KATAKANAHIRAGANA
	KeyCode::None,	// KEY_MUHENKAN
	KeyCode::None,	// KEY_MUHENKAN
	KpEnter,
	RightCtrl,
	KpSlash,
	SysRq,
	RightAlt,
	KeyCode::None,	// KEY_LINEFEED
	Home,
	UpArrow,
	PgUp,
	LeftArrow,
	RightArrow,
	End,
	DownArrow,
	PgDn,
	Insert,
	Delete,
	KeyCode::None,	// KEY_MACRO
	Mute,
	VolDn,
	VolUp,
	Power,
	KpEqual,
	KeyCode::None,	// KEY_KPPLUSMINUS
	Pause,
	// ... a whole lot more
	];
//const _: [(); (KEYMAP[119] == KeyCode::Pause) as usize - 1] = [];

