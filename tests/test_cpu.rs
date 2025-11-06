use serde_json::{Result, Value};
use std::fs;
use std::path::PathBuf;
use webboy::cpu::CPU;
use webboy::cpu::register::Registers;
use webboy::ram::{Ram, RamOperations};

fn read_file(file_name: &str) -> Result<Value> {
	let data: String = fs::read_to_string(file_name).unwrap();
	serde_json::from_str(&data)
}

#[test]
fn test_all_jsons() {
	let paths = fs::read_dir("./tests/cpu_tests").unwrap();
	let mut test_ran: bool = false;

	for result in paths {
		test_ran = true;
		let path = result.unwrap().path();
		println!("Running test file: {:?}", path);
		test_json(path);
	}

	if !test_ran {
		panic!("No CPU tests found. You'll need to manually download the JSON specs from https://github.com/adtennant/GameboyCPUTests/tree/master and place them in the cpu_test folder. (I'm too lazy to write a script for this- sorry!)")
	}
}

fn test_json(path: PathBuf) {
	let test_json = read_file(path.to_str().unwrap()).unwrap();
	let tests_array = test_json.as_array().unwrap();

	for (i, test) in tests_array.iter().enumerate() {
		println!("Running test {}, with name {:?}", i, test["name"].as_str().unwrap());
		run_test(test);
	}
}

fn run_test(test: &Value) {
	// Initialize
	let initial_values = &test["initial"];
	let mut cpu = CPU::new();
	set_ram(&mut cpu.ram, initial_values);

	set_registers(&mut cpu.registers, initial_values);

	// Run operation
	println!("Running instruction 0o{:o}", cpu.ram[cpu.registers.pc as usize]);
	cpu.execute(true);
	cpu.print_cpu();

	// Validate
	assert_expected(&cpu, &test);
}

fn set_registers(registers: &mut Registers, initial_values: &Value) {
	registers.a = initial_values["a"].as_u64().unwrap() as u8;
	registers.b = initial_values["b"].as_u64().unwrap() as u8;
	registers.c = initial_values["c"].as_u64().unwrap() as u8;
	registers.d = initial_values["d"].as_u64().unwrap() as u8;
	registers.e = initial_values["e"].as_u64().unwrap() as u8;
	registers.f = initial_values["f"].as_u64().unwrap() as u8;
	registers.h = initial_values["h"].as_u64().unwrap() as u8;
	registers.l = initial_values["l"].as_u64().unwrap() as u8;
	registers.pc = (initial_values["pc"].as_u64().unwrap() - 1) as u16;
	registers.set_sp(initial_values["sp"].as_u64().unwrap() as u16);
}

fn set_ram(ram: &mut Ram, initial_values: &Value) {
	for ram_array in initial_values["ram"].as_array().unwrap() {
		let array_values = ram_array.as_array().unwrap();
		ram[array_values[0].as_u64().unwrap() as usize] = array_values[1].as_u64().unwrap() as u8;
	}
}

fn assert_expected(cpu: &CPU, test: &Value) {
	let final_values = &test["final"];

	// Test registers
	assert_eq!(cpu.registers.a, final_values["a"].as_u64().unwrap() as u8, "Register A mismatch");
	assert_eq!(cpu.registers.b, final_values["b"].as_u64().unwrap() as u8, "Register B mismatch");
	assert_eq!(cpu.registers.c, final_values["c"].as_u64().unwrap() as u8, "Register C mismatch");
	assert_eq!(cpu.registers.d, final_values["d"].as_u64().unwrap() as u8, "Register D mismatch");
	assert_eq!(cpu.registers.e, final_values["e"].as_u64().unwrap() as u8, "Register E mismatch");
	assert_eq!(cpu.registers.f, final_values["f"].as_u64().unwrap() as u8, "Register F mismatch");
	assert_eq!(cpu.registers.h, final_values["h"].as_u64().unwrap() as u8, "Register H mismatch");
	assert_eq!(cpu.registers.l, final_values["l"].as_u64().unwrap() as u8, "Register L mismatch");
	assert_eq!(cpu.registers.pc, (final_values["pc"].as_u64().unwrap() - 1) as u16, "PC mismatch");
	assert_eq!(cpu.registers.get_sp(), final_values["sp"].as_u64().unwrap() as u16, "SP mismatch");

	// Test ram
	let final_ram = &final_values["ram"];
	for ram_array in final_ram.as_array().unwrap() {
		let array_values = ram_array.as_array().unwrap();
		assert_eq!(
			cpu.ram[array_values[0].as_u64().unwrap() as usize],
			array_values[1].as_u64().unwrap() as u8,
			"RAM value mismatch at address 0x{:04X}", array_values[0].as_u64().unwrap() as usize
		);
	}

	// Test cycles
	// TODO: Add cycle tests
	let expected_cycle_count = test["cycles"].as_array().unwrap().len();
	assert_eq!(cpu.timer.cycles as usize, expected_cycle_count, "Cycle count mismatch");
}