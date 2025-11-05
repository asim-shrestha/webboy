use std::ptr::fn_addr_eq;
use crate::ram::Interrupt;
use super::register::{Flag, Registers};
use super::{carry, Ime};
use super::super::ram::{Ram, RamOperations};
use super::{CPU, Mode};

type Instruction = u8;
type InstructionHandler = fn(&mut CPU, instruction: &Instruction) -> ();

trait InstructionOps {
	fn first_u3(&self) -> u8;
	fn middle_u3(&self) -> u8;
	fn last_u3(&self) -> u8;
	fn interleaved_r16(&self, is_zero_indexed: bool) -> u8;
}

type CycleCount = u8;

impl InstructionOps for Instruction {
	fn first_u3(&self) -> u8 {
		(self >> 6) & 0b0000_0111u8
	}

	fn middle_u3(&self) -> u8 {
		(self >> 3) & 0b00000111u8
	}

	fn last_u3(&self) -> u8 {
		self & 0b00000111u8
	}

	fn interleaved_r16(&self, is_zero_indexed: bool) -> u8 {
		let digits = self.middle_u3();
		if is_zero_indexed || digits == 0 {
			digits / 2
		} else {
			(digits - 1) / 2
		}
	}
}

trait FollowingBytesOps {
	fn n8(&self) -> u8;
	fn n16(&self) -> u16;
}

impl CPU {
	pub fn new() -> Self {
		CPU {
			registers: Registers::new(),
			ram: Ram::new(),
			ime: Ime::Off,
			cycle_count: 0,
			mode: Mode::NormalSpeed,
			halt_bug_active: false,
		}
	}

	pub fn new_with_ram(ram: Ram) -> Self {
		CPU {
			registers: Registers::new(),
			ram,
			ime: Ime::Off,
			cycle_count: 0,
			mode: Mode::NormalSpeed,
			halt_bug_active: false,
		}
	}

	fn print_cpu(&self) {
		println!(
			"A:{:02X} F:{:02X} B:{:02X} C:{:02X} D:{:02X} E:{:02X} H:{:02X} L:{:02X} SP:{:04X} PC:{:04X} PCMEM:{:02X},{:02X},{:02X},{:02X}",
			self.registers.a,
			self.registers.f,
			self.registers.b,
			self.registers.c,
			self.registers.d,
			self.registers.e,
			self.registers.h,
			self.registers.l,
			self.registers.get_sp(),
			self.registers.pc,
			self.ram[self.registers.pc as usize],
			self.ram[(self.registers.pc + 1) as usize],
			self.ram[(self.registers.pc + 2) as usize],
			self.ram[(self.registers.pc + 3) as usize]
		)
	}

	fn read_byte(&mut self) -> u8 {
		let data = self.ram[self.registers.pc as usize];
		self.registers.pc += 1;

		data
	}

	fn read_two_bytes(&mut self) -> u16 {
		let data = u16::from_le_bytes([self.ram[self.registers.pc as usize], self.ram[(self.registers.pc + 1) as usize]]);
		self.registers.pc += 2;

		data
	}

	pub fn boot(&mut self) {
		self.registers.a = 0x01;
		self.registers.f = 0xB0;
		self.registers.b = 0x00;
		self.registers.c = 0x13;
		self.registers.d = 0x00;
		self.registers.e = 0xD8;
		self.registers.h = 0x01;
		self.registers.l = 0x4D;
		self.registers.set_sp(0xFFFE);
		self.registers.pc = 0x0100;
	}

	pub fn execute(&mut self, should_print: bool) {
		if should_print {
			self.print_cpu();
		}

		let operation = self.get_operation();
		self.run_operation(operation);
	}

	fn get_operation(&mut self) -> (Instruction, InstructionHandler) {
		let instruction = self.read_byte();

		if self.halt_bug_active {
			// The halt bug will cause the immediate next instruction to be read twice by failing to increment the PC
			self.registers.pc = self.registers.pc.wrapping_sub(1);
			self.halt_bug_active = false;
		}

		if instruction == 0o313 { return self.get_cb_operation(); }

		let function = match instruction {
			0o000 => CPU::no_op,
			0o020 => CPU::stop,
			0o010 => CPU::ld_a16_sp,
			0o067 => CPU::scf,
			0o077 => CPU::ccf,
			0o166 => CPU::halt,
			i if i.first_u3() == 0 && i.middle_u3() % 2 == 0 && i.last_u3() == 3 => CPU::inc_r16,
			i if i.first_u3() == 0 && i.middle_u3() % 2 == 1 && i.last_u3() == 3 => CPU::dec_r16,
			0o064 => CPU::inc_hl,
			i if i.first_u3() == 0 && i.last_u3() == 4 => CPU::inc_r8,
			0o065 => CPU::dec_hl,
			i if i.first_u3() == 0 && i.last_u3() == 5 => CPU::dec_r8,
			i if i.first_u3() == 0 && i.middle_u3() == 6 && i.last_u3() == 6 => CPU::ld_hl_n8,
			i if i.first_u3() == 0 && i.last_u3() == 6 => CPU::ld_r8_n8,
			i if i.first_u3() == 0 && i.middle_u3() % 2 == 0 && i.last_u3() == 1 => CPU::ld_r16_n16,
			i if i.first_u3() == 0 && i.middle_u3() % 2 == 1 && i.last_u3() == 1 => CPU::add_hl_r16,
			0o002 => CPU::ld_r16_a,
			0o012 => CPU::ld_a_r16,
			0o022 => CPU::ld_r16_a,
			0o032 => CPU::ld_a_r16,
			0o042 => CPU::ld_hli_a,
			0o052 => CPU::ld_a_hli,
			0o062 => CPU::ld_hld_a,
			0o072 => CPU::ld_a_hld,
			0o007 => CPU::rlc_a,
			0o017 => CPU::rrc_a,
			0o027 => CPU::rl_a,
			0o037 => CPU::rr_a,
			0o047 => CPU::daa,
			0o057 => CPU::cpl,
			0o030 => CPU::jr_n16,
			0o040 => CPU::jr_cc_n16,
			0o050 => CPU::jr_cc_n16,
			0o060 => CPU::jr_cc_n16,
			0o070 => CPU::jr_cc_n16,
			i if i.first_u3() == 0 && i.last_u3() == 2 => CPU::ld_r16_a,
			i if i.first_u3() == 1 && i.middle_u3() == 6 => CPU::ld_hl_r8,
			i if i.first_u3() == 1 && i.last_u3() == 6 => CPU::ld_r8_hl,
			i if i.first_u3() == 1 => CPU::ld_r8_r8,
			0o206 => CPU::add_a_hl,
			0o200..=0o207 => CPU::add_a_r8,
			0o216 => CPU::addc_a_hl,
			0o210..=0o217 => CPU::addc_a_r8,
			0o226 => CPU::sub_a_hl,
			0o220..=0o227 => CPU::sub_a_r8,
			0o236 => CPU::subc_a_hl,
			0o230..=0o237 => CPU::subc_a_r8,
			0o246 => CPU::and_a_hl,
			0o240..=0o247 => CPU::and_a_r8,
			0o256 => CPU::xor_a_hl,
			0o250..=0o257 => CPU::xor_a_r8,
			0o266 => CPU::or_a_hl,
			0o260..=0o267 => CPU::or_a_r8,
			0o276 => CPU::cp_a_hl,
			0o270..=0o277 => CPU::cp_a_r8,
			0o303 => CPU::jp_n16,
			0o304 => CPU::call_cc_n16,
			0o314 => CPU::call_cc_n16,
			0o324 => CPU::call_cc_n16,
			0o334 => CPU::call_cc_n16,
			0o300 => CPU::ret_cc,
			0o310 => CPU::ret_cc,
			0o320 => CPU::ret_cc,
			0o330 => CPU::ret_cc,
			0o311 => CPU::ret,
			0o331 => CPU::reti,
			0o307 => CPU::rst,
			0o317 => CPU::rst,
			0o327 => CPU::rst,
			0o337 => CPU::rst,
			0o347 => CPU::rst,
			0o357 => CPU::rst,
			0o367 => CPU::rst,
			0o377 => CPU::rst,
			0o306 => CPU::add_a_n8,
			0o316 => CPU::addc_a_n8,
			0o326 => CPU::sub_a_n8,
			0o336 => CPU::subc_a_n8,
			0o346 => CPU::and_a_n8,
			0o301 => CPU::pop_r16,
			0o321 => CPU::pop_r16,
			0o341 => CPU::pop_r16,
			0o350 => CPU::add_sp_e8,
			0o351 => CPU::jp_hl,
			0o361 => CPU::pop_r16,
			0o363 => CPU::di,
			0o370 => CPU::ld_hl_sp_plus_e8,
			0o371 => CPU::ld_sp_hl,
			0o373 => CPU::ei,
			0o305 => CPU::push_r16,
			0o315 => CPU::call_n16,
			0o325 => CPU::push_r16,
			0o335 => panic!("Attempted to run instruction 0o335, which the CPU does not support"),
			0o345 => CPU::push_r16,
			0o355 => panic!("Attempted to run instruction 0o355, which the CPU does not support"),
			0o365 => CPU::push_r16,
			0o375 => panic!("Attempted to run instruction 0o375, which the CPU does not support"),
			0o302 => CPU::jp_cc_n16,
			0o312 => CPU::jp_cc_n16,
			0o322 => CPU::jp_cc_n16,
			0o332 => CPU::jp_cc_n16,
			0o356 => CPU::xor_a_n8,
			0o366 => CPU::or_a_n8,
			0o376 => CPU::cp_a_n8,
			0o340 => CPU::ldh_n16_a,
			0o342 => CPU::ldh_c_a,
			0o352 => CPU::ld_n16_a,
			0o360 => CPU::ldh_a_n16,
			0o362 => CPU::ldh_a_c,
			0o372 => CPU::ld_a_n16,
			_ => panic!("Unhandled instruction: {instruction}"),
		};

		(instruction, function)
	}

	fn run_operation(&mut self, data: (Instruction, InstructionHandler)) {
		let (instruction, op) = data;
		op(self, &instruction);

		// ime flag setting has a one instruction delay
		if !fn_addr_eq(op, CPU::ei as InstructionHandler) && self.ime == Ime::ToSet {
			self.ime = Ime::Set;
		}
	}

