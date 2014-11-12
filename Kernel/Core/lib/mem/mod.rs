
pub use self::rc::Rc;

mod rc;

#[lang = "owned_box"]
pub struct Box<T>(*mut T);

// vim: ft=rust

