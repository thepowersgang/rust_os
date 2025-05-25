
use crate::hw::structs::Trb;

pub struct CommandRing
{
	ring_page: ::kernel::memory::virt::AllocHandle,

	#[allow(dead_code)] // Never read, just storing
	scratchpad_entries: ::kernel::lib::Vec< ::kernel::memory::virt::AllocHandle>,
	#[allow(dead_code)] // Never read, just storing
	scratchpad_array: Option< ::kernel::memory::virt::ArrayHandle<u64> >,

	/// Offset of the first command
	base_offset: u8,
	/// Current enqueue offset
	offset: u8,
	/// Current enqueue cycle bit
	cycle_bit: bool,
}

const ENT_SIZE: usize = ::core::mem::size_of::<Trb>();
const MAX_OFFSET: u8 = (::kernel::PAGE_SIZE / ENT_SIZE - 1) as u8;

// See 4.9.3 "Command Ring Management"
// To send a command
// - Push a command to the command ring
//  > On each push, ensure that the `cycle_bit` matches the CCS value read from the controller (CRCR.RCS)
// - Write to the command doorbell (offset 0 in the doorbell registers)
impl CommandRing
{
	pub /*unsafe*/ fn new(regs: &crate::hw::Regs, n_device_slots: u8) -> Result<Self,::kernel::device_manager::DriverBindError> {
		let mut ring_page = ::kernel::memory::virt::alloc_dma(64, 1, "usb_xhci")?;

		//   - Set DCBAAP to the device context array
		//     > Entry zero points to an array of scratchpad pages, see the `Max Scratchpad Buffers Hi/Lo` fields in HCSPARAMS2 TODO check s4.20 of the spec
		let num_scratchpad_buffers = regs.max_scratchpad_buffers();
		let mut scratchpad_entries = ::kernel::lib::vec::Vec::with_capacity(num_scratchpad_buffers as usize);
		let mut scratchpad_array = None;
		let base_offset;
		if num_scratchpad_buffers > 0
		{
			// Max of 1023 buffers to be requested, which will require 8KB for the list
			// - If fewer than 128 buffers required, then stick after the dcbaa (leaving 31 command slots)
			// - Otherwise, they need to go in a separate page
			let array = if num_scratchpad_buffers < 128 {
					base_offset = (((256 + num_scratchpad_buffers as usize) * 8  + ENT_SIZE-1) / ENT_SIZE) as u8;
					ring_page.as_mut_slice(256*8, num_scratchpad_buffers as usize)
				}
				else {
					scratchpad_array = Some(::kernel::memory::virt::alloc_dma(64, 1, "usb_xhci")?.into_array());
					base_offset = ((256 * 8) / 32) as u8;
					&mut scratchpad_array.as_mut().unwrap()[..]
				};
			for i in 0 .. regs.max_scratchpad_buffers() as usize
			{
				let e = ::kernel::memory::virt::alloc_dma(64, 1, "usb_xhci")?;
				array[i] = ::kernel::memory::virt::get_phys( e.as_ref::<()>(0) ) as u64;
				scratchpad_entries.push(e);
			}
			*ring_page.as_mut(0) = ::kernel::memory::virt::get_phys(array) as u64;
		}
		else
		{
			base_offset = ((256 * 8) / 32) as u8;
		}
		// - Store the DCBAA
		// SAFE: The pointer used is valid, and will stay valid as long as this structure exists
		unsafe {
			regs.set_dcbaap(::kernel::memory::virt::get_phys(ring_page.as_ref::<()>(0)) as u64);
			regs.write_config(n_device_slots as u32);
		}

		// Initialise the command ring
		{
			let ring = ring_page.as_mut_slice(base_offset as usize * ENT_SIZE, (::kernel::PAGE_SIZE - base_offset as usize * ENT_SIZE) / 32);
			let start_addr = ::kernel::memory::virt::get_phys(ring.as_ptr()) as u64;
			log_debug!("ring = {:#x}", start_addr);
			// SAFE: The pointer used is valid, and will stay valid as long as this structure exists
			unsafe {
				// - Add a LINK entry to the end
				*ring.last_mut().unwrap() = crate::hw::structs::IntoTrb::into_trb( crate::hw::structs::TrbLink::new_loopback(start_addr), true);
				// - Store the result
				regs.set_crcr(start_addr | 1);
			}
		}
		Ok(CommandRing {
			ring_page,
			scratchpad_entries,
			scratchpad_array,
			base_offset,
			offset: base_offset,
			cycle_bit: true,
		})
	}

