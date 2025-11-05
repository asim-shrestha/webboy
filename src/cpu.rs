use crate::cpu::register::Registers;
use crate::ram::Ram;

pub mod instruction;
pub mod register;
mod carry;

#[derive(PartialEq, Eq, Debug)]
enum Ime {
	Off,
	ToSet,
	Set,
}

#[derive(PartialEq, Eq, Debug)]
enum Mode {
	VeryLowPower,
	LowPower,
	NormalSpeed,
	DoubleSpeed,
}

pub struct CPU {
	pub registers: Registers,
	ram: Ram,
	ime: Ime,
	cycle_count: u64,
	mode: Mode,
	halt_bug_active: bool,
}