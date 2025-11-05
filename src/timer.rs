use crate::ram::{Interrupt, Ram, RamOperations};

const M_CYCLES_TO_CLOCK_CYCLES: u16 = 4;
const M_CYCLES_TO_DIV_INCREMENT: u16 = 64;


const DIV_ADDRESS: usize = 0xFF04;
const TIMA_ADDRESS: usize = 0xFF05;
const TMA_ADDRESS: usize = 0xFF06;
const TAC_ADDRESS: usize = 0xFF07;

pub struct Timer {
	pub cycles: u128,
	cycles_since_div: u16,
	cycles_since_tima: u16,
}

impl Timer {
	pub fn new() -> Self {
		Timer {
			cycles: 0,
			cycles_since_div: 0,
			cycles_since_tima: 0,
		}
	}

	pub fn enabled(ram: &mut Ram) -> bool {
		(ram[TAC_ADDRESS] & 0b0000_0100) != 0
	}

	pub fn increment_cycle(&mut self, ram: &mut Ram, cycle_count: u8) {
		self.cycles = self.cycles.wrapping_add(cycle_count as u128);

		if cycle_count == 0 || cycle_count > 6 {
			panic!("timer: Invalid cycle count increase of {}", cycle_count);
		}

		self.cycles_since_div += cycle_count as u16;
		self.cycles_since_tima += cycle_count as u16;

		// DIV is always incremented at the cycle interval
		if self.cycles_since_div >= M_CYCLES_TO_DIV_INCREMENT {
			ram[DIV_ADDRESS] = ram[DIV_ADDRESS].wrapping_add(1);
			self.cycles_since_div -= M_CYCLES_TO_DIV_INCREMENT;
		}

		// TIMA is incremented based on the TMA register
		let cycles_to_tma = Timer::cycles_to_tma(ram);
		if self.cycles_since_tima >= cycles_to_tma && Timer::enabled(ram) {
			self.cycles_since_tima -= cycles_to_tma;

			let (res, overflow) = ram[TIMA_ADDRESS].overflowing_add(1u8);
			ram[TIMA_ADDRESS] = res;

			// When TIMA overflows, we reset and send an interrupt
			if overflow {
				ram[TIMA_ADDRESS] = ram[TMA_ADDRESS];
				ram.request_interrupt(Interrupt::Timer);
			}
		}
	}

	pub fn cycles_to_tma(ram: &mut Ram) -> u16 {
		let tac = ram[TAC_ADDRESS];
		let control_bits = tac & 0b0000_0011;

		match control_bits {
			0b00 => 256,
			0b01 => 4,
			0b10 => 16,
			0b11 => 64,
			bit => panic!("timer: Invalid control bit of {bit}"),
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_increment() {
		let mut timer = Timer {
			cycles: 0,
			cycles_since_div: 61,
			cycles_since_tima: 2,
		};

		let mut ram = Ram::new();
		ram[TAC_ADDRESS] = 0b01;
		ram[TMA_ADDRESS] = 70;

		timer.increment_cycle(&mut ram, 1);
		assert_eq!(ram[TIMA_ADDRESS], 0, "Value should not have been incremented");
		assert_eq!(ram[DIV_ADDRESS], 0, "Value should not have been incremented");

		// TIMA inc
		timer.increment_cycle(&mut ram, 1);
		assert_eq!(ram[TIMA_ADDRESS], 1, "Value should have been incremented");
		assert_eq!(timer.cycles_since_tima, 0, "TIMA Cycles should have been reset");
		assert_eq!(ram[DIV_ADDRESS], 0, "Value should not have been incremented");

		// DIV inc
		timer.increment_cycle(&mut ram, 1);
		assert_eq!(ram[TIMA_ADDRESS], 1, "Value should not have been incremented");
		assert_eq!(ram[DIV_ADDRESS], 1, "Value should have been incremented");
		assert_eq!(timer.cycles_since_div, 0, "TIMA Cycles should have been reset");

		// TIMA overflow
		ram[TIMA_ADDRESS] = 0xFF;
		timer.cycles_since_tima = 3;
		timer.increment_cycle(&mut ram, 1);

		assert_eq!(ram[TIMA_ADDRESS], 70, "Value should have been set to TMA");
		assert_eq!(timer.cycles_since_tima, 0, "TIMA Cycles should have been reset");
		assert_eq!(timer.cycles_since_div, 1, "TIMA Cycles should have been reset");
		assert_eq!(ram[0xFF0F], 0b0000_0100, "The timer interrupt request should be set");
	}
}