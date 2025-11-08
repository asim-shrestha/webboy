use std::ops::{Index, IndexMut};

const TWO_TO_THE_16: usize = 65_536;

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
pub enum Interrupt {
	VBlank = 0,
	Stat = 1,
	Timer = 2,
	Serial = 3,
	Joypad = 4,
}

impl Interrupt {
	pub fn handler_address(&self) -> u16 {
		match self {
			Interrupt::VBlank => 0x40,
			Interrupt::Stat => 0x48,
			Interrupt::Timer => 0x50,
			Interrupt::Serial => 0x58,
			Interrupt::Joypad => 0x60,
		}
	}
}

pub struct Ram {
	data: [u8; TWO_TO_THE_16],
}

impl Index<usize> for Ram {
	type Output = u8;

	fn index(&self, index: usize) -> &Self::Output {
		&self.data[index]
	}
}

impl IndexMut<usize> for Ram {
	fn index_mut(&mut self, index: usize) -> &mut Self::Output {
		if index >= 0x8000 && index <= 0x97FF {
			// println!("Mutating index BRUHH!!: {:4X}", index);
		}
		&mut self.data[index]
	}
}


impl Ram {
	pub fn new() -> Self {
		Self {
			data: [0; TWO_TO_THE_16]
		}
	}

	pub fn load_rom(&mut self, rom: &[u8]) {
		// TODO: Handle MBCs for larger ROMs
		if rom.len() > 65536 {
			panic!(
				"ROM size incorrect. Expected {} bytes, got {} bytes",
				32768,
				rom.len()
			);
		}


		self.data[..rom.len()].copy_from_slice(rom);
	}

	pub fn interrupts_enabled(&self) -> bool {
		self.data[0xFFFF] > 0
	}

	pub fn pending_interrupt(&self) -> Option<Interrupt> {
		for interrupt in [
			Interrupt::VBlank,
			Interrupt::Stat,
			Interrupt::Timer,
			Interrupt::Serial,
			Interrupt::Joypad,
		] {
			let mask = 1 << (interrupt as u8);
			if (self.data[0xFFFF] & mask) != 0 && (self.data[0xFF0F] & mask) != 0 {
				return Some(interrupt);
			}
		}

		None
	}

	pub fn request_interrupt(&mut self, interrupt: Interrupt) {
		let mask = 1 << (interrupt as u8);
		self.data[0xFF0F] |= mask;
	}

	pub fn clear_interrupt(&mut self, interrupt: Interrupt) {
		let mask = 1 << (interrupt as u8);
		self.data[0xFF0F] &= !mask;
	}
}

pub trait TestRamOperations {
	// Arbitrarily load data within RAM. Should only be used within tests
	fn test_load(&mut self, location: u16, data: Vec<u8>);
}

impl TestRamOperations for Ram {
	fn test_load(&mut self, location: u16, data: Vec<u8>) {
		let start = location as usize;
		let end = start + data.len();
		self.data[start..end].copy_from_slice(&data);
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_test_load() {
		let mut ram = Ram::new();

		ram.test_load(0, vec![19, 123, 73, 57]);

		assert_eq!(ram[0], 19);
		assert_eq!(ram[1], 123);
		assert_eq!(ram[2], 73);
		assert_eq!(ram[3], 57);
	}

	#[test]
	fn test_interrupts() {
		let mut ram = Ram::new();

		// Enabled but none of the pending ones
		ram[0xFFFF] = 0b1111_0000;
		ram[0xFF0F] = 0b0000_1111;
		assert!(ram.interrupts_enabled());
		assert!(ram.pending_interrupt().is_none());

		// Enabled a pending interrupt
		ram[0xFFFF] = 0b1111_0010;
		assert!(ram.interrupts_enabled());
		assert_eq!(ram.pending_interrupt(), Some(Interrupt::Stat));

		// Try clearing some interrupts
		ram.clear_interrupt(Interrupt::Timer);
		assert_eq!(ram[0xFF0F], 0b0000_1011);
		ram.clear_interrupt(Interrupt::VBlank);
		assert_eq!(ram[0xFF0F], 0b0000_1010);

		// Try requesting interrupts back
		ram.request_interrupt(Interrupt::Timer);
		assert_eq!(ram[0xFF0F], 0b0000_1110);
	}
}