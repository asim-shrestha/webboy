use crate::cpu::register::Registers;
use crate::ram::Ram;
use crate::timer::Timer;

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
	pub ram: Ram,
	pub timer: Timer,
	ime: Ime,
	mode: Mode,
	halt_bug_active: bool,
}