
pub use surface::Colour;
pub use geom::Rect;

#[derive(Default)]
pub struct TextInput<'a>
{
	state: State,
	shadow: String,
	obscure_char: Option<char>,
	submit_cb: Option< Box<FnMut(&TextInput<'a>)+'a> >,
}

#[derive(Default)]
struct State
{
	value: String,
	insert_ofs: usize,
	view_ofs: usize,
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

	pub fn get_content(&self) -> &str {
		panic!("TODO: TextInput::get_content");
	}
}

impl<'a> ::Element for TextInput<'a>
{
	fn render(&self, surface: ::surface::SurfaceView) {
		surface.fill_rect( Rect::new(0,0,!0,1), Colour::white() );
		surface.fill_rect( Rect::new(0,surface.height()-1,!0,1), Colour::white() );
		surface.fill_rect( Rect::new(0,0,1,!0), Colour::white() );
		surface.fill_rect( Rect::new(surface.width()-1,0,1,!0), Colour::white() );
		let pos = Rect::new(1,1,!0,!0);
		if self.state.value == ""
		{
			// Render shadow text in a desaturated colour
			surface.draw_text( pos, self.shadow.chars(), Colour::gray() );
		}
		else if let Some(ch) = self.obscure_char
		{
			// Render obscured
			surface.draw_text( pos, self.state.value.chars().map(|_| ch), Colour::white() );
		}
		else
		{
			// Render plain
			surface.draw_text( pos, self.state.value.chars(), Colour::white() );
		}
	}
}

