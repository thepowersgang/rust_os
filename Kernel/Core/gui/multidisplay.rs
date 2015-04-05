use _common::*;
use super::{Dims,Pos};

/// State of the "virtual display"
///
/// The virtual display handles mapping a virtual canvas (which may have holes) to the actual
/// physical screens.
struct VirtualDisplayState
{
	total_dims: Dims,
	surfaces: Vec<VDSurface>,
}
/// A single backing surface for VirtualDisplayState
struct VDSurface
{
	/// Position relative to the top-left
	position: Pos,
	//backing: ::metadevs::video::Display,
}

