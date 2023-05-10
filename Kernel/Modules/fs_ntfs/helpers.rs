
#[derive(Copy,Clone,Debug)]
pub struct MftEntryIdx(pub u32);
impl ::core::fmt::Display for MftEntryIdx {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		write!(f, "E{}", self.0)
	}
}