	fn get_cb_operation(&mut self) -> (Instruction, InstructionHandler) {
		let instruction = self.read_byte();

		let op = match instruction {
			i if i.first_u3() == 0 && i.middle_u3() == 0 && i.last_u3() == 6 => CPU::rlc_hl,
			i if i.first_u3() == 0 && i.middle_u3() == 0 => CPU::rlc_r8,
			i if i.first_u3() == 0 && i.middle_u3() == 1 && i.last_u3() == 6 => CPU::rrc_hl,
			i if i.first_u3() == 0 && i.middle_u3() == 1 => CPU::rrc_r8,
			i if i.first_u3() == 0 && i.middle_u3() == 2 && i.last_u3() == 6 => CPU::rl_hl,
			i if i.first_u3() == 0 && i.middle_u3() == 2 => CPU::rl_r8,
			i if i.first_u3() == 0 && i.middle_u3() == 3 && i.last_u3() == 6 => CPU::rr_hl,
			i if i.first_u3() == 0 && i.middle_u3() == 3 => CPU::rr_r8,
			i if i.first_u3() == 0 && i.middle_u3() == 4 && i.last_u3() == 6 => CPU::sla_hl,
			i if i.first_u3() == 0 && i.middle_u3() == 4 => CPU::sla_r8,
			i if i.first_u3() == 0 && i.middle_u3() == 5 && i.last_u3() == 6 => CPU::sra_hl,
			i if i.first_u3() == 0 && i.middle_u3() == 5 => CPU::sra_r8,
			i if i.first_u3() == 0 && i.middle_u3() == 6 && i.last_u3() == 6 => CPU::swap_hl,
			i if i.first_u3() == 0 && i.middle_u3() == 6 => CPU::swap_r8,
			i if i.first_u3() == 0 && i.middle_u3() == 7 && i.last_u3() == 6 => CPU::srl_hl,
			i if i.first_u3() == 0 && i.middle_u3() == 7 => CPU::srl_r8,
			i if i.first_u3() == 1 && i.last_u3() == 6 => CPU::bit_u3_hl,
			i if i.first_u3() == 1 => CPU::bit_u3_r8,
			i if i.first_u3() == 2 && i.last_u3() == 6 => CPU::res_u3_hl,
			i if i.first_u3() == 2 => CPU::res_u3_r8,
			i if i.first_u3() == 3 && i.last_u3() == 6 => CPU::set_u3_hl,
			i if i.first_u3() == 3 => CPU::set_u3_r8,
			_ => panic!("Unhandled CB instruction: {instruction}"),
		};

		(instruction, op)
	}

	fn no_op(&mut self, _: &Instruction) {}

	fn add_a_r8(&mut self, instruction: &Instruction) {
		let src = *self.registers.get_r8(instruction.last_u3());
		self.alu_add_a(src, false);
	}

	fn add_a_hl(&mut self, _: &Instruction) {
		let src = self.ram[self.registers.get_hl() as usize];
		self.alu_add_a(src, false);
	}

	fn add_a_n8(&mut self, _: &Instruction) {
		let src = self.read_byte();
		self.alu_add_a(src, false);
	}

	fn addc_a_r8(&mut self, instruction: &Instruction) {
		let src = *self.registers.get_r8(instruction.last_u3());
		self.alu_add_a(src, true);
	}

	fn addc_a_hl(&mut self, _: &Instruction) {
		let src = self.ram[self.registers.get_hl() as usize];
		self.alu_add_a(src, true);
	}

	fn addc_a_n8(&mut self, _: &Instruction) {
		let src = self.read_byte();
		self.alu_add_a(src, true);
	}

	fn alu_add_a(&mut self, value: u8, include_carry: bool) {
		let prev = self.registers.a;
		let (res_without_carry, overflowed_without_carry) = self.registers.a.overflowing_add(value);
		let (mut res, mut overflowed) = (res_without_carry, overflowed_without_carry);
		let mut half_carried = carry::is_half_carry(prev, value);
		if include_carry {
			(res, overflowed) = res_without_carry.overflowing_add(self.registers.get_flag(Flag::Carry) as u8);
			half_carried |= carry::is_half_carry(res_without_carry, self.registers.get_flag(Flag::Carry) as u8);
		}
		self.registers.a = res;

		self.registers.set_flag(Flag::Zero, res == 0);
		self.registers.set_flag(Flag::Subtraction, false);
		self.registers.set_flag(Flag::HalfCarry, half_carried);
		self.registers.set_flag(Flag::Carry, overflowed_without_carry || overflowed);
	}

	fn add_hl_r16(&mut self, instruction: &Instruction) {
		let r16 = self.registers.get_r16(instruction.interleaved_r16(false));
		let prev = self.registers.get_hl();
		let overflow_11 = (prev & 0x0FFF) + (r16 & 0x0FFF) > 0x0FFF;
		let (res, overflow_15) = prev.overflowing_add(r16);
		self.registers.set_hl(res);

		self.registers.set_flag(Flag::Subtraction, false);
		self.registers.set_flag(Flag::HalfCarry, overflow_11);
		self.registers.set_flag(Flag::Carry, overflow_15);
	}

	fn sub_a_r8(&mut self, instruction: &Instruction) {
		let src = *self.registers.get_r8(instruction.last_u3());
		self.alu_sub_a(src, false);
	}

	fn sub_a_hl(&mut self, _: &Instruction) {
		let src = self.ram[self.registers.get_hl() as usize];
		self.alu_sub_a(src, false);
	}

	fn sub_a_n8(&mut self, _: &Instruction) {
		let src = self.read_byte();
		self.alu_sub_a(src, false);
	}

	fn subc_a_r8(&mut self, instruction: &Instruction) {
		let src = *self.registers.get_r8(instruction.last_u3());
		self.alu_sub_a(src, true);
	}

	fn subc_a_hl(&mut self, _: &Instruction) {
		let src = self.ram[self.registers.get_hl() as usize];
		self.alu_sub_a(src, true);
	}

	fn subc_a_n8(&mut self, _: &Instruction) {
		let src = self.read_byte();
		self.alu_sub_a(src, true);
	}

	fn alu_sub_a(&mut self, value: u8, include_borrow: bool) {
		let prev = self.registers.a;
		let (res_without_borrow, overflowed_without_borrow) = self.registers.a.overflowing_sub(value);
		let (mut res, mut overflowed) = (res_without_borrow, overflowed_without_borrow);
		let mut half_borrowed = carry::is_half_borrow(prev, value);
		if include_borrow {
			(res, overflowed) = res_without_borrow.overflowing_sub(self.registers.get_flag(Flag::Carry) as u8);
			half_borrowed |= carry::is_half_borrow(res_without_borrow, self.registers.get_flag(Flag::Carry) as u8);
		}
		self.registers.a = res;

		self.registers.set_flag(Flag::Zero, res == 0);
		self.registers.set_flag(Flag::Subtraction, true);
		self.registers.set_flag(Flag::HalfCarry, half_borrowed);
		self.registers.set_flag(Flag::Carry, overflowed_without_borrow || overflowed);
	}

	fn cpl(&mut self, _: &Instruction) {
		self.registers.a = !self.registers.a;
		self.registers.set_flag(Flag::Subtraction, true);
		self.registers.set_flag(Flag::HalfCarry, true);
	}

	fn and_a_r8(&mut self, instruction: &Instruction) {
		let src = *self.registers.get_r8(instruction.last_u3());
		self.alu_and_a(src);
	}

	fn and_a_hl(&mut self, _: &Instruction) {
		let src = self.ram[self.registers.get_hl() as usize];
		self.alu_and_a(src);
	}

	fn and_a_n8(&mut self, _: &Instruction) {
		let src = self.read_byte();
		self.alu_and_a(src);
	}

	fn alu_and_a(&mut self, value: u8) {
		self.registers.a &= value;

		self.registers.set_flag(Flag::Zero, self.registers.a == 0);
		self.registers.set_flag(Flag::Subtraction, false);
		self.registers.set_flag(Flag::HalfCarry, true);
		self.registers.set_flag(Flag::Carry, false);
	}

	fn xor_a_r8(&mut self, instruction: &Instruction) {
		let src = *self.registers.get_r8(instruction.last_u3());
		self.alu_xor_a(src);
	}

	fn xor_a_hl(&mut self, _: &Instruction) {
		let src = self.ram[self.registers.get_hl() as usize];
		self.alu_xor_a(src);
	}

	fn xor_a_n8(&mut self, _: &Instruction) {
		let src = self.read_byte();
		self.alu_xor_a(src);
	}

	fn alu_xor_a(&mut self, value: u8) {
		self.registers.a = self.registers.a ^ value;

		self.registers.set_flag(Flag::Zero, self.registers.a == 0);
		self.registers.set_flag(Flag::Subtraction, false);
		self.registers.set_flag(Flag::HalfCarry, false);
		self.registers.set_flag(Flag::Carry, false);
	}

	fn or_a_r8(&mut self, instruction: &Instruction) {
		let src = *self.registers.get_r8(instruction.last_u3());
		self.alu_or_a(src);
	}

	fn or_a_hl(&mut self, _: &Instruction) {
		let src = self.ram[self.registers.get_hl() as usize];
		self.alu_or_a(src);
	}

	fn or_a_n8(&mut self, _: &Instruction) {
		let src = self.read_byte();
		self.alu_or_a(src);
	}

	fn alu_or_a(&mut self, value: u8) {
		self.registers.a = self.registers.a | value;

		self.registers.set_flag(Flag::Zero, self.registers.a == 0);
		self.registers.set_flag(Flag::Subtraction, false);
		self.registers.set_flag(Flag::HalfCarry, false);
		self.registers.set_flag(Flag::Carry, false);
	}

	fn cp_a_r8(&mut self, instruction: &Instruction) {
		let value = *self.registers.get_r8(instruction.last_u3());
		self.alu_cp_a(value);
	}

	fn cp_a_hl(&mut self, _: &Instruction) {
		let value = self.ram[self.registers.get_hl() as usize];
		self.alu_cp_a(value);
	}

	fn cp_a_n8(&mut self, _: &Instruction) {
		let value = self.read_byte();
		self.alu_cp_a(value);
	}

