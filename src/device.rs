use crate::cpu::CPU;
use crate::ppu::PPU;
use crate::tlu::{TLUData, TLU};
use std::sync::mpsc::{Sender};
use crate::dma::DMA;

#[derive(Debug)]
pub struct ImageData {
	pub tlu_data: TLUData,
}

pub struct Device {
	cpu: CPU,
	ppu: PPU,
	tlu: TLU,
	dma: DMA,

	image_channel: Sender<ImageData>,
	frame_counter: u64,
}

impl Device {
	pub fn new(image_channel: Sender<ImageData>) -> Self {
		Self {
			cpu: CPU::new(),
			ppu: PPU::new(),
			tlu: TLU {},
			dma: DMA::new(),
			image_channel,
			frame_counter: 0,
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
		self.cpu.ram.write(0xFF44, 0x90); // Set LY to simulate some VBlank progress
	}

	pub fn tick(&mut self) {
		// The CPU is suspended during DMA transfers
		let m_cycles = self.cpu.execute(false);
		self.dma.tick_transfer(&mut self.cpu.ram, m_cycles);

		self.ppu.tick(m_cycles, &mut self.cpu.ram);

		// Only send frame data every ~70224 dots (60 FPS)
		// Each M-cycle = 4 dots, so send every ~17556 ticks
		self.frame_counter += m_cycles as u64;
		if self.frame_counter >= 17556 {
			let tlu_data = self.tlu.update(&self.cpu.ram);
			let _ = self.image_channel.send(ImageData {tlu_data});
			self.frame_counter -= 17556;
		}
	}
}