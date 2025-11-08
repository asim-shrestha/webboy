mod renderer;

use crate::renderer::window_conf;
use std::env;
use std::fs::read;
use std::sync::mpsc::{self, Sender};
use std::thread;
use webboy::device::{Device, ImageData};

#[macroquad::main(window_conf)]
async fn main() {
    let args = env::args().collect::<Vec<String>>();
    let file_name = if args.len() > 1 {
        &args[1]
    } else {
        println!("Usage: webboy <ROM file>");
        return;
    };

    let rom: Vec<u8> = load_rom(file_name);
    let (tx, rx) = mpsc::channel::<ImageData>();
    thread::spawn(move || {
        webboy(rom, tx);
    });

    loop {
        renderer::handle(&rx).await;
    }
}

fn webboy(rom: Vec<u8>, tx: Sender<ImageData>) {
    let mut device = Device::new(tx);
    device.load(&rom);

    let max_log_test_length = 7427500;
    for _ in 0..=max_log_test_length {
        device.tick();
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