	fn alu_cp_a(&mut self, value: u8) {
		let (res, _) = self.registers.a.overflowing_sub(value);

		self.registers.set_flag(Flag::Zero, res == 0);
		self.registers.set_flag(Flag::Subtraction, true);
		self.registers.set_flag(Flag::HalfCarry, carry::is_half_borrow(self.registers.a, value));
		self.registers.set_flag(Flag::Carry, value > self.registers.a);
	}

	fn inc_r8(&mut self, instruction: &Instruction) {
		let register = self.registers.get_r8(instruction.middle_u3());
		let prev = *register;
		let (res, _) = register.overflowing_add(1);
		*register = res;

		self.registers.set_flag(Flag::Zero, res == 0);
		self.registers.set_flag(Flag::Subtraction, false);
		self.registers.set_flag(Flag::HalfCarry, carry::is_half_carry(prev, 1));
	}

	fn inc_hl(&mut self, _: &Instruction) {
		let location = self.registers.get_hl();
		let byte = &mut self.ram[location as usize];

		let prev = *byte;
		let (res, _) = byte.overflowing_add(1);
		*byte = res;

		self.registers.set_flag(Flag::Zero, res == 0);
		self.registers.set_flag(Flag::Subtraction, false);
		self.registers.set_flag(Flag::HalfCarry, carry::is_half_carry(prev, 1));
	}

	fn inc_r16(&mut self, instruction: &Instruction) {
		let index = instruction.interleaved_r16(true);
		let r16 = self.registers.get_r16(index);
		let (res, _) = r16.overflowing_add(1);
		self.registers.set_r16(index, res);
	}

	fn dec_r8(&mut self, instruction: &Instruction) {
		let register = self.registers.get_r8(instruction.middle_u3());
		let prev = *register;
		let (res, _) = register.overflowing_sub(1);
		*register = res;

		self.registers.set_flag(Flag::Zero, res == 0);
		self.registers.set_flag(Flag::Subtraction, true);
		self.registers.set_flag(Flag::HalfCarry, carry::is_half_borrow(prev, 1));
	}

	fn dec_hl(&mut self, _: &Instruction) {
		let location = self.registers.get_hl();
		let byte = &mut self.ram[location as usize];

		let prev = *byte;
		let (res, _) = byte.overflowing_sub(1);
		*byte = res;

		self.registers.set_flag(Flag::Zero, res == 0);
		self.registers.set_flag(Flag::Subtraction, true);
		self.registers.set_flag(Flag::HalfCarry, carry::is_half_borrow(prev, 1));
	}

	fn dec_r16(&mut self, instruction: &Instruction) {
		let index = instruction.interleaved_r16(false);
		let r16 = self.registers.get_r16(index);
		let (res, _) = r16.overflowing_sub(1);
		self.registers.set_r16(index, res);
	}

	fn ld_r8_r8(&mut self, instruction: &Instruction) {
		let value_to_load = *self.registers.get_r8(instruction.last_u3());
		let register = self.registers.get_r8(instruction.middle_u3());

		*register = value_to_load;
	}

	fn ld_r8_n8(&mut self, instruction: &Instruction) {
		let value_to_load = self.read_byte();
		let register = self.registers.get_r8(instruction.middle_u3());

		*register = value_to_load;
	}


	fn ld_r16_n16(&mut self, instruction: &Instruction) {
		let value_to_load = self.read_two_bytes();
		let location = instruction.interleaved_r16(true);
		self.registers.set_r16(location, value_to_load);
	}

	fn ld_hl_sp_plus_e8(&mut self, _: &Instruction) {
		let e8 = self.read_byte() as i8;
		let sp = self.registers.get_sp();

		let (_, overflowed) = (sp as u8).overflowing_add(e8 as u8);
		let half_carried = carry::is_half_carry(sp as u8, e8 as u8);

		let res = sp.wrapping_add_signed(e8 as i16);
		self.registers.set_hl(res);

		self.registers.set_flag(Flag::Zero, false);
		self.registers.set_flag(Flag::Subtraction, false);
		self.registers.set_flag(Flag::HalfCarry, half_carried);
		self.registers.set_flag(Flag::Carry, overflowed);
	}

	fn ld_hl_r8(&mut self, instruction: &Instruction) {
		let value_to_load = *self.registers.get_r8(instruction.last_u3());
		let location = self.registers.get_hl();
		self.ram[location as usize] = value_to_load;
	}

	fn ld_hl_n8(&mut self, _: &Instruction) {
		let value_to_load = self.read_byte();
		let location = self.registers.get_hl();
		self.ram[location as usize] = value_to_load;
	}

	fn ld_sp_hl(&mut self, _: &Instruction) {
		let hl = self.registers.get_hl();
		self.registers.set_sp(hl);
	}

	fn ld_r8_hl(&mut self, instruction: &Instruction) {
		let location = self.registers.get_hl();
		let register = self.registers.get_r8(instruction.middle_u3());
		*register = self.ram[location as usize];
	}

	fn ld_r16_a(&mut self, instruction: &Instruction) {
		let value_to_load = self.registers.a;
		let location = self.registers.get_r16(instruction.interleaved_r16(true));
		self.ram[location as usize] = value_to_load;
	}

	fn ld_n16_a(&mut self, _: &Instruction) {
		let value_to_load = self.registers.a;
		let location = self.read_two_bytes();
		self.ram[location as usize] = value_to_load;
	}

	fn ld_a16_sp(&mut self, _: &Instruction) {
		let [lo, hi] = self.registers.get_sp().to_le_bytes();
		let location = self.read_two_bytes() as usize;
		self.ram[location] = lo;
		self.ram[location + 1] = hi;
	}

	fn ldh_n16_a(&mut self, _: &Instruction) {
		let location = self.read_byte();
		let location = 0xFF00 + location as u16;
		let value_to_load = self.registers.a;
		self.ram[location as usize] = value_to_load;
	}

	fn ldh_c_a(&mut self, _: &Instruction) {
		let c = self.registers.c as u16;
		let location = 0xFF00 + c;
		self.ram[location as usize] = self.registers.a;
	}

	fn ld_a_r16(&mut self, instruction: &Instruction) {
		let location = self.registers.get_r16(instruction.interleaved_r16(false));
		self.registers.a = self.ram[location as usize];
	}

	fn ld_a_n16(&mut self, _: &Instruction) {
		let location = self.read_two_bytes();
		self.registers.a = self.ram[location as usize];
	}

	fn ldh_a_n16(&mut self, _: &Instruction) {
		let location = self.read_byte();
		let location = 0xFF00 + location as u16;
		self.registers.a = self.ram[location as usize];
	}

	fn ldh_a_c(&mut self, _: &Instruction) {
		let c = self.registers.c as u16;
		let location = 0xFF00 + c;

		self.registers.a = self.ram[location as usize];
	}

	fn ld_hli_a(&mut self, _: &Instruction) {
		self.ld_hl_r8(&0o167);
		self.inc_r16(&0o043);
	}

	fn ld_hld_a(&mut self, _: &Instruction) {
		self.ld_hl_r8(&0o167);
		self.dec_r16(&0o053);
	}

	fn ld_a_hli(&mut self, _: &Instruction) {
		self.ld_r8_hl(&0o176);
		self.inc_r16(&0o043);
	}

	fn ld_a_hld(&mut self, _: &Instruction) {
		self.ld_r8_hl(&0o176);
		self.dec_r16(&0o053);
	}

	fn ccf(&mut self, _: &Instruction) {
		self.registers.set_flag(Flag::Subtraction, false);
		self.registers.set_flag(Flag::HalfCarry, false);
		self.registers.set_flag(Flag::Carry, !self.registers.get_flag(Flag::Carry));
	}

	fn scf(&mut self, _: &Instruction) {
		self.registers.set_flag(Flag::Subtraction, false);
		self.registers.set_flag(Flag::HalfCarry, false);
		self.registers.set_flag(Flag::Carry, true);
	}

	fn bit_u3_r8(&mut self, instruction: &Instruction) {
		let bit_index = instruction.middle_u3();
		let value = *self.registers.get_r8(instruction.last_u3());

		self.alu_bit_u3(bit_index, value);
	}

	fn bit_u3_hl(&mut self, instruction: &Instruction) {
		let bit_index = instruction.middle_u3();
		let location = self.registers.get_hl();
		let value = self.ram[location as usize];

		self.alu_bit_u3(bit_index, value);
	}

	fn alu_bit_u3(&mut self, bit_index: u8, value: u8) {
		let res = (value >> bit_index) & 1;

		self.registers.set_flag(Flag::Zero, res == 0);
		self.registers.set_flag(Flag::Subtraction, false);
		self.registers.set_flag(Flag::HalfCarry, true);
	}

	fn res_u3_r8(&mut self, instruction: &Instruction) {
		let bit_mask = 1 << instruction.middle_u3();
		let r8 = self.registers.get_r8(instruction.last_u3());

		*r8 &= !bit_mask;
	}

	fn res_u3_hl(&mut self, instruction: &Instruction) {
		let bit_mask = 1 << instruction.middle_u3();
		let location = self.registers.get_hl();
		let memory = &mut self.ram[location as usize];

		*memory &= !bit_mask;
	}

	fn set_u3_r8(&mut self, instruction: &Instruction) {
		let bit_mask = 1 << instruction.middle_u3();
		let r8 = self.registers.get_r8(instruction.last_u3());

		*r8 |= bit_mask;
	}

	fn set_u3_hl(&mut self, instruction: &Instruction) {
		let bit_mask = 1 << instruction.middle_u3();
		let location = self.registers.get_hl();
		let memory = &mut self.ram[location as usize];

		*memory |= bit_mask;
	}

	fn rl_r8(&mut self, instruction: &Instruction) {
		let r8_value = *self.registers.get_r8(instruction.last_u3());
		let res = self.alu_rotate(r8_value, Direction::LEFT, RotateType::RotateThroughCarry);
		*self.registers.get_r8(instruction.last_u3()) = res;
	}

	fn rl_hl(&mut self, _: &Instruction) {
		let value = self.ram[self.registers.get_hl() as usize];
		let res = self.alu_rotate(value, Direction::LEFT, RotateType::RotateThroughCarry);
		self.ram[self.registers.get_hl() as usize] = res;
	}

