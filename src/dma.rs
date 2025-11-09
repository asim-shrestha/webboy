use crate::cpu::instruction::MCycles;
use crate::ram::Ram;

pub struct DMA {
	pub active: bool,
	pub current_index: usize,
}

const DMA_ADDRESS: usize = 0xFF46;
const DESTINATION_START_ADDRESS: usize = 0xFE00;
const MAX_LOWER_NIBBLE: usize = 0x9F;

impl DMA {
	pub fn new() -> Self {
		Self {
			active: false,
			current_index: 0,
		}
	}

	pub fn start_transfer(&mut self) {
		self.active = true;
	}

	pub fn tick_transfer(&mut self, ram: &mut Ram, cycles: MCycles) {
		if !self.active { return; }

		let start_location = (ram[DMA_ADDRESS] as usize) << 8;

		for _ in 0..cycles {
			let source =  start_location + self.current_index;
			let destination = DESTINATION_START_ADDRESS + self.current_index;
			ram[destination] = ram[source];

			self.current_index += 1;

			if self.current_index > MAX_LOWER_NIBBLE { break; }
		}

		if self.current_index > MAX_LOWER_NIBBLE {
			self.active = false;
			self.current_index = 0;
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_dma() {
		let loaded_value = 69;
		let mut ram = Ram::new();
		ram[DMA_ADDRESS] = 0x80;

		for i in 0..=0x9F {
			ram[0x8000 + i] = loaded_value;
		}

		// Test init values
		let mut dma = DMA::new();
		assert_eq!(dma.active, false);
		assert_eq!(dma.current_index, 0);

		// Test start_transfer activates dma
		dma.start_transfer();
		assert_eq!(dma.active, true);

		// First tick: do a few bytes.
		dma.start_transfer();
		dma.tick_transfer(&mut ram, 1);
		assert_eq!(dma.current_index, 1);
		assert_eq!(ram[DESTINATION_START_ADDRESS], loaded_value);

		// Almost finish
		dma.tick_transfer(&mut ram, MAX_LOWER_NIBBLE - 1);
		assert_eq!(dma.current_index, MAX_LOWER_NIBBLE);

		// Finish (and over finish)
		dma.tick_transfer(&mut ram, 999);
		assert_eq!(dma.active, false);
		assert_eq!(dma.current_index, 0);

		// Assert every address is correct
		for i in 0..=MAX_LOWER_NIBBLE {
			assert_eq!(ram[DESTINATION_START_ADDRESS + i], loaded_value);
		}
		assert_eq!(ram[DESTINATION_START_ADDRESS + MAX_LOWER_NIBBLE], loaded_value);
		assert_eq!(ram[DESTINATION_START_ADDRESS + MAX_LOWER_NIBBLE + 1], 0);
	}
}
