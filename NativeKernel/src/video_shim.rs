//!
//!
//!
use ::kernel::metadevs::video;


/// Display surface backed by a window
pub struct Display
{
    size: video::Dims,
}
impl Display
{
    pub fn new() -> Self
    {
        Display {
            size: video::Dims::new(640, 480),
			// TODO: Use minifb? or a raw bitmap?
        }   
    }
}
impl video::Framebuffer for Display
{
	fn as_any(&self) -> &dyn std::any::Any {
        self
    }
	fn activate(&mut self) {
        // Anything?
    }
	
	fn get_size(&self) -> video::Dims {
        self.size
    }
	fn set_size(&mut self, newsize: video::Dims) -> bool  {
        todo!("set_size: {:?}", newsize)
    }
	
	fn blit_inner(&mut self, dst: video::Rect, src: video::Rect) {
        todo!("")
    }
	fn blit_ext(&mut self, dst: video::Rect, src: video::Rect, srf: &dyn video::Framebuffer) -> bool {
        todo!("")
    }
	fn blit_buf(&mut self, dst: video::Rect, buf: video::StrideBuf<'_,u32>) {
        //todo!("")
    }
	fn fill(&mut self, dst: video::Rect, colour: u32) {
        todo!("")
    }
	
	fn move_cursor(&mut self, p: Option<video::Pos>) {
        todo!("")
    }
}