	fn rl_a(&mut self, _: &Instruction) {
		let value = self.registers.a;
		let res = self.alu_rotate(value, Direction::LEFT, RotateType::RotateThroughCarry);
		self.registers.a = res;

		self.registers.set_flag(Flag::Zero, false);
	}

	fn rlc_r8(&mut self, instruction: &Instruction) {
		let r8_value = *self.registers.get_r8(instruction.last_u3());
		let res = self.alu_rotate(r8_value, Direction::LEFT, RotateType::RotateWithoutCarry);
		*self.registers.get_r8(instruction.last_u3()) = res;
	}

	fn rlc_hl(&mut self, _: &Instruction) {
		let value = self.ram[self.registers.get_hl() as usize];
		let res = self.alu_rotate(value, Direction::LEFT, RotateType::RotateWithoutCarry);
		self.ram[self.registers.get_hl() as usize] = res;
	}

	fn rlc_a(&mut self, _: &Instruction) {
		let value = self.registers.a;
		let res = self.alu_rotate(value, Direction::LEFT, RotateType::RotateWithoutCarry);
		self.registers.a = res;

		self.registers.set_flag(Flag::Zero, false);
	}

	fn sla_r8(&mut self, instruction: &Instruction) {
		let value = *self.registers.get_r8(instruction.last_u3());
		let res = self.alu_rotate(value, Direction::LEFT, RotateType::Shift);
		*self.registers.get_r8(instruction.last_u3()) = res;
	}

	fn sla_hl(&mut self, _: &Instruction) {
		let value = self.ram[self.registers.get_hl() as usize];
		let res = self.alu_rotate(value, Direction::LEFT, RotateType::Shift);
		self.ram[self.registers.get_hl() as usize] = res;
	}

	fn rr_r8(&mut self, instruction: &Instruction) {
		let r8_value = *self.registers.get_r8(instruction.last_u3());
		let res = self.alu_rotate(r8_value, Direction::RIGHT, RotateType::RotateThroughCarry);
		*self.registers.get_r8(instruction.last_u3()) = res;
	}

	fn rr_hl(&mut self, _: &Instruction) {
		let value = self.ram[self.registers.get_hl() as usize];
		let res = self.alu_rotate(value, Direction::RIGHT, RotateType::RotateThroughCarry);
		self.ram[self.registers.get_hl() as usize] = res;
	}

	fn rr_a(&mut self, _: &Instruction) {
		let value = self.registers.a;
		let res = self.alu_rotate(value, Direction::RIGHT, RotateType::RotateThroughCarry);
		self.registers.a = res;

		self.registers.set_flag(Flag::Zero, false);
	}

	fn rrc_r8(&mut self, instruction: &Instruction) {
		let r8_value = *self.registers.get_r8(instruction.last_u3());
		let res = self.alu_rotate(r8_value, Direction::RIGHT, RotateType::RotateWithoutCarry);
		*self.registers.get_r8(instruction.last_u3()) = res;
	}

	fn rrc_hl(&mut self, _: &Instruction) {
		let value = self.ram[self.registers.get_hl() as usize];
		let res = self.alu_rotate(value, Direction::RIGHT, RotateType::RotateWithoutCarry);
		self.ram[self.registers.get_hl() as usize] = res;
	}

	fn rrc_a(&mut self, _: &Instruction) {
		let value = self.registers.a;
		let res = self.alu_rotate(value, Direction::RIGHT, RotateType::RotateWithoutCarry);
		self.registers.a = res;

		self.registers.set_flag(Flag::Zero, false);
	}

	fn sra_r8(&mut self, instruction: &Instruction) {
		let value = *self.registers.get_r8(instruction.last_u3());
		let mut res = self.alu_rotate(value, Direction::RIGHT, RotateType::Shift);

		let bit_7_mask = value & 0b1000_0000;
		res = if bit_7_mask > 0 { res | bit_7_mask } else { res & !bit_7_mask };

		*self.registers.get_r8(instruction.last_u3()) = res;
	}

	fn sra_hl(&mut self, _: &Instruction) {
		let value = self.ram[self.registers.get_hl() as usize];
		let mut res = self.alu_rotate(value, Direction::RIGHT, RotateType::Shift);

		let bit_7_mask = value & 0b1000_0000;
		res = if bit_7_mask > 0 { res | bit_7_mask } else { res & !bit_7_mask };

		self.ram[self.registers.get_hl() as usize] = res;
	}

	fn srl_r8(&mut self, instruction: &Instruction) {
		let value = *self.registers.get_r8(instruction.last_u3());
		let res = self.alu_rotate(value, Direction::RIGHT, RotateType::Shift);
		*self.registers.get_r8(instruction.last_u3()) = res;
	}

	fn srl_hl(&mut self, _: &Instruction) {
		let value = self.ram[self.registers.get_hl() as usize];
		let res = self.alu_rotate(value, Direction::RIGHT, RotateType::Shift);
		self.ram[self.registers.get_hl() as usize] = res;
	}

	fn alu_rotate(&mut self, value: u8, direction: Direction, rotate_type: RotateType) -> u8 {
		let carry_value = self.registers.get_flag(Flag::Carry);
		let popped_value = if direction == Direction::LEFT { value >> 7 } else { value & 1 };
		let mut value = if direction == Direction::LEFT { value << 1 } else { value >> 1 };

		let modifier_value = match rotate_type {
			RotateType::Shift => 0,
			RotateType::RotateWithoutCarry => popped_value,
			RotateType::RotateThroughCarry => carry_value as u8,
		};

		value |= if direction == Direction::LEFT { modifier_value } else { modifier_value << 7 };

		self.registers.set_flag(Flag::Zero, value == 0);
		self.registers.set_flag(Flag::Subtraction, false);
		self.registers.set_flag(Flag::HalfCarry, false);
		self.registers.set_flag(Flag::Carry, popped_value == 1);

		value
	}

	fn swap_r8(&mut self, instruction: &Instruction) {
		let value = *self.registers.get_r8(instruction.last_u3());
		let res = self.alu_swap(value);
		*self.registers.get_r8(instruction.last_u3()) = res;
	}

	fn swap_hl(&mut self, _: &Instruction) {
		let value = self.ram[self.registers.get_hl() as usize];
		let res = self.alu_swap(value);
		self.ram[self.registers.get_hl() as usize] = res;
	}

	fn alu_swap(&mut self, value: u8) -> u8 {
		let lower_shifted = (value & 0b0000_1111) << 4;
		let upper_shifted = (value & 0b1111_0000) >> 4;
		let res = lower_shifted | upper_shifted;

		self.registers.set_flag(Flag::Zero, res == 0);
		self.registers.set_flag(Flag::Subtraction, false);
		self.registers.set_flag(Flag::HalfCarry, false);
		self.registers.set_flag(Flag::Carry, false);

		res
	}

	fn call_n16(&mut self, _: &Instruction) {
		let n16 = self.read_two_bytes();
		self.stack_push_16(self.registers.pc);
		self.registers.pc = n16;
	}

	fn call_cc_n16(&mut self, instruction: &Instruction) {
		let n16 = self.read_two_bytes();
		if self.registers.cc(instruction.middle_u3()) {
			self.stack_push_16(self.registers.pc);
			self.registers.pc = n16;
		}
	}

	fn ret_cc(&mut self, instruction: &Instruction) {
		if self.registers.cc(instruction.middle_u3()) {
			let new_pc = self.stack_pop_16();
			self.registers.pc = new_pc;
		}
	}

	fn ret(&mut self, _: &Instruction) {
		let new_pc = self.stack_pop_16();
		self.registers.pc = new_pc;
	}

	fn reti(&mut self, _: &Instruction) {
		let new_pc = self.stack_pop_16();
		self.registers.pc = new_pc;
		self.ime = Ime::Set;
	}

	fn rst(&mut self, instruction: &Instruction) {
		let addr = CPU::get_rst_address(instruction.middle_u3());
		self.stack_push_16(self.registers.pc);
		self.registers.pc = addr;
	}

	fn get_rst_address(value: u8) -> u16 {
		match value {
			0 => 0x00,
			1 => 0x08,
			2 => 0x10,
			3 => 0x18,
			4 => 0x20,
			5 => 0x28,
			6 => 0x30,
			7 => 0x38,
			_ => panic!("Invalid RST instruction value: {:#04X}", value),
		}
	}

	fn push_r16(&mut self, instruction: &Instruction) {
		let index = instruction.interleaved_r16(true);
		let value = if index == 3 {
			u16::from_be_bytes([self.registers.a, self.registers.f & 0xF0])
		} else {
			self.registers.get_r16(index)
		};

		self.stack_push_16(value);
	}

	fn stack_push_16(&mut self, value: u16) {
		let bytes = value.to_le_bytes();

		let mut sp = self.registers.get_sp();
		sp = sp.wrapping_sub(2);

		self.ram[sp as usize] = bytes[0];
		self.ram[(sp + 1) as usize] = bytes[1];

		self.registers.set_sp(sp);
	}

	fn pop_r16(&mut self, instruction: &Instruction) {
		let index = instruction.interleaved_r16(true);
		let value = self.stack_pop_16();

		if index == 3 {
			// Pop instruction will place in AF instead of SP
			let bytes: [u8; 2] = value.to_le_bytes();
			self.registers.a = bytes[1];
			self.registers.f = bytes[0] & 0xF0; // Lower nibble of F is always 0
		} else {
			self.registers.set_r16(index, value);
		}
	}

	fn stack_pop_16(&mut self) -> u16 {
		let mut sp = self.registers.get_sp();
		let values = [self.ram[sp as usize], self.ram[(sp + 1) as usize]];
		let value = u16::from_le_bytes(values);
		sp = sp.wrapping_add(2);
		self.registers.set_sp(sp);

		value
	}


	fn jp_hl(&mut self, _: &Instruction) {
		let hl = self.registers.get_hl();
		self.registers.pc = hl;
	}

	fn jp_n16(&mut self, _: &Instruction) {
		let n16 = self.read_two_bytes();
		self.registers.pc = n16;
	}

	fn jp_cc_n16(&mut self, instruction: &Instruction) {
		let n16 = self.read_two_bytes();
		if self.registers.cc(instruction.middle_u3()) {
			self.registers.pc = n16;
		}
	}

	fn jr_n16(&mut self, _: &Instruction) {
		let byte = self.read_byte();
		self.handle_jr(byte);
	}

