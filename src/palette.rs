#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Color {
	White=0,
	LightGray=1,
	DarkGray=2,
	Black=3,
}

impl Color {
	pub fn to_rgba(&self) -> [u8; 4] {
		match self {
			Color::White => [255, 255, 255, 255],
			Color::LightGray => [0, 255, 0, 255],
			Color::DarkGray => [255, 0, 0, 255],
			Color::Black => [0, 0, 255, 255],
		}
	}
}

impl Color {
	pub fn from_bits(v: u8) -> Color {
		match v {
			0 => Color::White,
			1 => Color::LightGray,
			2 => Color::DarkGray,
			3 => Color::Black,
			bits => panic!("Invalid color bits {}. Bits must be less than 3", bits),
		}
	}
}

struct Palette {
	id_zero: Color,
	id_one: Color,
	id_two: Color,
	id_three: Color,
}

fn get_palette(palette_register: u8) -> Palette {
	Palette {
		id_zero:Color::from_bits(palette_register & 0b11),
		id_one: Color::from_bits((palette_register >> 2) & 0b11),
		id_two: Color::from_bits((palette_register >> 4) & 0b11),
		id_three: Color::from_bits((palette_register >> 6) & 0b11),
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_get_palette() {
		assert_eq!(Color::from_bits(0), Color::White);
		assert_eq!(Color::from_bits(1), Color::LightGray);
		assert_eq!(Color::from_bits(2), Color::DarkGray);
		assert_eq!(Color::from_bits(3), Color::Black);

		let palette = get_palette(0b1101_1000);

		assert_eq!(palette.id_zero, Color::White);
		assert_eq!(palette.id_one, Color::DarkGray);
		assert_eq!(palette.id_two, Color::LightGray);
		assert_eq!(palette.id_three, Color::Black);
	}
}