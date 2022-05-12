
pub struct Mouse
{
    // Store x/y values (relative/absolute)
    is_relative: bool,
    x_value: u16,
    y_value: u16,

    // Button states
    cur_buttons: u16,
    prev_buttons: u16,

    gui_handle: ::gui::input::mouse::Instance,
}
impl Mouse
{
    pub fn new() -> Mouse
    {
        Mouse {
            is_relative: false,
            x_value: 0, y_value: 0,
            cur_buttons: 0,
            prev_buttons: 0,
            gui_handle: ::gui::input::mouse::Instance::new(),
            }
    }

    pub fn abs_x(&mut self, v: u16) {
        self.x_value = v;
        self.is_relative = false;
    }
    pub fn abs_y(&mut self, v: u16) {
        self.y_value = v;
        self.is_relative = false;
    }
    pub fn rel_x(&mut self, d: i16) {
        self.x_value = d as u16;
        self.is_relative = true;
    }
    pub fn rel_y(&mut self, d: i16) {
        self.y_value = d as u16;
        self.is_relative = true;
    }

    pub fn set_button(&mut self, idx: usize, is_pressed: bool) {
        if is_pressed && idx < 16 {
            self.cur_buttons |= 1 << idx;
        }
    }

    pub fn updated(&mut self) {
        // Update positions
        if self.is_relative {
            self.gui_handle.move_cursor(self.x_value as i16, self.y_value as i16);
        }
        else {
            // NOTE: Should already be normalised
            self.gui_handle.set_cursor(self.x_value, self.y_value);
        }
        // Update buttons
        for i in 0 .. 16
        {
            let cur  = (self.cur_buttons  & 1 << i) != 0;
            let prev = (self.prev_buttons & 1 << i) != 0;

            if cur != prev
            {
                if cur {
                    self.gui_handle.press_button(i);
                }
                else {
                    self.gui_handle.release_button(i);
                }
            }
        }
        self.prev_buttons = self.cur_buttons;
        self.cur_buttons = 0;
    }
}
