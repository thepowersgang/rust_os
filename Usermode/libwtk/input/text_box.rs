
pub use surface::Colour;
pub use geom::Rect;

#[derive(Default)]
pub struct TextInput<'a>
{
	state: ::std::cell::RefCell<State>,
	shadow: String,
	obscure_char: Option<char>,
	submit_cb: Option< Box<Fn(&TextInput<'a>)+'a> >,
}

#[derive(Default)]
struct State
{
	value: String,
	insert_ofs: usize,
	view_ofs: usize,

	is_focussed: bool,
}


impl<'a> TextInput<'a>
{
	pub fn new() -> TextInput<'a> {
		Default::default()
	}

	/// Set the "shadow" text (text shown when nothing has been entered)
	pub fn set_shadow<T: Into<String>>(&mut self, text: T) {
		self.shadow = text.into();
	}
	pub fn set_obscured(&mut self, replacement: char) {
		self.obscure_char = Some(replacement);
	}

	pub fn bind_submit<F: Fn(&Self)+'a>(&mut self, cb: F) {
		self.submit_cb = Some( Box::new(cb) );
	}

	pub fn get_content(&self) -> String {
		self.state.borrow().value.clone()
	}
}

impl<'a> ::Element for TextInput<'a>
{
	fn focus_change(&self, have: bool) {
		self.state.borrow_mut().is_focussed = have;
	}
	fn render(&self, surface: ::surface::SurfaceView) {
		surface.fill_rect( Rect::new(0,0,!0,1), Colour::theme_border_main() );
		surface.fill_rect( Rect::new(0,surface.height()-1,!0,1), Colour::theme_border_main() );
		surface.fill_rect( Rect::new(0,0,1,!0), Colour::theme_border_alt() );
		surface.fill_rect( Rect::new(surface.width()-1,0,1,!0), Colour::theme_border_alt() );
		let pos = Rect::new(1,1,!0,!0);
		surface.fill_rect( pos, Colour::theme_text_bg() );
		let state = self.state.borrow();
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
		if state.is_focussed {
			surface.fill_rect( Rect::new(cursor_pos as u32, 2, 1, surface.height()-4), Colour::theme_text() );
		}
	}

	fn handle_event(&self, ev: ::InputEvent) -> bool {
		match ev
		{
		::InputEvent::Text(v) => {
			self.state.borrow_mut().value.push_str(&v);
			true
			},
		::InputEvent::KeyUp(::syscalls::gui::KeyCode::Backsp) => {
			self.state.borrow_mut().value.pop();	// TODO: Should really pop a grapheme
			true
			},
		::InputEvent::KeyUp(::syscalls::gui::KeyCode::Return) =>
			if let Some(ref cb) = self.submit_cb
			{
				cb(self);
				true
			}
			else {
				false
			},
		_ => false,
		}
	}
}

