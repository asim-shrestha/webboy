use std::collections::VecDeque;
use crate::cpu::instruction::MCycles;
use crate::ram::{Interrupt, Ram};
use crate::lcd::{LCDControl};

/// Makes graphics. Has 12 registers
/// 160x144 pixels
/// Pixels are grouped in 8x8 squares called tiles with a color index from 0 to 3
/// 20x14 tiles
/// The system supports 256 background tiles and 256 object tiles
/// You can only have 40 sprites per game and 10 a single line

/// The screen is just a viewport into the background map.
/// The background map is 32x32
// I can choose my own palette for
struct Palette {
	colors: [u8; 4],
}

struct Tilemap<'a> {
	references: [Tile<'a>; 256],
}


/// Window is a directly overlay. Typically is placed on the right or bottom
/// There is no translucency or transparency
///
/// For objects, they all have an Object Attribute Map (OAM)
/// Sprites can be up to 16 pixels in height
/// They have translucency
struct Layers<'a> {
	background: Tilemap<'a>,
	window: Tilemap<'a>,
	objects: u8,
}


// Timing
// CRT style graphics will do scan-line based rendering




const DOTS_PER_M_CYCLE: usize = 4;
const DOTS_PER_M_CYCLE_DOUBLE_SPEED: usize = 8;
const DOTS_PER_60_FPS_FRAME: usize = 70_224;
const DOTS_PER_SCAN_LINE: u16 = 456;
const TOTAL_SCAN_LINES: u8 = 154;
const INTERRUPT_SCANLINE: u8 = 144;

type Tile<'a> = &'a [[u8; 8]; 8];

enum PPUMode {
	OAMScan=2,
	DrawingPixels=3,
	HorizontalBlank=0,
	VerticalBlank=1,
}

pub struct PPU {
	pub current_scanline: u8,
	current_scanline_dot: u16,
	mode: PPUMode,

	background_fifo: [u8; 16],
	object_fifo: [u8; 16],

	work_stack: VecDeque<(fn() -> (), fn() -> ())>,
}

type DotsTaken = u8;

// 160x144 pixels
impl PPU {
	pub fn new() -> Self {
		PPU {
			current_scanline: 0,
			current_scanline_dot: 0,
			mode: PPUMode::OAMScan,

			background_fifo: [0; 16],
			object_fifo: [0; 16],

			work_stack: VecDeque::new(),
		}
	}

	pub fn tick(&mut self, m_cycles: MCycles, ram: &mut Ram) {
		let dots = DOTS_PER_M_CYCLE * m_cycles;

		for _ in 0..dots {
			self.do_dot(ram);
		}
	}

	fn do_dot(&mut self, ram: &mut Ram) {
		let deque_work = self.work_stack.pop_front();

		if let Some(work_pair) = deque_work {
			work_pair.0();
			work_pair.1();
		} else {
			// println!("Nothing!");
		}

		// Send out two pixels per dot if fifo has > 8 pixels
		self.current_scanline_dot += 1;

		// Each line
		if self.current_scanline_dot == 456 {
			self.current_scanline += 1;
			self.current_scanline_dot = 0;
		}

		if self.current_scanline == INTERRUPT_SCANLINE && self.current_scanline_dot == 0 {
			self.mode = PPUMode::VerticalBlank;
			ram.request_interrupt(Interrupt::VBlank);
		}

		if self.current_scanline == TOTAL_SCAN_LINES {
			// TODO: Handle frame end
			self.current_scanline = 0;
			ram.clear_interrupt(Interrupt::VBlank);
		}

		self.handle_lcd_update(ram);
	}

	fn handle_lcd_update(&mut self, ram: &mut Ram) {
		ram.update_ly(self.current_scanline);

	}

	fn handle_oam_scan(&mut self) {
		// We don't need to split this up at all. We just need to do this once

		// 	The Game Boy PPU can display up to 40 movable objects (or sprites), each 8×8 or 8×16 pixels.
		// Because of a limitation of hardware, only ten objects can be displayed per scanline.
		// Object tiles have the same format as BG tiles, but they are taken from tile blocks 0 and 1 located at $8000-8FFF and have unsigned numbering.
	}

	fn handle_pixel_fetch(&mut self) {
		// During Mode 3, by default the PPU outputs one pixel to the screen per dot, from left to right; the screen is 160 pixels wide, so the minimum Mode 3 length is 160 + 121 = 172 dots.
	}

	fn get_tile(&mut self) {

	}

	fn get_tile_map_start(&mut self, ram: &Ram) -> usize {
		if ram.bg_tile_map_control() {
			0x9C00
		} else {
			0x9800
		}

	}

	fn sleep(&mut self) -> DotsTaken {
		// Do nothing
		2
	}

	fn get_tile_color(tile: Tile) -> [[u8; 8]; 8] {
		todo!();
	}
}