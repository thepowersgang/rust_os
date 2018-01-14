#![feature(proc_macro)]	// allow defining non-derive proc macros

extern crate proc_macro;

use proc_macro::TokenStream;

#[proc_macro]
pub fn utf16(input: TokenStream) -> TokenStream
{
	let mut it = input.into_iter();

	let mut rv = Vec::new();
	loop
	{
		match it.next()
		{
		Some(::proc_macro::TokenTree { kind: ::proc_macro::TokenNode::Literal(l), .. }) => {
			let s = match literal_to_string(l)
				{
				Ok(s) => s,
				Err(l) => panic!("Unexpected token '{}'", l),
				};
			//println!("s = {:?}", s);
			for c in s.chars()
			{
				if c as u32 <= 0xFFFF {
					rv.push(::proc_macro::TokenNode::Literal(::proc_macro::Literal::u16(c as u32 as u16)));
					rv.push(::proc_macro::TokenNode::Op(',', ::proc_macro::Spacing::Alone));
				}
				else {
					let v = c as u32 - 0x1_0000;
					let hi = v >> 10;
					assert!(hi <= 0x3FF);
					let lo = v & 0x3FF;

					rv.push(::proc_macro::TokenNode::Literal(::proc_macro::Literal::u16(0xD800 + hi as u16)));
					rv.push(::proc_macro::TokenNode::Op(',', ::proc_macro::Spacing::Alone));
					rv.push(::proc_macro::TokenNode::Literal(::proc_macro::Literal::u16(0xDC00 + lo as u16)));
					rv.push(::proc_macro::TokenNode::Literal(::proc_macro::Literal::u16(c as u32 as u16)));
					rv.push(::proc_macro::TokenNode::Op(',', ::proc_macro::Spacing::Alone));
				}
			}
			},
		Some(t) => panic!("Unexpected token '{}'", t),
		None => panic!("utf16! requires a string literal argument"),
		}


		match it.next()
		{
		Some(::proc_macro::TokenTree { kind: ::proc_macro::TokenNode::Op(',', _), .. }) => {},
		Some(t) => panic!("Unexpected token '{}'", t),
		None => break,
		}
	}
	//println!("{:?}", rv);

	vec![
		::proc_macro::TokenNode::Op('&', ::proc_macro::Spacing::Alone),
		::proc_macro::TokenNode::Group(::proc_macro::Delimiter::Bracket, rv.into_iter().collect()),
		].into_iter().collect()
}

fn literal_to_string(lit: ::proc_macro::Literal) -> Result<String,::proc_macro::Literal>
{
	let formatted = lit.to_string();
	
	let mut it = formatted.chars();
	if it.next() != Some('"') {
		return Err(lit);
	}

	let mut rv = String::new();
	loop
	{
		match it.next()
		{
		Some('"') =>
			match it.next()
			{
			Some(v) => panic!("malformed string, stray \" in the middle (followed by '{:?}')", v),
			None => break,
			},
		Some('\\') =>
			match it.next()
			{
			Some('0') => rv.push('\0'),
			Some('\\') => rv.push('\\'),
			Some(c) => panic!("TODO: Escape sequence \\{:?}", c),
			None => panic!("malformed string, unexpected EOS (after \\)"),
			},
		Some(c) => rv.push(c),
		None => panic!("malformed string, unexpected EOS"),
		}
	}

	Ok(rv)
}

