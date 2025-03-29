
#[derive(Clone)]
pub struct StatusWindow(::std::rc::Rc<super::rich_text_ele::TextConsole>);
impl StatusWindow {
	pub fn new() -> (Self, impl ::wtk::Element) {
		let inner = ::std::rc::Rc::new(super::rich_text_ele::TextConsole::new(1024));
		inner.new_line();
		inner.append_text(0, "Hello world!");
		(StatusWindow(inner.clone()), inner,)
	}
	pub fn print_error(&self, server: &str, args: ::std::fmt::Arguments) {
		self.0.new_line();
		self.0.append_fmt(0, format_args!("[{}] ", server));
		self.0.append_fmt(0, args);
	}
}


#[derive(Clone)]
pub struct ChannelWindow(::std::rc::Rc<super::rich_text_ele::TextConsole>);

impl ChannelWindow {
	pub fn new(name: &[u8]) -> (Self, impl ::wtk::Element) {
		let _ = name;
		let inner = ::std::rc::Rc::new(super::rich_text_ele::TextConsole::new(1024));
		(ChannelWindow(inner.clone()), inner,)
	}
	fn get_time(&self) -> &'static str {
		"12:34"
	}
	pub fn set_topic(&self, topic: &[u8]) {
		let timestamp = self.get_time();
		let topic = String::from_utf8_lossy(topic);
		self.0.new_line();
		self.0.append_fmt(0, format_args!("{} [TOPIC] {}", timestamp, topic));
	}
	pub fn append_message(&self, nickname: &[u8], message: &[u8]) {
		let timestamp = self.get_time();
		let nickname = String::from_utf8_lossy(nickname);
		let message = String::from_utf8_lossy(message);
		let user_colour = ::wtk::Colour::theme_text_alt();
		// Append "{timestamp} <{flag}{username}> {message}" to this window
		self.0.new_line();
		self.0.append_fmt(0, format_args!("{} <", timestamp));
		self.0.append_fg_set(0, Some(user_colour));
		self.0.append_text(0, &nickname);
		self.0.append_fg_set(0, None);
		self.0.append_text(0, "> ");
		// TODO: Parse mIRC codes to colour the text
		self.0.append_text(0, &message);
	}
}