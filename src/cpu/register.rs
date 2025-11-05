#[derive(Debug, PartialEq, Eq)]
pub struct Registers {
    // Groups of these two registers can either be looked at as single 8bit registers or one 16bit register
    pub a: u8,
    pub f: u8,

    pub b: u8,
    pub c: u8,

    pub d: u8,
    pub e: u8,

    pub h: u8,
    pub l: u8,

    pub sp_0: u8,
    pub sp_1: u8,
    pub pc: u16,
}

pub enum Flag {
    Zero,
    Subtraction,
    HalfCarry,
    Carry,
}

impl Registers {
    // Let r8 be some register
    pub fn new() -> Self {
        Registers {
            a: 0,
            f: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            sp_0: 0,
            sp_1: 0,
            pc: 0,
        }
    }

    pub fn get_r8(&mut self, location: u8) -> &mut u8 {
        // Only 7 registers so we only care about the first 3 bits
        let location = location & 0b0000_0111;

        // The game boy CPU maps these values to these registers
        match location {
            0 => &mut self.b,
            1 => &mut self.c,
            2 => &mut self.d,
            3 => &mut self.e,
            4 => &mut self.h,
            5 => &mut self.l,
            // 6 => TODO: bytes pointed to by HL,
            7 => &mut self.a,
            _ => panic!("Unsupported r8 location {location}"),
        }
    }

    pub fn get_r16(&self, location: u8) -> u16 {
        // Only four 16bit registers
        let location = location & 0b0000_0011;

        match location {
            0 => u16::from_be_bytes([self.b, self.c]),
            1 => u16::from_be_bytes([self.d, self.e]),
            2 => u16::from_be_bytes([self.h, self.l]),
            3 => u16::from_be_bytes([self.sp_0, self.sp_1]),
            _ => panic!("Unsupported r16 location {location}"),
        }
    }

    pub fn set_r16(&mut self, location: u8, value: u16) {
        // Only four 16bit registers
        let (upper, lower) = match location {
            0 => (&mut self.b, &mut self.c),
            1 => (&mut self.d, &mut self.e),
            2 => (&mut self.h, &mut self.l),
            3 => (&mut self.sp_0, &mut self.sp_1),
            _ => panic!("Unsupported r16 location {location}"),
        };

        let bytes: [u8; 2] = value.to_be_bytes();
        *upper = bytes[0];
        *lower = bytes[1];
    }

    pub fn get_hl(&self) -> u16 {
        self.get_r16(2)
    }

    pub fn set_hl(&mut self, value: u16) {
        self.set_r16(2, value);
    }

    pub fn get_sp(&self) -> u16 {
        self.get_r16(3)
    }

    pub fn set_sp(&mut self, value: u16) {
        self.set_r16(3, value);
    }

    pub fn set_flag(&mut self, flag: Flag, is_set: bool) {
        let mask = Registers::get_flag_mask(flag);

        if is_set {
            self.f |= mask;
        } else {
            self.f &= !mask;
        }

        self.f &= 0xF0; // (Game Boy quirk) low nibble of F must be 0
    }

    pub fn get_flag(&self, flag: Flag) -> bool {
        let mask = Self::get_flag_mask(flag);
        (self.f & mask) > 0
    }

    pub fn cc(&self, cc: u8) -> bool {
        match cc {
            0 => !self.get_flag(Flag::Zero),
            1 => self.get_flag(Flag::Zero),
            2 => !self.get_flag(Flag::Carry),
            3 => self.get_flag(Flag::Carry),
            code => panic!("Invalid condition code '{code}' received"),
        }
    }

    fn get_flag_mask(flag: Flag) -> u8 {
        match flag {
            Flag::Zero => 0b1000_0000,
            Flag::Subtraction => 0b0100_0000,
            Flag::HalfCarry => 0b0010_0000,
            Flag::Carry => 0b0001_0000,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get_flag() {
        let mut registers = Registers::new();
        assert!(!registers.get_flag(Flag::Zero));

        registers.f = 0b1000_0000;
        assert!(registers.get_flag(Flag::Zero));
        assert!(!registers.get_flag(Flag::Subtraction));

        registers.f = 0b0100_0000;
        assert!(registers.get_flag(Flag::Subtraction));
        assert!(!registers.get_flag(Flag::HalfCarry));

        registers.f = 0b0010_0000;
        assert!(registers.get_flag(Flag::HalfCarry));
        assert!(!registers.get_flag(Flag::Carry));

        registers.f = 0b0001_0000;
        assert!(registers.get_flag(Flag::Carry));
        assert!(!registers.get_flag(Flag::Zero));
    }

    #[test]
    fn test_get_r16() {
        let mut registers = Registers::new();
        registers.b = 0b0100_0000;
        registers.c = 0b0000_1111;

        registers.d = 0b0000_0000;
        registers.e = 0b1010_1100;

        registers.h = 0b1111_1111;
        registers.l = 0b0000_1111;

        registers.sp_0 = 0b1100_1100;
        registers.sp_1 = 0b0011_0011;

        assert_eq!(registers.get_r16(0), 0b0100_0000_0000_1111);
        assert_eq!(registers.get_r16(1), 0b0000_0000_1010_1100);
        assert_eq!(registers.get_r16(2), 0b1111_1111_0000_1111);
        assert_eq!(registers.get_r16(3), 0b1100_1100_0011_0011);
    }

    #[test]
    fn test_set_r16() {
        let mut registers = Registers::new();
        assert_eq!(registers.d, 0);
        assert_eq!(registers.e, 0);
        assert_eq!(registers.get_r16(1), 0);

        registers.set_r16(1, 0b0101_1010_1111_0000);

        assert_eq!(registers.d, 0b0101_1010);
        assert_eq!(registers.e, 0b1111_0000);
        assert_eq!(registers.get_r16(1), 0b0101_1010_1111_0000);
    }

    #[test]
    fn test_cc() {
        let mut registers = Registers::new();
        registers.f = 0b0000_0000;
        assert!(registers.cc(0), "NZ should be true as Zero is not set");
        assert!(!registers.cc(1), "Z should be false as Zero is not set");
        assert!(registers.cc(2), "NC should be true as Carry is not set");
        assert!(!registers.cc(3), "C should be false as Carry is not set");

        registers.f = 0b1000_0000;
        assert!(!registers.cc(0), "NZ should be false as Zero is set");
        assert!(registers.cc(1), "Z should be true as Zero is set");
        assert!(registers.cc(2), "NC should be true as Carry is not set");
        assert!(!registers.cc(3), "C should be false as Carry is not set");

        registers.f = 0b1001_0000;
        assert!(!registers.cc(0), "NZ should be false as Zero is set");
        assert!(registers.cc(1), "Z should be true as Zero is set");
        assert!(!registers.cc(2), "NC should be false as Carry is set");
        assert!(registers.cc(3), "C should be true as Carry is set");
    }
}
