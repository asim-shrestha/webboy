use webboy::ram::{Ram, RamOperations};
use webboy::cpu::CPU;
use std::fs::read;
use std::env;

fn main() {
    let args = env::args().collect::<Vec<String>>();
    let file_name = if args.len() > 1 {
        &args[1]
    } else {
        println!("Usage: webboy <ROM file>");
        return;
    };

    let rom: Vec<u8> = load_rom(file_name);
    let mut cpu = CPU::new();
    cpu.ram[0xFF44] = 0x90; // Set LY to simulate some VBlank progress
    cpu.ram.load_rom(&rom);

    cpu.boot();

    let max_log_test_length = 7427500;
    for _ in 0..=max_log_test_length {
        cpu.execute(true);
    }
}

fn load_rom(file_name: &str) -> Vec<u8> {
    match read(file_name) {
        Ok(data) => data,
        Err(e) => {
            panic!("Failed to read ROM file '{}': {}", file_name, e);
        }
    }
}