	/// Update an entry in the DCBA
	pub(crate) unsafe fn set_dcba(&self, index: u8, handle: u64) {
		assert!(index > 0);
		::core::ptr::write_volatile(self.ring_page.as_int_mut(index as usize * 8), handle);
	}

	/// Enqueue a command
	pub(crate) fn enqueue_command(&mut self, regs: &crate::hw::Regs, command: impl crate::hw::commands::CommandTrb) {
		log_debug!("enqueue_command: {:?}", command);
		self.enqueue_command_inner(regs, command.into_trb(self.cycle_bit))
	}
	fn enqueue_command_inner(&mut self, regs: &crate::hw::Regs, command_desc: Trb) {
		// 1. Read CRCR to ensure that the ring isn't full
		{
			let crcr = regs.crcr();
			let ctrlr_addr = crcr & !(ENT_SIZE as u64 - 1);
			let ctrlr_cycle_bit = (crcr & 1) == 1;

			let read_idx = self.get_cmd_index(ctrlr_addr).expect("CRCR value out of range");
			let (read_cycle,read_idx) = {
				let ri = read_idx - 1;
				if ri < self.base_offset {
					(!ctrlr_cycle_bit, MAX_OFFSET - 1)
				}
				else {
					(ctrlr_cycle_bit, ri)
				}
				};
			// TODO: Subtract one from the read position
			if read_idx == self.offset && read_cycle == self.cycle_bit {
				panic!("Command ring full!");
			}
		}

		// 2. Write a new entry to the ring 
		let dst = self.ring_page.as_mut(self.offset as usize * ENT_SIZE);
		log_debug!("{}:{} ({:#x}) = {:?}", self.cycle_bit, self.offset, ::kernel::memory::virt::get_phys(dst), command_desc);
		*dst = command_desc;
		self.offset += 1;
		// - If the new offset is equal to the max (i.e. the entry used by the link), then roll over
		if self.offset == MAX_OFFSET {
			self.cycle_bit = !self.cycle_bit;
			self.offset = self.base_offset;
		}

		// 3. Poke the device
		regs.ring_doorbell(0, 0);
	}

	fn get_cmd_index(&self, addr: u64) -> Option<u8> {
		let base_addr = ::kernel::memory::virt::get_phys(self.ring_page.as_ref::<()>(0)) as u64;
		let ofs = match addr.checked_sub(base_addr)
			{
			Some(v) if v < ::kernel::PAGE_SIZE as u64 => v,
			Some(_) => { log_warning!("Bad CommandRing address: {:#x} - well above the ring base {:#x}", addr, base_addr); return None; }
			None => { log_warning!("Bad CommandRing address: {:#x} - below the ring base {:#x}", addr, base_addr); return None; }
			};
		let idx = (ofs / ENT_SIZE as u64) as u8;
		if self.base_offset <= idx && idx <= MAX_OFFSET {
			Some(idx as u8)
		}
		else {
			log_warning!("Bad CommandRing address: {:#x} => {} - outside of valid range", addr, idx);
			None
		}
	}

	/// Read a command and get the type
	pub fn get_command_type(&self, addr: u64) -> Option<crate::hw::structs::TrbType> {
		let idx = self.get_cmd_index(addr)?;
		crate::hw::structs::TrbType::from_trb_word3(self.ring_page.as_ref::<Trb>(idx as usize * ENT_SIZE).word3).ok()
	}
}