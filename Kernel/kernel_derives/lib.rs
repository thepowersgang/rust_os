
use proc_macro::TokenStream;
use syn::DeriveInput;

#[proc_macro_derive(EncodedLE)]
pub fn derive_encoded_le(item: TokenStream) -> TokenStream {
	derive_encoded(item, "EncodedLE")
}
#[proc_macro_derive(EncodedBE)]
pub fn derive_encoded_be(item: TokenStream) -> TokenStream {
	derive_encoded(item, "EncodedBE")
}
fn derive_encoded(item: TokenStream, trait_name: &str) -> TokenStream {
	let input = ::syn::parse_macro_input!(item as DeriveInput);
	let ::syn::Data::Struct(data) = input.data else {
		panic!("Cannot derive `{}` on a non-struct", trait_name);
		};
	let typename = input.ident;
	let fields: Vec<_> = match data.fields
		{
		::syn::Fields::Named(fields) => fields.named.into_pairs().map(|v| v.into_value()).collect(),
		_ => panic!("Can only derive `{}` on a named struct", trait_name),
		};
	let trait_name = ::quote::format_ident!("{}", trait_name);
	let field_names: Vec<_> = fields.iter().map(|v| v.ident.as_ref().expect("Unanmed field in Named struct?")).collect();
	TokenStream::from(::quote::quote!{
		impl ::kernel::lib::byteorder::#trait_name for #typename {
			fn encode(&self, buf: &mut &mut [u8]) -> ::kernel::lib::byteorder::Result<()> {
				#( ::kernel::lib::byteorder::#trait_name::encode( &self.#field_names, buf )?; )*
				Ok( () )
			}
			fn decode(buf: &mut &[u8]) -> ::kernel::lib::byteorder::Result<Self> {
				Ok(Self {
					#( #field_names: ::kernel::lib::byteorder::#trait_name::decode(buf)?, )*
				})
			}
		}
	})
}

