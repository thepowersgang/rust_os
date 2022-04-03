//!
//!
//!
use ::kernel::metadevs::video;
use ::gui::input::keyboard as gui_keyboard;
use ::gui::input::mouse as gui_mouse;
use ::gui::input::keyboard::KeyCode;
use ::std::sync::Arc;
use ::std::sync::Mutex;

/// Structure for the window-based console
pub struct Console
{
	_worker: ::std::thread::JoinHandle<()>,
	state: Arc<Mutex<State>>,
}
struct State
{
	/// Current size of the window
    size: video::Dims,
	/// Backing buffer
	backbuffer: Vec<u32>,
	/// Indicates that the buffer has changed since it was last sent to the window
	dirty: bool,
}
impl Console
{
    pub fn new() -> Arc<Self>
	{
		let size = video::Dims::new(1280, 768);
		let state = Arc::new(Mutex::new(State {
			size: size,
			backbuffer: ::std::vec![ 0u32; size.w as usize * size.h as usize ],
			dirty: true,
			}));

		enum InputEvent {
			KeyDown(KeyCode),
			KeyUp(KeyCode),
			MousePos(u16,u16),
			MouseDown(u8),
			MouseUp(u8),
		}

		let (input_tx, input_rx) = ::std::sync::mpsc::channel::<InputEvent>();
		// A kernel thread to handle sending keystrokes to the GUI
		let input_worker = ::kernel::threads::WorkerThread::new("GUI Input", move || {
			let gui_keyboard = gui_keyboard::Instance::new();
			let gui_mouse = gui_mouse::Instance::new();
			loop
			{
				let ev = ::kernel::arch::imp::threads::test_pause_thread(|| input_rx.recv().expect("Input sender dropped") );
				match ev
				{
				InputEvent::KeyDown(key) => gui_keyboard.press_key(key),
				InputEvent::KeyUp(key) => gui_keyboard.release_key(key),
				InputEvent::MousePos(x,y) => gui_mouse.set_cursor(x, y),
				InputEvent::MouseDown(btn) => gui_mouse.press_button(btn),
				InputEvent::MouseUp(btn) => gui_mouse.release_button(btn),
				}
			}
			});

		Arc::new(Console {
			state: state.clone(),
			_worker: ::std::thread::spawn(move || {
				let _ = input_worker;
				let size = state.lock().unwrap().size;
				let mut window = ::minifb::Window::new("RustOS Native", size.width() as usize, size.height() as usize, Default::default()).expect("Failed to spawn window");

				window.limit_update_rate(Some(::std::time::Duration::from_millis(16)));
				let mut prev_keys = ::std::vec![];
				let mut prev_pos = (0,0);
				let mut prev_mouse = [false; 3];
				loop {
					// NOTE: This should limit its update rate (if no events are present, waits for 16ms)
					window.update();

					{
						let mut lh = state.lock().unwrap();
						if ::std::mem::replace(&mut lh.dirty, false) {
							window.update_with_buffer(&lh.backbuffer, lh.size.width() as usize, lh.size.height() as usize)
								.expect("Failed to update window buffer contents");
						}
					}

					if ! window.is_open() {
						eprintln!("GUI window closed, exiting immediately");
						std::process::exit(0);
					}

					// TODO: Mouse handling
					if let Some( p ) = window.get_mouse_pos(::minifb::MouseMode::Discard)
					{
						let x = (p.0 / size.w as f32 * 0xFFFF as f32) as u16;
						let y = (p.1 / size.h as f32 * 0xFFFF as f32) as u16;
						if (x,y) != prev_pos {
							prev_pos = (x,y);
							input_tx.send(InputEvent::MousePos(x, y)).expect("Input worker quit");
						}
						for (i,&b) in [::minifb::MouseButton::Left, ::minifb::MouseButton::Middle, ::minifb::MouseButton::Right].iter().enumerate() {
							let s = window.get_mouse_down(b);
							if prev_mouse[i] != s {
								prev_mouse[i] = s;
								input_tx.send(if s { InputEvent::MouseDown(i as u8) } else { InputEvent::MouseUp(i as u8) }).expect("Input worker quit");
							}
						}
					}

					if let Some(mut keys) = window.get_keys()
					{
						if keys != prev_keys
						{
							log_debug!("keys = {:?} -> {:?}", prev_keys, keys);
						}

						fn translate_keycode(k: &::minifb::Key) -> KeyCode {
							use ::minifb::Key;
							macro_rules! keymap {
								( $($k:ident $(=> $k2:ident)? ,)* ) => {
									match k
									{
									$(Key::$k => keymap!(@sel $k $($k2)?) ,)*
									_ => KeyCode::None,
									}
								};
								(@sel $k:ident) => { KeyCode::$k };
								(@sel $k:ident $k2:ident) => { KeyCode::$k2 };
							}
							keymap!(
								A, B, C, D, E, F, G, H, I, J, K, L, M,
								N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
								Backquote => GraveTilde,
								Key1 => Kb1,
								Key2 => Kb2,
								Key3 => Kb3,
								Key4 => Kb4,
								Key5 => Kb5,
								Key6 => Kb6,
								Key7 => Kb7,
								Key8 => Kb8,
								Key9 => Kb9,
								Key0 => Kb0,
								Minus, Equal => Equals,
								LeftBracket => SquareOpen, RightBracket => SquareClose,
								Semicolon, Apostrophe => Quote,
								Comma, Period, Slash,
								Space,
								F1, F2, F3, F4,
								F5, F6, F7, F8,
								F9, F10, F11, F12,
								F13, F14, F15,
								// keycodes.inc.rs goes to F24, minifb only to F15

								Tab,
								Enter => Return,
								Backspace => Backsp,
								Escape => Esc,

								// Modifiers
								LeftShift,
								RightShift,
								LeftCtrl,
								RightCtrl,
								LeftAlt,
								RightAlt,
								
								Insert, Home, PageUp => PgUp,
								Delete, End, PageDown => PgDn,
								// Arrows etc
								Up => UpArrow,
								Down => DownArrow,
								Left => LeftArrow,
								Right => RightArrow,

								// Numpad
								NumLock => Numlock,
								NumPadSlash => KpSlash, 
								NumPadAsterisk => KpStar,
								NumPadMinus => KpMinus,
								NumPadPlus => KpPlus,
								NumPadEnter => KpEnter,
								NumPadDot => KpPeriod,
								NumPad0 => Kp0,
								NumPad1 => Kp1,
								NumPad2 => Kp2,
								NumPad3 => Kp3,
								NumPad4 => Kp4,
								NumPad5 => Kp5,
								NumPad6 => Kp6,
								NumPad7 => Kp7,
								NumPad8 => Kp8,
								NumPad9 => Kp9,
								)
						}

						// Get the difference between the two
						keys.sort();
						let mut it_p = prev_keys.iter();
						let mut it_n = keys.iter();

						let mut cur_p = it_p.next();
						let mut cur_n = it_n.next();
						loop
						{
							let (is_release, minifb_key) = match (cur_p, cur_n)
								{

								// `p` is released
								(Some(p), Some(n)) if p < n => {
									cur_p = it_p.next();
									(true, p)
									},
								(Some(p), None) => {
									cur_p = it_p.next();
									(true, p)
									},

								// `n` has been pressed
								(Some(p), Some(n)) if p > n => {
									cur_n = it_n.next();
									(false, n)
									},
								(None, Some(n)) => {
									cur_n = it_n.next();
									(false, n)
									},

								// Still pressed, don't send an event
								(Some(p), Some(n)) => {
									assert!( p == n );
									cur_p = it_p.next();
									cur_n = it_n.next();
									continue ;
									},

								// End of both lists, stop
								(None, None) => break,
								};
							let key = translate_keycode(minifb_key);
							if key == KeyCode::None {
								log_notice!("No translation for minifb::Key::{:?} -> gui::KeyCode", minifb_key)
							}
							else {
								input_tx.send( if is_release { InputEvent::KeyUp(key) } else { InputEvent::KeyDown(key) } ).expect("Input worker quit");
							}
						}
						
						prev_keys = keys;
					}
				}
				}),
			})
	}

