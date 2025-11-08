use crate::cpu::CPU;
use crate::ppu::PPU;
use crate::ram::RamOperations;

pub struct Device {
	cpu: CPU,
	ppu: PPU,
}

impl Device {
	pub fn new() -> Self {
		Self {
			cpu: CPU::new(),
			ppu: PPU::new()
		}
	}

	pub fn load(&mut self, rom: &[u8]) {
		// Delegate load to ram
		self.cpu.ram.load_rom(rom);

		// Default values that get set after a typical boot screen
		self.cpu.registers.a = 0x01;
		self.cpu.registers.f = 0xB0;
		self.cpu.registers.b = 0x00;
		self.cpu.registers.c = 0x13;
		self.cpu.registers.d = 0x00;
		self.cpu.registers.e = 0xD8;
		self.cpu.registers.h = 0x01;
		self.cpu.registers.l = 0x4D;
		self.cpu.registers.set_sp(0xFFFE);
		self.cpu.registers.pc = 0x0100;

		// TODO: Remove this. The below simulates VBlank progress. Once our PPU is online we don't need to worry about that shit
		self.cpu.ram[0xFF44] = 0x90; // Set LY to simulate some VBlank progress
	}

	pub fn tick(&mut self) {
		let m_cycles = self.cpu.execute(true);
		self.ppu.tick(m_cycles);
	}
}