	fn jr_cc_n16(&mut self, instruction: &Instruction) {
		let byte = self.read_byte();
		let cc_offset = 4;
		if self.registers.cc(instruction.middle_u3() - cc_offset) {
			self.handle_jr(byte);
		}
	}

	fn handle_jr(&mut self, byte: u8) {
		let offset = (byte as i8) as i16; // Casting to i8 will convert to negative

		let new_addr = if offset.is_negative() {
			self.registers.pc.wrapping_sub(offset.abs() as u16)
		} else {
			self.registers.pc.wrapping_add(offset as u16)
		};

		self.registers.pc = new_addr;
	}

	fn add_sp_e8(&mut self, _: &Instruction) {
		let e8 = self.read_byte() as i8;
		let sp = self.registers.get_sp();

		let (_, overflowed) = (sp as u8).overflowing_add(e8 as u8);
		let res = sp.wrapping_add_signed(e8 as i16);
		let half_carried = carry::is_half_carry(sp as u8, e8 as u8);
		self.registers.set_sp(res);

		self.registers.set_flag(Flag::Zero, false);
		self.registers.set_flag(Flag::Subtraction, false);
		self.registers.set_flag(Flag::HalfCarry, half_carried);
		self.registers.set_flag(Flag::Carry, overflowed);
	}

	fn dec_sp(&mut self, _: Instruction) {
		let prev = self.registers.get_sp();
		let res = prev.wrapping_sub(1);
		self.registers.set_sp(res);
	}

	fn di(&mut self, _: &Instruction) {
		self.ime = Ime::Off;
	}

	fn ei(&mut self, _: &Instruction) {
		self.ime = Ime::ToSet;
	}

	fn halt(&mut self, _: &Instruction) {
		// TODO finalize
		// IME is set
		if self.ime == Ime::Set {
			self.mode = Mode::LowPower;
			return;
		}

		// IME not set and no interrupts pending
		if self.ram.pending_interrupt().is_none() {
			self.mode = Mode::LowPower;
			return;
		}

		// IME not set and interrupts are pending
		if self.ram.pending_interrupt().is_some() {
			// TODO: Handle halt bug properly
			self.halt_bug_active = true;
			return;
		}
	}

	fn stop(&mut self, _: &Instruction) {
		self.mode = Mode::VeryLowPower;
	}

	fn daa(&mut self, _: &Instruction) {
		let mut adjustment = 0;
		let mut new_carry_flag = false;

		if self.registers.get_flag(Flag::Subtraction) {
			// TODO: Do i need to calculate carry flag?
			adjustment += if self.registers.get_flag(Flag::HalfCarry) { 0x6 } else { 0 };
			adjustment += if self.registers.get_flag(Flag::Carry) { 0x60 } else { 0 };
			self.registers.a = self.registers.a.wrapping_sub(adjustment);
			new_carry_flag = self.registers.get_flag(Flag::Carry);
		} else {
			adjustment += if self.registers.get_flag(Flag::HalfCarry) || (self.registers.a & 0xF > 0x9) { 0x6 } else { 0 };
			adjustment += if self.registers.get_flag(Flag::Carry) || (self.registers.a > 0x99) {
				new_carry_flag = true;
				0x60
			} else { 0 };
			self.registers.a = self.registers.a.wrapping_add(adjustment);
		}

		self.registers.set_flag(Flag::Zero, self.registers.a == 0);
		self.registers.set_flag(Flag::HalfCarry, false);
		self.registers.set_flag(Flag::Carry, new_carry_flag);

	}

	fn handle_interrupt(&mut self) {
		if self.ime != Ime::Set {
			return;
		}

		let pending_interrupt = self.ram.pending_interrupt();

		if let Some(interrupt) = pending_interrupt {
			self.stack_push_16(self.registers.pc);
			self.registers.pc = interrupt.handler_address();
			self.ime = Ime::Off;
			self.ram.clear_interrupt(interrupt);
		}

	}
}

#[derive(PartialEq, Eq)]
enum Direction {
	LEFT,
	RIGHT,
}

enum RotateType {
	Shift,
	RotateWithoutCarry,
	RotateThroughCarry,
}

#[cfg(test)]
mod test {
	use super::*;
	use super::super::super::ram::{RamOperations, TestRamOperations};
	use std::ptr::fn_addr_eq;
	use crate::cpu::Mode::{LowPower, NormalSpeed, VeryLowPower};

	#[test]
	fn test_new_cpu() {
		let cpu = CPU::new();
		assert_eq!(cpu.registers.f, 0, "No flags should be set to start");
		assert_eq!(cpu.registers.pc, 0, "PC should be 0 to start");
		assert!(cpu.ram.iter().all(|value| *value == 0), "Ram should be empty to start");
	}

	#[test]
	fn test_get_operation() {
		let mut cpu = CPU::new();
		let (_, operation) = cpu.get_operation();
		assert!(fn_addr_eq(
			operation,
			CPU::no_op as InstructionHandler
		));
	}

	#[test]
	fn test_get_instruction_value() {
		let instruction: Instruction = 0b_1100_0111;
		assert_eq!(instruction.last_u3(), 7u8);
	}

	#[test]
	fn test_pc() {
		let mut cpu = CPU::new();
		cpu.ram[0] = 0b0000_1001;
		cpu.ram[1] = 0b1100_1100;
		cpu.ram[2] = 0b1101_1100;
		cpu.ram[3] = 0b0001_1100;

		assert_eq!(cpu.registers.pc, 0);
		assert_eq!(cpu.read_byte(), 0b0000_1001);
		assert_eq!(cpu.registers.pc, 1);

		assert_eq!(cpu.read_two_bytes(), 0b1101_1100_1100_1100);
		assert_eq!(cpu.registers.pc, 3);

		assert_eq!(cpu.read_byte(), 0b0001_1100);
		assert_eq!(cpu.registers.pc, 4);
	}

