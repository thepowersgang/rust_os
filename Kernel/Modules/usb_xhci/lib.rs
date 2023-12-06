// "Tifflin" Kernel - OHCI USB driver
// - By John Hodge (Mutabah / thePowersGang)
//
// Modules/usb_xhci/lib.rs
//! eXtensible Host Controller Interface (xHCI) driver
#![no_std]
#![feature(linkage)]	// for module_define!
#![feature(ptr_metadata)]	// for hacks
use kernel::prelude::*;
use kernel::lib::mem::aref::{Aref,ArefBorrow};

#[macro_use]
extern crate kernel;
extern crate usb_core;

::kernel::module_define!(usb_xhci, [usb_core], init);

mod pci;
mod hw;
mod usb_host;
mod command_ring;
mod event_ring;
//mod memory_pools; // TODO: Eventually use this for queues without needing a whole page

mod device_state;

fn init()
{
	static PCI_DRIVER: pci::PciDriver = pci::PciDriver;
	::kernel::device_manager::register_driver(&PCI_DRIVER);
}

#[derive(Default)]
struct AtomicBitset256([::core::sync::atomic::AtomicU32; 256/32]);
impl AtomicBitset256 {
	pub fn set(&self, idx: u8) {
		use ::core::sync::atomic::Ordering;
		self.0[idx as usize / 32].fetch_or(1 << idx%32, Ordering::SeqCst);
	}
	pub fn get_first_set_and_clear(&self) -> Option<usize> {
		use ::core::sync::atomic::Ordering;
		for (i,s) in self.0.iter().enumerate() {
			let v = s.load(Ordering::SeqCst);
			if v != 0 {
				let j = v.trailing_zeros() as usize;
				let bit = 1 << j;
				if s.fetch_and(!bit, Ordering::SeqCst) & bit != 0 {
					return Some(i*32 + j);
				}
			}
		}
		None
	}
	#[cfg(false_)]
	pub fn iter_set_and_clear(&self) -> impl Iterator<Item=usize> + '_ {
		use ::core::sync::atomic::Ordering;
		self.0.iter().enumerate()
			// Load/clear each word
			.map(|(i,v)| (i,v.swap(0, Ordering::SeqCst)))
			// Convert into (Index,IsSet)
			.flat_map(|(j,v)| (0..32).map(move |i| (j*32+i, (v >> i) & 1 != 0) ))
			// Yield only the indexes of set bits
			.filter_map(|(idx,isset)| if isset { Some(idx) } else { None })
	}
}

type HostRef = ArefBorrow<HostInner>;
struct HostInner {
	regs: hw::Regs,

	command_ring: ::kernel::sync::Mutex<command_ring::CommandRing>,
	event_ring_zero: event_ring::EventRing<event_ring::Zero>,

	port_update: AtomicBitset256,
	port_update_waker: ::kernel::sync::Spinlock<::core::task::Waker>,

	slot_enable_ready: ::kernel::futures::flag::SingleFlag,
	slot_enable_idx: ::core::sync::atomic::AtomicU8,

	_irq_handle: Option<::kernel::irqs::ObjectHandle>,

	enum_state: device_state::EnumState,

	/// Indexed by the `usb_core` address minus 1
	devices: [::kernel::sync::Mutex<Option<Box<device_state::DeviceInfo>>>; 255],

