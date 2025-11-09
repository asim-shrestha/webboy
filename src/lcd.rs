use crate::ram::Ram;

pub trait LCDControl {
	fn lcd_enabled(&self) -> bool;
	fn window_tile_map_control(&self) -> bool;
	fn window_enabled(&self) -> bool;
	fn bg_and_window_tile_data_control(&self) -> bool;
	fn bg_tile_map_control(&self) -> bool;
	fn obj_size_control(&self) -> bool;
	fn obj_enabled(&self) -> bool;
	fn bg_and_window_enabled(&self) -> bool;
	fn update_ly(&mut self, value: u8);
}

impl LCDControl for Ram {
	fn lcd_enabled(&self) -> bool {
		(self.read(LCDC_ADDRESS) & 0b1000_0000) != 0
	}
	fn window_tile_map_control(&self) -> bool {
		(self.read(LCDC_ADDRESS) & 0b0100_0000) != 0
	}

	fn window_enabled(&self) -> bool {
		(self.read(LCDC_ADDRESS) & 0b0010_0000) != 0
	}

	fn bg_and_window_tile_data_control(&self) -> bool {
		(self.read(LCDC_ADDRESS) & 0b0001_0000) != 0
	}

	fn bg_tile_map_control(&self) -> bool {
		(self.read(LCDC_ADDRESS) & 0b0000_1000) != 0
	}

	fn obj_size_control(&self) -> bool {
		(self.read(LCDC_ADDRESS) & 0b0000_0100) != 0
	}

	fn obj_enabled(&self) -> bool {
		(self.read(LCDC_ADDRESS) & 0b0000_0010) != 0
	}

	fn bg_and_window_enabled(&self) -> bool {
		(self.read(LCDC_ADDRESS) & 0b0000_0001) != 0
	}

	fn update_ly(&mut self, value: u8) {
		self.write(LY_ADDRESS, value);
	}
}

const LCDC_ADDRESS: u16 = 0xFF40;
const LY_ADDRESS: u16 = 0xFF44;
const LYC_LY_COMPARE_ADDRESS: u16 = 0xFF45;
const STAT_ADDRESS: u16 = 0xFF41;