	#[test]
	fn test_add_a_r8() {
		// Add 0 to 'A' register
		let mut cpu = CPU::new();
		cpu.ram[0] = 0o200;
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::add_a_r8 as InstructionHandler));

		// Actually run the above instruction
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.a, 0);
		assert_eq!(cpu.registers.f, 0b1000_0000, "Expected just the zero flag to be set");

		// Add 4 to 'A' register
		cpu.registers.b = 0b1100_0100;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.a, 0b1100_0100);
		assert_eq!(cpu.registers.f, 0b0000_0000, "No flags should be set again");

		// Add 4 to 'A' register again, causing an overflow
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.a, 0b1000_1000);
		assert_eq!(cpu.registers.f, 0b0001_0000, "Expected only the carry flag to be set");

		// Induce a half carry
		cpu.registers.b = 0b0000_1000;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.a, 0b1001_0000);
		assert_eq!(cpu.registers.f, 0b0010_0000, "Expected only the half carry flag to be set");
	}

	#[test]
	fn test_addc_a_a8() {
		let mut cpu = CPU::new();
		cpu.ram[0] = 0o210; // ADC A, B
		let (instruction, operation) = cpu.get_operation();
		assert!(fn_addr_eq(operation, CPU::addc_a_r8 as InstructionHandler));

		// Addition without carry flag
		cpu.registers.b = 1;
		operation(&mut cpu, &instruction);
		assert_eq!(cpu.registers.a, 0 + 1, "The carry bit should not have been added");

		// Addition with carry flag
		cpu.registers.set_flag(Flag::Carry, true);
		operation(&mut cpu, &instruction);
		assert_eq!(cpu.registers.a, 1 + 2, "The carry bit should have been added");
	}

	#[test]
	fn test_add_hl_r16() {
		let mut cpu = CPU::new();
		cpu.ram[0] = 0o031;
		let (instruction, operation) = cpu.get_operation();
		assert!(fn_addr_eq(operation, CPU::add_hl_r16 as InstructionHandler));

		// Half carry test
		cpu.registers.set_hl(0b0000_1000_0000_0000);
		cpu.registers.set_r16(1, 0b1000_1000_0000_0000);
		operation(&mut cpu, &instruction);
		assert_eq!(cpu.registers.get_hl(), 0b1001_0000_0000_0000);
		assert_eq!(cpu.registers.f, 0b0010_0000, "Expected only the half carry flag to be set");

		// Fully carry test
		operation(&mut cpu, &instruction);
		assert_eq!(cpu.registers.get_hl(), 0b0001_1000_0000_0000);
		assert_eq!(cpu.registers.f, 0b0001_0000, "Expected only the carry flag to be set");
	}

	#[test]
	fn test_add_sp_e8() {
		let mut cpu = CPU::new();

		cpu.ram.test_load(0, vec![0o350, 0x0F, 0o350, 1, 0o350, 1, 0o350, 0b1000_0000]);
		let (instruction, operation) = cpu.get_operation();
		assert!(fn_addr_eq(operation, CPU::add_sp_e8 as InstructionHandler));

		// No carry test
		cpu.registers.set_sp(0x1000);
		operation(&mut cpu, &instruction);
		assert_eq!(cpu.registers.get_sp(), 0x100F);
		assert_eq!(cpu.registers.f, 0b0000_0000, "No flags should be set");

		// Half carry test
		let (instruction, operation) = cpu.get_operation();
		operation(&mut cpu, &instruction);
		assert_eq!(cpu.registers.get_sp(), 0x1010);
		assert_eq!(cpu.registers.f, 0b0010_0000, "Expected only the half carry flag to be set");

		// Full carry test
		cpu.registers.set_sp(0x10FF);
		let (instruction, operation) = cpu.get_operation();
		operation(&mut cpu, &instruction);
		assert_eq!(cpu.registers.get_sp(), 0x1100);
		assert_eq!(cpu.registers.f, 0b0011_0000, "Expected both carry and half carry flags to be set");

		// Subtraction test
		let (instruction, operation) = cpu.get_operation();
		operation(&mut cpu, &instruction);
		assert_eq!(cpu.registers.get_sp(), 0x1100 - 0x80);
		assert_eq!(cpu.registers.f, 0b0000_0000, "No flags set");
	}

	#[test]
	fn test_sub_a_r8() {
		let mut cpu = CPU::new();
		cpu.ram[0] = 0o220;
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::sub_a_r8 as InstructionHandler));

		// Subtract 0s
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.a, 0);
		assert_eq!(cpu.registers.f, 0b1100_0000, "Zero and subtract flag should be set");

		// Subtract without borrow
		cpu.registers.a = 0b0001_0100;
		cpu.registers.b = 0b0000_0100;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.a, 0b0001_0000);
		assert_eq!(cpu.registers.f, 0b0100_0000, "Only the subtract flag should be set");

		// Subtract with lower nibble borrow
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.a, 0b0000_1100);
		assert_eq!(cpu.registers.f, 0b0110_0000, "Subtract and half carry flag should be set");

		// Full underflow without lower nibble
		cpu.registers.b = 0b0001_0000;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.a, 0b1111_1100);
		assert_eq!(cpu.registers.f, 0b0101_0000, "Subtract and carry flags should be set but not the half");
	}
	//
	#[test]
	fn test_subc_a_a8() {
		let mut cpu = CPU::new();
		cpu.ram[0] = 0o235;
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::subc_a_r8 as InstructionHandler));

		// Subtraction without carry flag
		cpu.registers.a = 10;
		cpu.registers.l = 1;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.a, 10 - 1, "The carry bit should not have been subtracted");

		// Subtraction with carry flag
		cpu.registers.set_flag(Flag::Carry, true);
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.a, 9 - 2, "The carry bit should have been subtracted");
	}

	#[test]
	fn test_cpl() {
		let mut cpu = CPU::new();
		cpu.ram[0] = 0o057;
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::cpl as InstructionHandler));
		cpu.registers.f = 0b1111_0000; // Pre-set all flag bits

		let value = 0b0101_1100;
		cpu.registers.a = value;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.a, !value);
		assert_eq!(cpu.registers.f, 0b1111_0000, "CPL should only force the half carry and subtraction flags");

		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.a, value);
	}

	#[test]
	fn test_and() {
		let mut cpu = CPU::new();
		cpu.ram[0] = 0o241;
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::and_a_r8 as InstructionHandler));
		cpu.registers.f = 0b0101_0000; // Test that these values get overrode

		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.a, 0, "A is 0 to start");
		assert_eq!(cpu.registers.f, 0b1010_0000, "The zero and half carry flags should be set");

		cpu.registers.a = 0b0001_1000;
		cpu.registers.c = cpu.registers.a;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.a, 0b0001_1000);
		assert_eq!(cpu.registers.f, 0b0010_0000, "Only the half carry flags should be set");

		cpu.registers.c = 0b0001_0111;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.a, 0b0001_0000);
	}

	#[test]
	fn test_xor_a_r8() {
		let mut cpu = CPU::new();
		cpu.ram[0] = 0o252;
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::xor_a_r8 as InstructionHandler));
		cpu.registers.f = 0b1111_0000; // Test that these values get overrode

		// 0 XOR 0
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.a, 0, "A is 0 to start");
		assert_eq!(cpu.registers.f, 0b1000_0000, "No flags set at the start");

		// Test basic XOR
		cpu.registers.a = 0b1101_1010;
		cpu.registers.d = 0b0011_0110;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.a, 0b1110_1100);
		assert_eq!(cpu.registers.d, 0b0011_0110, "D remains unchanged");
		assert_eq!(cpu.registers.f, 0b0000_0000, "No flags should be set");

		// X XOR X == 0
		op(&mut cpu, &0o257);
		assert_eq!(cpu.registers.a, 0);
	}

	#[test]
	fn test_or_a_r8() {
		let mut cpu = CPU::new();
		cpu.ram[0] = 0o263;
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::or_a_r8 as InstructionHandler));
		cpu.registers.f = 0b1111_0000; // Test that these values get overrode

		// 0 OR 0
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.a, 0, "A is 0 to start");
		assert_eq!(cpu.registers.f, 0b1000_0000, "No flags set at the start");

		// Test basic OR
		cpu.registers.a = 0b1101_1010;
		cpu.registers.e = 0b0010_0101;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.a, 0b1111_1111, "All bits should be set");
		assert_eq!(cpu.registers.e, 0b0010_0101, "D remains unchanged");
		assert_eq!(cpu.registers.f, 0b0000_0000, "No flags should be set");

		// X OR X == X
		op(&mut cpu, &0o267);
		assert_eq!(cpu.registers.a, 0b1111_1111);
	}

	#[test]
	fn test_inc() {
		let mut cpu = CPU::new();
		cpu.ram[0] = 0o04;
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::inc_r8 as InstructionHandler));

		cpu.registers.a = 1;
		cpu.registers.b = 14;
		op(&mut cpu, &0o04);
		assert_eq!(cpu.registers.a, 1);
		assert_eq!(cpu.registers.b, 15);
		assert_eq!(cpu.registers.f, 0b0000_0000, "No flags should be set");

		op(&mut cpu, &0o04);
		assert_eq!(cpu.registers.b, 16);
		assert_eq!(cpu.registers.f, 0b0010_0000, "The half carry flag should be set");

		cpu.registers.c = 255;
		op(&mut cpu, &0o14);
		assert_eq!(cpu.registers.c, 0);
		assert_eq!(cpu.registers.f, 0b01010_0000, "Zero and half carry flag should be set. Full carry is ignored");
	}

	#[test]
	fn test_inc_r16_and_dec_r16() {
		let mut cpu = CPU::new();
		cpu.ram[0] = 0o063;
		cpu.ram[1] = 0o073;
		let (inc_instruction, inc_operation) = cpu.get_operation();
		let (dec_instruction, dec_operation) = cpu.get_operation();
		assert!(fn_addr_eq(inc_operation, CPU::inc_r16 as InstructionHandler));
		assert!(fn_addr_eq(dec_operation, CPU::dec_r16 as InstructionHandler));

		assert_eq!(cpu.registers.get_r16(3), 0);
		inc_operation(&mut cpu, &inc_instruction);
		assert_eq!(cpu.registers.get_r16(3), 1);
		dec_operation(&mut cpu, &dec_instruction);
		assert_eq!(cpu.registers.get_r16(3), 0);
	}

	#[test]
	fn test_dec() {
		let mut cpu = CPU::new();
		cpu.ram[0] = 0o05;
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::dec_r8 as InstructionHandler));

		cpu.registers.a = 1;
		cpu.registers.b = 5;
		op(&mut cpu, &0o05);
		assert_eq!(cpu.registers.a, 1);
		assert_eq!(cpu.registers.c, 0);
		assert_eq!(cpu.registers.b, 5 - 1);
		assert_eq!(cpu.registers.f, 0b0100_0000, "Only the subtraction  flag should be set");

		op(&mut cpu, &0o75);
		assert_eq!(cpu.registers.a, 1 - 1);
		assert_eq!(cpu.registers.c, 0);
		assert_eq!(cpu.registers.b, 5 - 1);
		assert_eq!(cpu.registers.f, 0b1100_0000, "The subtraction and zero flags should be set");

		op(&mut cpu, &0o15);
		assert_eq!(cpu.registers.c, 255);
		assert_eq!(cpu.registers.f, 0b0110_0000, "All flags should be set");
	}

	#[test]
	fn test_ld_r8_r8() {
		let mut cpu = CPU::new();
		cpu.ram[0] = 0o100;

		let (instruction, operation) = cpu.get_operation();
		assert!(fn_addr_eq(operation, CPU::ld_r8_r8 as InstructionHandler));

		// Actually perform operation testing flags are not touched
		operation(&mut cpu, &instruction);
		cpu.registers.f = 0b1111_0000;
		assert_eq!(cpu.registers.b, 0);
		assert_eq!(cpu.registers.f, 0b1111_0000);

		// Load c into b
		cpu.registers.c = 10;
		operation(&mut cpu, &0o101);
		assert_eq!(cpu.registers.b, 10);
		assert_eq!(cpu.registers.c, 10);
		assert_eq!(cpu.registers.f, 0b1111_0000);

		// Load c into a
		operation(&mut cpu, &0o171);
		assert_eq!(cpu.registers.a, 10);
		assert_eq!(cpu.registers.b, 10);
		assert_eq!(cpu.registers.c, 10);
	}

	#[test]
	fn test_ld_r8_n8() {
		// Load b on to b
		let mut cpu = CPU::new();
		cpu.ram.test_load(0, vec![0o56, 10, 0o56, 0]);
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::ld_r8_n8 as InstructionHandler));

		// Actually perform operation. testing flags are not touched
		cpu.registers.f = 0b1111_0000;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.l, 10);
		assert_eq!(cpu.registers.f, 0b1111_0000);
		assert_eq!(cpu.registers.pc, 2);

		// Run another operation to load 0 back
		let (instruction, op) = cpu.get_operation();
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.l, 0);
	}

	#[test]
	fn test_ld_r16_r16() {
		let mut cpu = CPU::new();
		cpu.ram.test_load(0, vec![0o41, 0b0000_1000, 0b0000_1010, 0o41, 0, 0]);
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::ld_r16_n16 as InstructionHandler));

		// Actually perform operation. testing flags are not touched
		cpu.registers.f = 0b1111_0000;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.get_r16(2), 0b0000_1010_0000_1000);
		assert_eq!(cpu.registers.f, 0b1111_0000);


		// Load 0 back
		let (instruction, op) = cpu.get_operation();
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.get_r16(2), 0);
	}

	#[test]
	fn test_ld_hl_r8() {
		// Load b on to b
		let mut cpu = CPU::new();
		cpu.ram[0] = 0o163;
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::ld_hl_r8 as InstructionHandler));

		// Set location
		let location = 1029u16;
		cpu.registers.set_hl(location);
		assert_eq!(cpu.registers.get_hl(), location);

		// Perform op
		let value = 199u8;
		cpu.registers.e = value;
		cpu.registers.f = 0b1111_0000;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.ram[location as usize], value);
		assert_eq!(cpu.registers.f, 0b1111_0000);

		// Load 0 back
		cpu.registers.e = 0;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.ram[location as usize], 0);
	}

	#[test]
	fn test_ld_hl_n8() {
		// Load b on to b
		let mut cpu = CPU::new();
		let value = 29u8;
		cpu.ram.test_load(0, vec![0o066, value]);
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::ld_hl_n8 as InstructionHandler));

		// Set location
		let location = 9291u16;
		cpu.registers.set_hl(location);
		assert_eq!(cpu.registers.get_hl(), location);

		// Perform op
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.ram[location as usize], value);
	}

	#[test]
	fn test_carry_flag_ops() {
		let mut cpu = CPU::new();
		cpu.ram.test_load(0, vec![0o067, 0o067, 0o077, 0o077]);

		cpu.registers.f = 0b1110_0000;
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::scf as InstructionHandler));
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.f, 0b1001_0000, "Carry flag should be set");

		let (instruction, op) = cpu.get_operation();
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.f, 0b1001_0000, "Carry flag should still be set");

		cpu.registers.f = 0b1111_0000;
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::ccf as InstructionHandler));
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.f, 0b1000_0000, "Carry flag should be complemented (unset)");

		let (instruction, op) = cpu.get_operation();
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.f, 0b1001_0000, "Carry flag should be set again after complement");
	}

	#[test]
	fn test_cp_a_r8() {
		// Compare A with D
		let mut cpu = CPU::new();
		cpu.ram[0] = 0o272;
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::cp_a_r8 as InstructionHandler));

		// Actually run the above instruction
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.a, 0);
		assert_eq!(cpu.registers.d, 0);
		assert_eq!(cpu.registers.f, 0b1100_0000, "Zero and subtract flag should be set");

		// A > D
		cpu.registers.a = 10;
		cpu.registers.d = 5;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.a, 10);
		assert_eq!(cpu.registers.d, 5);
		assert_eq!(cpu.registers.f, 0b0100_0000, "Subtract flag should be set");

		// A == D
		cpu.registers.d = 10;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.a, 10);
		assert_eq!(cpu.registers.d, 10);
		assert_eq!(cpu.registers.f, 0b1100_0000, "Zero and subtract flag should be set");

		// Lower nibble borrow
		cpu.registers.a = 0b0001_0000;
		cpu.registers.d = 0b0000_1000;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.a, 16);
		assert_eq!(cpu.registers.d, 8);
		assert_eq!(cpu.registers.f, 0b0110_0000, "Zero and subtract flag should be set");

		// Underflow with nibble borrow
		cpu.registers.a = 0b0001_0000;
		cpu.registers.d = 0b0001_1000;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.a, 16);
		assert_eq!(cpu.registers.d, 24);
		assert_eq!(cpu.registers.f, 0b0111_0000, "Subtract and both carry flags should be set");
	}

	#[test]
	fn test_rl() {
		// RL [HL]
		let mut cpu = CPU::new();
		cpu.ram.test_load(0, vec![0o313, 0o026]);
		let (_, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::rl_hl as InstructionHandler));

		// RLA
		let mut cpu = CPU::new();
		cpu.ram[0] = 0o027;
		let (_, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::rl_a as InstructionHandler));

		// RLC [HL]
		let mut cpu = CPU::new();
		cpu.ram.test_load(0, vec![0o313, 0o006]);
		let (_, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::rlc_hl as InstructionHandler));

		// RLC A
		let mut cpu = CPU::new();
		cpu.ram[0] = 0o007;
		let (_, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::rlc_a as InstructionHandler));
	}

	#[test]
	fn test_rl_r8() {
		let mut cpu = CPU::new();
		cpu.ram.test_load(0, vec![0o313, 0o022]);
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::rl_r8 as InstructionHandler));

		cpu.registers.d = 0b0100_0100;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.d, 0b1000_1000);
		assert!(!cpu.registers.get_flag(Flag::Carry));

		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.d, 0b0001_0000);
		assert!(cpu.registers.get_flag(Flag::Carry));

		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.d, 0b0010_0001);
		assert!(!cpu.registers.get_flag(Flag::Carry));
	}

	#[test]
	fn test_rlc_r8() {
		let mut cpu = CPU::new();
		cpu.ram.test_load(0, vec![0o313, 0o004]);
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::rlc_r8 as InstructionHandler));

		cpu.registers.h = 0b0110_0110;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.h, 0b1100_1100);
		assert!(!cpu.registers.get_flag(Flag::Carry));

		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.h, 0b1001_1001);
		assert!(cpu.registers.get_flag(Flag::Carry));

		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.h, 0b0011_0011);
		assert!(cpu.registers.get_flag(Flag::Carry));
	}

	#[test]
	fn test_rr() {
		// RR [HL]
		let mut cpu = CPU::new();
		cpu.ram.test_load(0, vec![0o313, 0o036]);
		let (_, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::rr_hl as InstructionHandler));

		// RRA
		let mut cpu = CPU::new();
		cpu.ram[0] = 0o037;
		let (_, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::rr_a as InstructionHandler));

		// RRC [HL]
		let mut cpu = CPU::new();
		cpu.ram.test_load(0, vec![0o313, 0o016]);
		let (_, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::rrc_hl as InstructionHandler));

		// RRC A
		let mut cpu = CPU::new();
		cpu.ram[0] = 0o017;
		let (_, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::rrc_a as InstructionHandler));

		// SRA [HL]
		let mut cpu = CPU::new();
		cpu.ram.test_load(0, vec![0o313, 0o056]);
		let (_, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::sra_hl as InstructionHandler));

		// SRL [HL]
		let mut cpu = CPU::new();
		cpu.ram.test_load(0, vec![0o313, 0o076]);
		let (_, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::srl_hl as InstructionHandler));
	}

	#[test]
	fn test_rr_r8() {
		let mut cpu = CPU::new();
		cpu.ram.test_load(0, vec![0o313, 0o032]);
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::rr_r8 as InstructionHandler));

		cpu.registers.d = 0b0100_1010;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.d, 0b0010_0101);
		assert!(!cpu.registers.get_flag(Flag::Carry));

		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.d, 0b0001_0010);
		assert!(cpu.registers.get_flag(Flag::Carry));

		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.d, 0b1000_1001);
		assert!(!cpu.registers.get_flag(Flag::Carry));
	}

	#[test]
	fn test_rrc_r8() {
		let mut cpu = CPU::new();
		cpu.ram.test_load(0, vec![0o313, 0o014]);
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::rrc_r8 as InstructionHandler));

		cpu.registers.h = 0b0111_0110;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.h, 0b0011_1011);
		assert!(!cpu.registers.get_flag(Flag::Carry));

		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.h, 0b1001_1101);
		assert!(cpu.registers.get_flag(Flag::Carry));

		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.h, 0b1100_1110);
		assert!(cpu.registers.get_flag(Flag::Carry));
	}

	#[test]
	fn test_sla_r8() {
		let mut cpu = CPU::new();
		cpu.ram.test_load(0, vec![0o313, 0o040]);
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::sla_r8 as InstructionHandler));

		cpu.registers.b = 0b1011_0001;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.b, 0b0110_0010);
		assert!(cpu.registers.get_flag(Flag::Carry));

		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.b, 0b1100_0100);
		assert!(!cpu.registers.get_flag(Flag::Carry));

		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.b, 0b1000_1000);
		assert!(cpu.registers.get_flag(Flag::Carry));
	}

	#[test]
	fn test_sra_r8() {
		let mut cpu = CPU::new();
		cpu.ram.test_load(0, vec![0o313, 0o050]);
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::sra_r8 as InstructionHandler));

		cpu.registers.b = 0b0000_0000;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.b, 0b0000_0000);
		assert_eq!(cpu.registers.f, 0b1000_0000);

		cpu.registers.b = 0b1000_0110;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.b, 0b1100_0011);
		assert!(!cpu.registers.get_flag(Flag::Carry));

		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.b, 0b1110_0001);
		assert!(cpu.registers.get_flag(Flag::Carry));
	}

	#[test]
	fn test_swap() {
		let mut cpu = CPU::new();
		cpu.ram.test_load(0, vec![0o313, 0o061]);
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::swap_r8 as InstructionHandler));

		// Swap 0s
		cpu.registers.f = 0b1111_0000;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.c, 0b0000_0000);
		assert_eq!(cpu.registers.f, 0b1000_0000);

		// Swap
		cpu.registers.c = 0b0110_1010;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.c, 0b1010_0110);
		assert_eq!(cpu.registers.f, 0b0000_0000);

		// Swap back
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.c, 0b0110_1010);
		assert_eq!(cpu.registers.f, 0b0000_0000);
	}

	#[test]
	fn test_call_cc_n16() {
		let mut cpu = CPU::new();
		cpu.ram.test_load(0, vec![0o314, 0b1111_0110, 0, 0o314, 0b1111_0110, 0]);
		cpu.registers.set_sp(100);
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::call_cc_n16 as InstructionHandler));

		// Condition not met
		assert_eq!(cpu.registers.get_sp(), 100);
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.get_sp(), 100);
		assert_eq!(cpu.registers.pc, 3);

		// Condition met
		cpu.registers.set_flag(Flag::Zero, true);
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::call_cc_n16 as InstructionHandler));
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.get_sp(), 98);
		assert_eq!(cpu.registers.pc, 0b1111_0110);
		assert_eq!(cpu.ram[99], 0);
		assert_eq!(cpu.ram[98], 6);
	}

	#[test]
	fn test_ret() {
		let mut cpu = CPU::new();
		cpu.ram.test_load(0, vec![0o311, 0o325, 0o335]);
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::ret as InstructionHandler));

		// Push return address onto stack
		cpu.registers.set_sp(100);
		cpu.stack_push_16(0b0000_1111_1010_1010);

		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.get_sp(), 100);
		assert_eq!(cpu.registers.pc, 0b0000_1111_1010_1010);
	}

	#[test]
	fn test_push_r16() {
		let mut cpu = CPU::new();
		cpu.ram.test_load(0, vec![0o325, 0o365]);
		cpu.registers.set_sp(100);
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::push_r16 as InstructionHandler));

		cpu.registers.d = 0b1111_0000;
		cpu.registers.e = 0b1010_1010;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.get_sp(), 98);
		assert_eq!(cpu.ram[99], 0b1111_0000);
		assert_eq!(cpu.ram[98], 0b1010_1010);

		cpu.registers.a = 0b0000_1111;
		cpu.registers.f = 0b0101_1111;
		let (instruction, op) = cpu.get_operation();
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.get_sp(), 96);
		assert_eq!(cpu.ram[97], 0b0000_1111);
		assert_eq!(cpu.ram[96], 0b0101_0000);
	}

	#[test]
	fn test_pop_r16() {
		let mut cpu = CPU::new();
		cpu.ram.test_load(0, vec![0o301, 0o361]);
		cpu.registers.set_sp(100);
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::pop_r16 as InstructionHandler));

		cpu.stack_push_16(0b0000_1111_0101_0101);
		cpu.stack_push_16(0b1111_0000_1010_1010);

		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.b , 0b1111_0000);
		assert_eq!(cpu.registers.c , 0b1010_1010);
		assert_eq!(cpu.registers.f, 0b0000_0000);
		assert_eq!(cpu.registers.get_sp(), 98);

		let (instruction, op) = cpu.get_operation();
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.a , 0b0000_1111);
		assert_eq!(cpu.registers.f , 0b0101_0000);
		assert_eq!(cpu.registers.get_sp(), 100);
	}

	#[test]
	fn test_j() {
		// JP n16
		let mut cpu = CPU::new();
		cpu.ram[0] = 0o303;
		let (_, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::jp_n16 as InstructionHandler));

		// JP HL
		let mut cpu = CPU::new();
		cpu.ram[0] = 0o351;
		let (_, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::jp_hl as InstructionHandler));

		// JP cc n16
		let mut cpu = CPU::new();
		cpu.ram[0] = 0o302;
		let (_, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::jp_cc_n16 as InstructionHandler));

		// JR n16
		let mut cpu = CPU::new();
		cpu.ram[0] = 0o030;
		let (_, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::jr_n16 as InstructionHandler));
	}

	#[test]
	fn test_jr_cc_n16() {
		let mut cpu = CPU::new();
		cpu.registers.pc = 500;
		cpu.ram.test_load(500, vec![0o070, 0b1000_0000, 0o070, 0b1000_0000]);
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::jr_cc_n16 as InstructionHandler));

		// Condition not met
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.pc, 502);

		// Condition met and we jump backwards
		cpu.registers.set_flag(Flag::Carry, true);
		let (instruction, op) = cpu.get_operation();
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.pc, 504 - 128);

		// Test jump ahead
		cpu.ram.test_load(504 - 128, vec![0o070, 0b0100_0000]);
		let (instruction, op) = cpu.get_operation();
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.pc, (504 - 128 + 2) + 64);
	}

	#[test]
	fn test_bit_u3_r8() {
		let mut cpu = CPU::new();
		cpu.ram.test_load(0, vec![0o313, 0o121]);
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::bit_u3_r8 as InstructionHandler));

		// Run with bit set
		cpu.registers.c = 0b0000_0100;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.f, 0b0010_0000);

		// Run with bit unset
		cpu.registers.c = 0b1111_1011;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.f, 0b1010_0000);

		cpu.registers.c = 0b0000_0000;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.f, 0b1010_0000);
	}

	#[test]
	fn test_bit_u3_hl() {
		let mut cpu = CPU::new();
		cpu.ram.test_load(0, vec![0o313, 0o166]);
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::bit_u3_hl as InstructionHandler));

		// Run with bit set
		cpu.registers.set_hl(10);
		cpu.ram[10] = 0b0100_0000;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.f, 0b0010_0000);

		// Run with bit unset
		cpu.ram[10] = 0b1011_1111;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.f, 0b1010_0000);

		cpu.ram[10] = 0b0000_0000;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.f, 0b1010_0000);
	}

	#[test]
	fn test_res_u3_r8() {
		let mut cpu = CPU::new();
		cpu.ram.test_load(0, vec![0o313, 0o250]);
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::res_u3_r8 as InstructionHandler));

		// Run with bit set
		cpu.registers.b = 0b1111_1111;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.b, 0b1101_1111);

		// Run with bit unset
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.b, 0b1101_1111);
	}

	#[test]
	fn test_res_u3_hl() {
		let mut cpu = CPU::new();
		cpu.ram.test_load(0, vec![0o313, 0o226]);
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::res_u3_hl as InstructionHandler));

		// Run with bit set
		cpu.registers.set_hl(25);
		cpu.ram[25] = 0b0110_0101;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.ram[25], 0b0110_0001);

		// Run with bit unset
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.ram[25], 0b0110_0001);
	}

	#[test]
	fn test_set_u3_r8() {
		let mut cpu = CPU::new();
		cpu.ram.test_load(0, vec![0o313, 0o350]);
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::set_u3_r8 as InstructionHandler));

		// Run with bit set
		cpu.registers.b = 0b1101_1111;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.b, 0b1111_1111);

		// Run with bit unset
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.b, 0b1111_1111);
	}

	#[test]
	fn test_set_u3_hl() {
		let mut cpu = CPU::new();
		cpu.ram.test_load(0, vec![0o313, 0o326]);
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::set_u3_hl as InstructionHandler));

		// Run with bit set
		cpu.registers.set_hl(25);
		cpu.ram[25] = 0b0110_0001;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.ram[25], 0b0110_0101);

		// Run with bit unset
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.ram[25], 0b0110_0101);
	}

	#[test]
	#[should_panic]
	fn test_panics_for_unknown() {
		let mut cpu = CPU::new();
		cpu.ram[0] = 0o374;
		cpu.get_operation();
	}

	#[test]
	fn test_ei_di() {
		let mut cpu = CPU::new();
		cpu.ram.test_load(0, vec![0o373, 0, 0o373, 0o363]);
		let i_data = cpu.get_operation();
		assert!(fn_addr_eq(i_data.1, CPU::ei as InstructionHandler));
		cpu.run_operation(i_data);

		// IME not set yet
		cpu.run_operation(i_data);
		assert_eq!(cpu.ime, Ime::ToSet);

		// After an instruction is run
		let i_data = cpu.get_operation();
		cpu.run_operation(i_data);
		assert_eq!(cpu.ime, Ime::Set);

		// Reset IME and retry
		cpu.ime = Ime::Off;
		let i_data = cpu.get_operation();
		cpu.run_operation(i_data);
		assert_eq!(cpu.ime, Ime::ToSet);

		// If DI is the next instruction
		let i_data = cpu.get_operation();
		cpu.run_operation(i_data);
		assert_eq!(cpu.ime, Ime::Off);
	}

	#[test]
	fn test_halt() {
		let mut cpu = CPU::new();
		cpu.ram.test_load(0 , vec![0o166, 0o166, 0o306, 99]);
		let (instruction, op) = cpu.get_operation();

		// IME not set and no interrupt pending
		assert!(fn_addr_eq(op, CPU::halt as InstructionHandler));
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.mode, LowPower);

		// IME not set and interrupt pending, we get the halt bug
		cpu.mode = NormalSpeed;
		cpu.ram[0xFFFF] = 0b1111_1111;
		cpu.ram[0xFF0F] = 0b1111_1111;
		let i_data = cpu.get_operation();
		cpu.run_operation(i_data);
		assert_eq!(cpu.mode, NormalSpeed);
		assert!(cpu.halt_bug_active);


		// Now when we run our add instruction, we expect to add 99 to 'A' but instead we add 0o306
		let i_data = cpu.get_operation();
		assert!(fn_addr_eq(i_data.1, CPU::add_a_n8 as InstructionHandler));
		cpu.run_operation(i_data);
		assert_eq!(cpu.registers.a, 0o306);
		assert_eq!(cpu.registers.pc, 3, "The program counter should be pointing to our n8 value now");
		assert_eq!(cpu.ram[cpu.registers.pc as usize], 99);
	}

	#[test]
	fn test_stop() {
		let mut cpu = CPU::new();
		cpu.ram[0] = 0o020;
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::stop as InstructionHandler));

		cpu.run_operation((instruction, op));
		assert_eq!(cpu.mode, VeryLowPower);
	}

	#[test]
	fn test_daa() {
		let mut cpu = CPU::new();
		cpu.ram[0] = 0o047;
		let (instruction, op) = cpu.get_operation();
		assert!(fn_addr_eq(op, CPU::daa as InstructionHandler));

		// A is 0 and no flags are set
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.f, 0b1000_0000);

		// Subtraction flag NOT set
		cpu.registers.a = 0;
		cpu.registers.f = 0b1010_0000;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.f, 0b0000_0000);
		assert_eq!(cpu.registers.a, 0b0000_0110, "0x6 should have been added due to the half carry");

		cpu.registers.a = 0;
		cpu.registers.f = 0b1001_0000;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.f, 0b0001_0000);
		assert_eq!(cpu.registers.a, 0b0110_0000, "0x60 should have been added due to the half carry");

		cpu.registers.a = 0b0000_1010;
		cpu.registers.f = 0b0000_0000;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.f, 0b0000_0000);
		assert_eq!(cpu.registers.a, 0b0001_0000, "0x6 should have been added as A & 0xF > $9");

		// Subtraction flag SET
		// Zero
		cpu.registers.a = 0;
		cpu.registers.f = 0b1100_0000;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.f, 0b1100_0000);
		assert_eq!(cpu.registers.a, 0);

		// If subtract flag set, we don't subtract anything even if A is greater than some numbers
		cpu.registers.a = 0b0111_1111;
		cpu.registers.f = 0b1100_0000;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.f, 0b0100_0000);
		assert_eq!(cpu.registers.a, 0b0111_1111);

		// Half carry
		cpu.registers.a = 7;
		cpu.registers.f = 0b1110_0000;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.f, 0b0100_0000);
		assert_eq!(cpu.registers.a, 1, "0x6 should have been subtracted");

		// Carry removes carry flag
		cpu.registers.a = 0x61;
		cpu.registers.f = 0b1101_0000;
		cpu.run_operation((instruction, op));
		assert_eq!(cpu.registers.f, 0b0101_0000, "Carry flag should be kept if subtract flag is set");
		assert_eq!(cpu.registers.a, 1, "0x60 should have been subtracted");
	}

	#[test]
	fn test_instruction() {
		let instruction = 0b0000_0000_1000_1111 as Instruction;
		assert_eq!(instruction.first_u3(), 0b010);
		assert_eq!(instruction.middle_u3(), 0b001);
		assert_eq!(instruction.last_u3(), 0b111);
		assert_eq!(instruction.interleaved_r16(true), 0);
		assert_eq!(instruction.interleaved_r16(false), 0);

		let instruction = 0b0000_0000_1001_1111 as Instruction;
		assert_eq!(instruction.interleaved_r16(false), 1);
	}
}
