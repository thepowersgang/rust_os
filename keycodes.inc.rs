
#[derive(Copy,Clone,Debug,PartialEq)]
#[repr(u8)]
#[allow(dead_code)]
/// Same values as the USB HID protocol
pub enum KeyCode
{
	None,
	ErrorRollover,
	PostFail,
	ErrorUndefined,
	// 0x04 / 4
	A, B, C, D, E, F,
	G, H, I, J, K, L,
	M, N, O, P, Q, R,
	S, T, U, V, W, X,
	Y, Z,
	
	// 0x1E / 30
	Kb1, Kb2, Kb3, Kb4,
	Kb5, Kb6, Kb7, Kb8,
	Kb9, Kb0,
	
	Return,	// Enter
	Esc,	// Esc.
	Backsp,	// Backspace
	Tab,	// Tab
	Space,	// Spacebar
	Minus,	// - _
	Equals,	// = +
	SquareOpen,	// [ {
	SquareClose,	// ] }
	Backslash,	// \ |
	HashTilde,	// # ~ (Non-US)
	Semicolon,	// ; :
	Quote,	// ' "
	GraveTilde,	// Grave Accent, Tilde
	Comma,	// , <
	Period,	// . >
	Slash,	// / ?
	Caps,	// Caps Lock
	F1, F2,
	F3, F4,
	F5, F6,
	F7, F8,
	F9, F10,
	F11, F12,
	PrintScreen,
	ScrollLock,
	Pause,
	Insert,
	Home,
	PgUp,
	Delete,
	End,
	PgDn,
	RightArrow,
	LeftArrow,
	DownArrow,
	UpArrow,
	
	Numlock,
	KpSlash,
	KpStar,
	KpMinus,
	KpPlus,
	KpEnter,
	Kp1,
	Kp2,
	Kp3,
	Kp4,
	Kp5,
	Kp6,
	Kp7,
	Kp8,
	Kp9,
	Kp0,
	KpPeriod,
	
	NonUSBackslash,
	Application,	// Menu
	Power,
	KpEquals,
	
	F13, F14,
	F15, F16,
	F17, F18,
	F19, F20,
	F21, F22,
	F23, F24,
	Execute,
	Help,
	Menu,
	Select,
	Stop,
	Again,
	Undo,
	Cut,
	Copy,
	Paste,
	Find,
	Mute,
	VolUp,
	VolDn,
	LockingCaps,	// Physically toggles
	LogkingNum,
	LogkingScroll,
	KpComma,
	KpEqual,
	KbInt1,
	KbInt2,
	KbInt3,
	KbInt4,
	KbInt5,
	KbInt6,
	KbInt7,
	KbInt8,
	KbInt9,

	Lang1,
	Lang2,
	Lang3,
	Lang4,
	Lang5,
	Lang6,
	Lang7,
	Lang8,
	Lang9,

	AltErase,
	SysRq,
	Cancel,
	Clear,
	Prior,
	Return_,
	Separator,
	Out,
	Oper,
	// TODO: Define this void
	
	LeftCtrl = 0xE0,
	LeftShift,
	LeftAlt,
	LeftGui,	// Menu?
	RightCtrl,
	RightShift,
	RightAlt,
	RightGui
}

impl ::core::convert::From<u8> for KeyCode
{
	fn from(v: u8) -> KeyCode {
		// SAFE: Bounds checks performed internally.
		unsafe {
			if v <= KeyCode::Oper as u8 {
				::core::mem::transmute(v as u8)
			}
			else if v < 0xE0 {
				panic!("KeyCode::from - Out of range");
			}
			else if v <= KeyCode::RightGui as u8 {
				::core::mem::transmute(v as u8)
			}
			else {
				panic!("KeyCode::from - Out of range");
			}
		}
	}
}

