//
//
//
///
use prelude::*;
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
}


/*
NOTES
====

Initially (as in, as soon as possible), use the boot-provided video mode

When another driver registers, "forget" this information and use the new driver.

TODO: Replace metadevs::video with this code (direct passthrough shim)
TODO: Write up API for video surfaces (with blitting to/from them)
*/

