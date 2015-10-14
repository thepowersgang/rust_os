//
//
//
//! Simple text input field
pub use surface::Colour;
pub use geom::Rect;

#[derive(Default)]
/// A single-line text input widget, supporting shadow text and optionally obscured input
pub struct TextInput<'a>
{
	state: ::std::cell::RefCell<State>,
	shadow: String,
	obscure_char: Option<char>,
	submit_cb: Option< Box<Fn(&TextInput<'a>, &mut ::window::Window)+'a> >,
}

#[derive(Default)]
struct State
{
	value: String,
	//insert_ofs: usize,
	//view_ofs: usize,

	is_dirty: bool,
	is_focussed: bool,
}


impl<'a> TextInput<'a>
{
	/// Create a new input widget
	pub fn new() -> TextInput<'a> {
		Default::default()
	}

	/// Set the "shadow" text (text shown when nothing has been entered)
	pub fn set_shadow<T: Into<String>>(&mut self, text: T) {
		self.shadow = text.into();
	}
	/// Set the obsucuring character
	pub fn set_obscured(&mut self, replacement: char) {
		self.obscure_char = Some(replacement);
	}

	/// Set a function to be called when "enter" is pressed
	/// 
	/// Closure is passed a shared handle to this widget, and a mutable handle to the owning
	/// window.
	pub fn bind_submit<F: Fn(&Self, &mut ::window::Window)+'a>(&mut self, cb: F) {
		self.submit_cb = Some( Box::new(cb) );
	}

	/// Returns the inner content of this text box
	pub fn get_content(&self) -> /* impl Deref<Target=str>+Display*/ Content {
		Content(self.state.borrow())
	}

	pub fn clear(&self) {
		self.state.borrow_mut().value.clear();
	}
}

/// Borrow of a `TextInput` widget's content
pub struct Content<'a>(::std::cell::Ref<'a, State>);
impl<'a> ::std::ops::Deref for Content<'a> {
	type Target = str;
	fn deref(&self) -> &str { &self.0.value }
}
impl<'a> ::std::fmt::Display for Content<'a> {
	fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
		::std::fmt::Display::fmt( &self.0.value, f )
	}
}

impl<'a> ::Element for TextInput<'a>
{
	// On focus change, update flag used to render the cursor
	fn focus_change(&self, have: bool) {
		let mut state = self.state.borrow_mut();
		state.is_focussed = have;
		state.is_dirty = true;
	}

	fn render(&self, surface: ::surface::SurfaceView, force: bool) {
		let mut state = self.state.borrow_mut();
		if force || state.is_dirty {
			let (w,h) = (surface.width(), surface.height());
			// A basic raised border (top-left illuminated)
			// TODO: Shouldn't the border be the job of a wrapping frame? Kinda heavy, but cleaner
			//surface.fill_rect( Rect::new(0,0,w,1), Colour::theme_border_main() );
			//surface.fill_rect( Rect::new(0,0,1,h), Colour::theme_border_main() );
			//surface.fill_rect( Rect::new(0,h-1,w,1), Colour::theme_border_alt() );
			//surface.fill_rect( Rect::new(w-1,0,1,h), Colour::theme_border_alt() );

			surface.fill_rect( Rect::new(0,0,w,h), Colour::theme_text_bg() );
			// Text positioned 1px from the corners
			let pos = Rect::new(1,1,w-2,h-2);
			// TODO: Support interior editing (and have the cursor midway)
			let cursor_pos =
				if state.value == ""
				{
					// Render shadow text in a desaturated colour
					surface.draw_text( pos, self.shadow.chars(), Colour::theme_text_alt() );
					0
				}
				else if let Some(ch) = self.obscure_char
				{
					// Render obscured
					surface.draw_text( pos, state.value.chars().map(|_| ch), Colour::theme_text() )
				}
				else
				{
					// Render plain
					surface.draw_text( pos, state.value.chars(), Colour::theme_text() )
				};
			// If focused, render a cursor at the insert position.
			// - Vertical line from 2px to -2px
			if state.is_focussed {
				surface.fill_rect( Rect::new(1 + cursor_pos as u32, 2,  1, h-4), Colour::theme_text() );
			}

			state.is_dirty = false;
		}
	}

	fn handle_event(&self, ev: ::InputEvent, win: &mut ::window::Window) -> bool {
		match ev
		{
		::InputEvent::Text(v) => {
			let mut state = self.state.borrow_mut();
			state.value.push_str(&v);
			state.is_dirty = true;
			true
			},
		::InputEvent::KeyUp(::syscalls::gui::KeyCode::Backsp) => {
			let mut state = self.state.borrow_mut();
			state.value.pop();	// TODO: Should really pop a grapheme
			state.is_dirty = true;
			true
			},
		::InputEvent::KeyUp(::syscalls::gui::KeyCode::Return) =>
			if let Some(ref cb) = self.submit_cb
			{
				cb(self, win);
				true
			}
			else {
				false
			},
		_ => false,
		}
	}
	fn element_at_pos(&self, _x: u32, _y: u32) -> (&::Element, (u32,u32)) {
		(self, (0,0))
	}
}