    pub fn get_display(&self) -> Display
    {
        Display {
			state: self.state.clone(),
        }
    }
}

/// Display surface backed by a window
pub struct Display
{
	state: Arc<Mutex<State>>,
}
impl video::Framebuffer for Display
{
	fn as_any(&self) -> &dyn std::any::Any {
        self
    }
	fn activate(&mut self) {
        // Anything?
    }
	
	fn get_size(&self) -> video::Dims {
		self.state.lock().unwrap().size
    }
	fn set_size(&mut self, newsize: video::Dims) -> bool  {
        todo!("set_size: {:?}", newsize)
    }
	
	fn blit_inner(&mut self, dst: video::Rect, src: video::Rect) {
        todo!("blit_inner({:?}, {:?})", dst, src)
    }
	fn blit_ext(&mut self, dst: video::Rect, src: video::Rect, srf: &dyn video::Framebuffer) -> bool {
        todo!("blit_ext({:?}, {:?}, srf={:p})", dst, src, srf)
    }
	fn blit_buf(&mut self, dst: video::Rect, buf: video::StrideBuf<'_,u32>) {
		let mut lh = self.state.lock().unwrap();
		let backbuffer_w = lh.size.width() as usize;
		for (row,src) in ::kernel::lib::ExactZip::new( dst.top() .. dst.bottom(), buf.chunks(dst.w() as usize) )
		{
			//let seg = self.buffer.scanline_slice(row as usize, dst.left() as usize, dst.right() as usize);
			let seg = &mut lh.backbuffer[row as usize * backbuffer_w ..][dst.left() as usize .. dst.right() as usize];
			seg.copy_from_slice( src );
		}
		lh.dirty = true;
    }
	fn fill(&mut self, dst: video::Rect, colour: u32) {
        todo!("fill({:?}, {:#x})", dst, colour)
    }
	
	fn move_cursor(&mut self, _p: Option<video::Pos>) {
        //todo!("move_cursor({:?})", p)
    }
}
