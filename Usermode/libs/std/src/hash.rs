
pub trait Hasher
{
}

pub trait Hash: Sized
{
	fn hash<H: Hasher>(&self, state: &mut H);
	fn hash_slice<H: Hasher>(data: &[Self], state: &mut H) {
		for v in data {
			v.hash(state);
		}
	}
}


impl Hash for () {
	fn hash<H: Hasher>(&self, _: &mut H) {}
}

//impl<T..> Hash for (T..) {
//	  fn hash<H: Hasher>(&self, state: &mut H) {
//	      
//	  }
//}
