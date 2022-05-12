
pub struct Keyboard
{
    cur_state: BitSet256,
    last_state: BitSet256,
    gui_handle: ::gui::input::keyboard::Instance,
}
impl Keyboard
{
    pub fn new() -> Self {
        Keyboard {
            cur_state: BitSet256::new(),
            last_state: BitSet256::new(),
            gui_handle: ::gui::input::keyboard::Instance::new(),
            }
    }
    pub fn set_key(&mut self, k: u8)
    {
        self.cur_state.set( k as usize );
    }
    pub fn updated(&mut self) {
        for i in 0 .. 256
        {
            let cur = self.cur_state.get(i);
            let prev = self.last_state.get(i);

            if cur != prev
            {
                let k = match ::gui::input::keyboard::KeyCode::try_from( i as u8 )
                    {
                    Some(k) => k,
                    None => {
                        log_notice!("Bad key code: {:02x}", i);
                        continue
                        },
                    };

                if cur {
                    self.gui_handle.press_key(k);
                }
                else {
                    self.gui_handle.release_key(k);
                }
            }
        }
        self.last_state = ::core::mem::replace(&mut self.cur_state, BitSet256::new());
    }
}
struct BitSet256([u8; 256/8]);
#[allow(dead_code)]
impl BitSet256
{
    pub fn new() -> Self {
        BitSet256([0; 256/8])
    }
    pub fn get(&self, i: usize) -> bool {
        if i >= 256 {
            return false;
        }
        self.0[i / 8] & 1 << (i%8) != 0
    }
    pub fn set(&mut self, i: usize) {
        if i < 256 {
            self.0[i / 8] |= 1 << (i%8);
        }
    }
    pub fn clr(&mut self, i: usize) {
        if i < 256 {
            self.0[i / 8] &= !(1 << (i%8));
        }
    }
}
impl ::core::ops::BitXor for &'_ BitSet256
{
    type Output = BitSet256;
    fn bitxor(self, other: &BitSet256) -> BitSet256
    {
        let mut rv = BitSet256::new();
        for (d,(a,b)) in Iterator::zip( rv.0.iter_mut(), Iterator::zip(self.0.iter(), other.0.iter()) )
        {
            *d = *a ^ *b;
        }
        rv
    }
}