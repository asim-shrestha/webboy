use crate::ram::Ram;
use crate::palette::Color;
use crate::lcd::LCDControl;

// Tile rendering unit
pub struct TLU {
}

#[derive(Debug)]
pub struct TLUData {
	pub tile_data: Vec<Vec<Color>>,
	pub background_data: Vec<Vec<Color>>,
}

impl TLU {
	fn get_tile_by_index(ram: &Ram, tile_index: u8) -> [[Color; 8]; 8] {
		let block_start: u16 = if ram.bg_and_window_tile_data_control() { 0x8000 } else { 0x9000 };
		let offset = if ram.bg_and_window_tile_data_control() { tile_index as i16 * 16 } else { (tile_index as i8 as i16) * 16 };

		let tile_start_address = block_start.wrapping_add_signed(offset);

		TLU::get_tile_at_location(ram, tile_start_address)
	}

	fn get_tile_at_location(ram: &Ram, tile_start_address: u16) -> [[Color; 8]; 8] {
		let mut res = [[Color::LightGray; 8]; 8];

		for row_index in 0..8 {
			// Each pixel is 2 bits so there is 2 bytes per row
			let first_byte = ram.read(tile_start_address + (row_index * 2));
			let second_byte = ram.read(tile_start_address + 1 + (row_index * 2));

				for bit in 0..8 {
					let left_bit = ((second_byte >> (7 - bit)) & 1) << 1;
					let right_bit = (first_byte >> (7 - bit)) & 1;
					let bits = left_bit + right_bit;
					let color = Color::from_bits(bits);

					res[row_index as usize][bit as usize] = color;
				}
			}

		res
	}

	pub fn update(&self, ram: &Ram) -> TLUData {
		let mut res: Vec<Vec<Color>> = vec![vec![Color::LightGray; 32 * 8]; 8 * 8];

		for tile_index in 0..=255 {
			let row = (tile_index / 32) as usize;
			let col = (tile_index % 32) as usize;
			let colors = TLU::get_tile_by_index(ram, tile_index);

			for bit_row in 0..colors.len() {
				for bit_col in 0..colors.len() {
					res[(row * 8) + bit_row][(col * 8) + bit_col] = colors[bit_row][bit_col];
				}
			}
		}

		// Get the background
		let start: u16 = 0x9800;
		let mut background_res: Vec<Vec<Color>> = vec![vec![Color::LightGray; 32 * 8]; 32 * 8];

		for pixel_index in 0..32 * 32 {
			let row = (pixel_index / 32) as usize;
			let col = (pixel_index % 32) as usize;

			let tile_index_location = start + pixel_index;
			let tile_index = ram.read(tile_index_location);

			let colors = TLU::get_tile_by_index(ram, tile_index);

			for bit_row in 0..colors.len() {
				for bit_col in 0..colors[0].len() {
					background_res[(row * 8) + bit_row][(col * 8) + bit_col] = colors[bit_row][bit_col];
				}
			}
		}

		TLUData {
			tile_data: res,
			background_data: background_res
		}
	}
}