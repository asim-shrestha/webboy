use crate::cpu::instruction::MCycles;
use crate::ram::Ram;

pub struct DMA {
	pub current_index: u16,
}

const DMA_ADDRESS: u16 = 0xFF46;
const DESTINATION_START_ADDRESS: u16 = 0xFE00;
const MAX_LOWER_NIBBLE: u16 = 0x9F;

impl DMA {
	pub fn new() -> Self {
		Self {
			current_index: 0,
		}
	}

	pub fn tick_transfer(&mut self, ram: &mut Ram, cycles: MCycles) {
		if !ram.dma_requested() { return; }

		let start_location: u16 = (ram.unblocked_read(DMA_ADDRESS) as u16) << 8;

		for _ in 0..cycles {
			let source = start_location + self.current_index;
			let destination = DESTINATION_START_ADDRESS + self.current_index;
			ram.write(destination, ram.unblocked_read(source));

			self.current_index += 1;

			if self.current_index > MAX_LOWER_NIBBLE { break; }
		}

		if self.current_index > MAX_LOWER_NIBBLE {
			self.current_index = 0;
			ram.clear_dma_request();
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

		// Place values to be loaded
		for i in 0..=0x9F {
			ram.write(0x8000 + i, loaded_value);
		}

		// Test init values
		let mut dma = DMA::new();
		assert_eq!(ram.dma_requested(), false);
		assert_eq!(dma.current_index, 0);
		assert_eq!(ram.unblocked_read(DESTINATION_START_ADDRESS), 0);


		// Test start_transfer activates dma
		ram.write(DMA_ADDRESS, 0x80);
		assert_eq!(ram.dma_requested(), true);

		// First tick: do a few bytes.
		dma.tick_transfer(&mut ram, 1);
		assert_eq!(dma.current_index, 1);
		assert_eq!(ram.unblocked_read(DESTINATION_START_ADDRESS), loaded_value);

		// Almost finish
		dma.tick_transfer(&mut ram, (MAX_LOWER_NIBBLE - 1) as MCycles);
		assert_eq!(dma.current_index, MAX_LOWER_NIBBLE);
		assert_eq!(ram.dma_requested(), true);

		// Finish (and over finish)
		dma.tick_transfer(&mut ram, 999);
		assert_eq!(ram.dma_requested(), false);
		assert_eq!(dma.current_index, 0);

		// Assert every address is correct
		for i in 0..=MAX_LOWER_NIBBLE {
			assert_eq!(ram.unblocked_read(DESTINATION_START_ADDRESS + i), loaded_value);
		}
		assert_eq!(ram.unblocked_read(DESTINATION_START_ADDRESS + MAX_LOWER_NIBBLE), loaded_value);
		assert_eq!(ram.unblocked_read(DESTINATION_START_ADDRESS + MAX_LOWER_NIBBLE + 1), 0);
	}
}
