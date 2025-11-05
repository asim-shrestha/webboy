const UPPER_NIBBLE_MASK: u8 = 0xF0;
const LOWER_NIBBLE_MASK: u8 = 0x0F;

pub fn is_half_carry(left: u8, right: u8) -> bool {
	let lower_nibble_add = (left & LOWER_NIBBLE_MASK) + (right & LOWER_NIBBLE_MASK);

	// If the lower nibbles added together carry, the next bit will be set
	lower_nibble_add & 0x10 == 0x10
}

pub fn is_half_borrow(left: u8, right: u8) -> bool {
	left & LOWER_NIBBLE_MASK < right & LOWER_NIBBLE_MASK
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_is_half_carry_add() {
		// Not half carries
		// Only upper nibble math
		assert!(!is_half_carry(0b0000_0000u8, 0b0000_0000));
		assert!(!is_half_carry(0b0000_0000u8, 0b1111_1111));
		assert!(!is_half_carry(0b0000_0000u8, 0b0001_0000));
		assert!(!is_half_carry(0b0000_0000u8, 0b0010_0000));
		assert!(!is_half_carry(0b0000_0000u8, 0b0100_0000));
		assert!(!is_half_carry(0b0000_0000u8, 0b1000_0000));
		assert!(!is_half_carry(0b0000_0000u8, 0b1111_0000));
		// Lower nibble math doesn't carry
		assert!(!is_half_carry(0b0000_0001u8, 0b1111_0001));
		assert!(!is_half_carry(0b0000_0010u8, 0b1111_0000));
		assert!(!is_half_carry(0b0000_0100u8, 0b1111_0000));
		// Misc
		assert!(!is_half_carry(0b0000_0111u8, 0b0010_0111));
		assert!(!is_half_carry(0b1111_0000u8, 0b1111_1111));
		assert!(!is_half_carry(0b1111_0111u8, 0b1111_1000));
		assert!(!is_half_carry(0b1111_0111u8, 0b0010_0111));
		assert!(!is_half_carry(0b0000_1110u8, 0b0000_0001));

		// Half carries
		assert!(is_half_carry(0b0000_1000u8, 0b0000_1000));
		assert!(is_half_carry(0b0000_1100u8, 0b0000_0100));
		assert!(is_half_carry(0b0000_1110u8, 0b0000_0010));
		assert!(is_half_carry(0b0000_1111u8, 0b0000_0001));
		assert!(is_half_carry(0b0000_1111u8, 0b0000_1111));
		assert!(is_half_carry(0b1111_1000u8, 0b1111_1000));
		assert!(is_half_carry(0b1111_1111u8, 0b1111_1111));
	}

	#[test]
	fn test_is_half_carry_sub() {
		// Not half carries
		// Only upper nibble math
		assert!(!is_half_borrow(0b0000_0001, 0b0000_0000));
		assert!(!is_half_borrow(0b0000_1111, 0b0000_0000));
		assert!(!is_half_borrow(0b0001_0000, 0b0000_0000));
		assert!(!is_half_borrow(0b1111_1111, 0b0000_0000));
		assert!(!is_half_borrow(0b1111_1111, 0b0000_0000));
		// No carry from upper
		assert!(!is_half_borrow(0b1111_1111, 0b1111_1111));
		assert!(!is_half_borrow(0b1000_1111, 0b0100_0000));
		assert!(!is_half_borrow(0b1000_1111, 0b0010_0000));
		assert!(!is_half_borrow(0b1000_1111, 0b0001_0000));
		assert!(!is_half_borrow(0b0000_0010, 0b0000_0001));
		assert!(!is_half_borrow(0b0000_0100, 0b0000_0001));
		assert!(!is_half_borrow(0b0000_1000, 0b0000_0001));

		// Half carries
		assert!(is_half_borrow(0b0000_0000, 0b0000_0001));
		assert!(is_half_borrow(0b0001_0000, 0b0000_0001));
		assert!(is_half_borrow(0b0010_0000, 0b0000_0001));
		assert!(is_half_borrow(0b0001_0000, 0b0000_0001));
		assert!(is_half_borrow(0b0000_0000, 0b0000_1000));
		assert!(is_half_borrow(0b0001_0000, 0b0000_1000));
		assert!(is_half_borrow(0b0000_0000, 0b0000_1111));
		assert!(is_half_borrow(0b0000_0000, 0b0000_0111));
		assert!(is_half_borrow(0b0000_0000, 0b0000_0011));
		assert!(is_half_borrow(0b0000_0000, 0b0000_0001));
	}
}