	/// Events for a given slot (indexed by slot index minus 1)
	slot_events: Vec<SlotEvents>,
}
#[derive(Default)]
struct SlotEvents {
	/// A `ConfigureEndpoint` command has completed
	configure: ::kernel::sync::EventChannel,
	/// Transfer completed on an endpoint
	endpoints: [::kernel::futures::single_channel::SingleChannel<(hw::structs::TrbNormalData,u32,crate::hw::structs::TrbCompletionCode,)>; 31],
}
impl HostInner
{
	/// Construct a new instance
	fn new_aref(irq: u32, io: ::kernel::device_manager::IOBinding) -> Result<Aref<Self>, ::kernel::device_manager::DriverBindError>
	{
		log_debug!("new_boxed(irq={irq}, io={io:?}");
		// SAFE: This function is only called with a valid register binding
		let regs = unsafe { hw::Regs::new(io) };

		// Controller init
		// - Trigger a reset and wait for USBSTS.NCR to become zero
		// SAFE: Correct write
		unsafe {
			regs.write_usbcmd(hw::regs::USBCMD_HCRST);
		}
		while regs.usbsts() & hw::regs::USBSTS_CNR != 0 {
			// TODO: Sleep with timeout
			::kernel::futures::block_on(::kernel::futures::msleep(5));
		}

		// - Device init, any order:
		//   - Configure CONFIG.MaxSlotsEn to the number of device slots desired
		//   - Set the command ring (Set the initial dequeue pointer?)
		let command_ring = command_ring::CommandRing::new(&regs, 128)?;
		//   - Set up MSI-X (aka the event ring)
		let event_ring_zero = event_ring::EventRing::new_zero(&regs)?;

		let nports = regs.max_ports();
		let mut rv = Aref::new(HostInner {
			regs,

			command_ring: ::kernel::sync::Mutex::new(command_ring),
			event_ring_zero,
			_irq_handle: None,  // Initialised after construction
			
			port_update_waker: ::kernel::sync::Spinlock::new(kernel::futures::null_waker()),
			port_update: Default::default(),

			slot_enable_ready: Default::default(),
			slot_enable_idx: Default::default(),
			enum_state: Default::default(),
			devices: [(); 255].map(|_| Default::default()),
			slot_events: (0..255).map(|_| Default::default()).collect(),
			});

		// Bind interrupt
		{
			struct RawSend<T: Send>(*const T);
			unsafe impl<T: Send> Send for RawSend<T> {}
			let ret_raw = RawSend(&*rv);
			// SAFE: Pointer _should_ be valid as long as this IRQ binding exists
			let binding = ::kernel::irqs::bind_object(irq, Box::new(move || unsafe { (*ret_raw.0).handle_irq() } ));
			Aref::get_mut(&mut rv).unwrap()._irq_handle = Some(binding);
		}
			
		// - Set USBCMD.RUN = 1
		log_debug!("pre-start: USBSTS {:#x}", rv.regs.usbsts());
		// SAFE: Correct write
		unsafe {
			rv.regs.write_usbcmd(hw::regs::USBCMD_RS|hw::regs::USBCMD_INTE);
		}
		log_debug!("Post-run: USBSTS {:#x}", rv.regs.usbsts());
		
		::usb_core::register_host(Box::new(usb_host::UsbHost { host: rv.borrow() }), nports);

		// Test commands
		if false {
			log_debug!("--- TESTING COMMAND ---");
			rv.command_ring.lock().enqueue_command(&rv.regs, hw::commands::Nop);
			rv.command_ring.lock().enqueue_command(&rv.regs, hw::commands::Nop);
			rv.command_ring.lock().enqueue_command(&rv.regs, hw::commands::Nop);
			::kernel::threads::yield_time();
			log_debug!("-/- TESTING COMMAND -/-");
		}
		
		// TODO: Determine how to prepare the already-connected ports
		// - Should they generate an interrupt when interrupts are enabled? or just pre-scan?
		for p in 0 .. rv.regs.max_ports()
		{
			log_debug!("Port Status #{}: {:#x}", 1+p, rv.regs.port(p).sc());
			if rv.regs.port(p).sc() & hw::regs::PORTSC_CCS != 0 {
				rv.port_update.set(p);
			}
		}
		rv.port_update_waker.lock().wake_by_ref();

		Ok(rv)
	}

	fn handle_irq(&self) -> bool
	{
		let sts = self.regs.usbsts();
		log_trace!("USBSTS = {:#x}", sts);
		if sts & (hw::regs::USBSTS_EINT|hw::regs::USBSTS_HCE|hw::regs::USBSTS_PCD) != 0 {
			let mut h = 0;
			if sts & hw::regs::USBSTS_HCE != 0 {
				todo!("Host controller error raised!");
			}
			if sts & hw::regs::USBSTS_EINT != 0 {
				h |= hw::regs::USBSTS_EINT;
				// Signal any waiting
				self.event_ring_zero.check_int(&self.regs);
				while let Some(ev) = self.event_ring_zero.poll(&self.regs) {
					use event_ring::Event;
					match ev
					{
					Event::PortStatusChange { port_id, completion_code: _ } => {
						// Port IDs are indexed from 1
						self.port_update.set(port_id - 1);
						self.port_update_waker.lock().wake_by_ref();
						},
					Event::CommandCompletion { trb_pointer, completion_code, param: _param, slot_id, vf_id: _vf_id } => {
						let ty = self.command_ring.lock().get_command_type(trb_pointer);
						if let crate::hw::structs::TrbCompletionCode::Success = completion_code {
							log_trace!("CommandCompletion {:#x} {:?}: SUCCESS", trb_pointer, ty);
							match ty
							{
							Some(hw::structs::TrbType::NoOpCommand) => {
								},
							Some(hw::structs::TrbType::EnableSlotCommand) => {
								self.slot_enable_idx.store(slot_id, ::core::sync::atomic::Ordering::SeqCst);
								self.slot_enable_ready.trigger();
								},
							Some(hw::structs::TrbType::AddressDeviceCommand) => {
								self.slot_enable_ready.trigger();
								}
							Some(hw::structs::TrbType::ConfigureEndpointCommand) => {
								self.slot_events[slot_id as usize - 1].configure.post();
								},
							_ => {},
							}
						}
						else {
							log_error!("CommandCompletion {:#x} {:?}: Not success, {:?}", trb_pointer, ty, completion_code);
						}
						},
					Event::Transfer { data, transfer_length, completion_code, slot_id, endpoint_id } => {
						self.slot_events[slot_id as usize - 1].endpoints[endpoint_id as usize - 1].store( (data, transfer_length, completion_code) );
						},
					_ => {},
					}
				}
			}
			if h != sts {
				todo!("Unhandled interrupt bit {:#x}", sts ^ h);
			}
			self.regs.write_usbsts(h);
			true
		}
		else {
			false
		}
	}
}
