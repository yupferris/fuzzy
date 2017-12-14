extern crate serialport;
extern crate rustual_boy_core;
extern crate rustual_boy_middleware;
extern crate rand;
extern crate time;
extern crate byteorder;
extern crate minifb;

mod command;
mod crapsum;
mod emu;
mod teensy_vb;
mod transport;

use rand::{Rng, StdRng, SeedableRng};

use byteorder::{LittleEndian, WriteBytesExt};

use emu::*;

use std::fmt::Debug;
use std::io::{stdout, Read, Write};

fn main() {
    let mut hw_port = teensy_vb::connect("COM4").expect("Couldn't connect to teensy");
    let mut emu_port = EmulatedVbSerialPort::new();

    macro_rules! test {
        ($name:ident) => ((Box::new($name), stringify!($name)));
    }

    let tests: Vec<(Box<Fn(&mut _, &mut _, usize) -> Result<(), String>>, &'static str)> = vec![
        test!(single_ret),
        test!(muls),
    ];

    let num_tests = tests.len();
    let mut passed_tests = 0;
    let mut failed_tests = 0;

    for (index, (test_fn, test_name)) in tests.into_iter().enumerate() {
        print!("({}) running test `{}` ... ", index, test_name);
        stdout().flush().unwrap();
        match test_fn(&mut hw_port, &mut emu_port, index) {
            Ok(()) => {
                println!("ok");
                passed_tests += 1;
            }
            Err(e) => {
                println!("ERROR: {}", e);
                failed_tests += 1;
            }
        }
    }

    println!("");
    println!("Ran {} tests, {} passed, {} failed", num_tests, passed_tests, failed_tests);
}

trait Generator {
    fn next(&mut self, buf: &mut Vec<u8>);
}

struct Ret;

impl Generator for Ret {
    fn next(&mut self, buf: &mut Vec<u8>) {
        let op = 0b000110;
        let reg1 = 31;
        buf.write_u16::<LittleEndian>((op << 10) | reg1).unwrap();
    }
}

fn single_ret<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, test_index: usize) -> Result<(), String> {
    let mut rom = Vec::new();
    Ret.next(&mut rom);
    
    let seed: &[_] = &[test_index];
    let mut rng = SeedableRng::from_seed(seed);
    let initial_regs = random_regs(&mut rng);

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

struct Mul {
    rng: StdRng,
}

impl Mul {
    fn new(rng: StdRng) -> Mul {
        Mul {
            rng: rng,
        }
    }
}

impl Generator for Mul {
    fn next(&mut self, buf: &mut Vec<u8>) {
        let op = 0b001000;
        let reg1 = self.rng.gen::<u32>() % 32;
        let reg2 = self.rng.gen::<u32>() % 31; // Don't include r31
        buf.write_u16::<LittleEndian>((op << 10) | ((reg2 as u16) << 5) | (reg1 as u16)).unwrap();
    }
}

fn muls<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, test_index: usize) -> Result<(), String> {
    let mut rom = Vec::new();

    let seed: &[_] = &[test_index];
    let mut rng: StdRng = SeedableRng::from_seed(seed);
    let mut mul = Mul::new(rng);

    for _ in 0..100 {
        mul.next(&mut rom);
    }

    Ret.next(&mut rom);

    /*{
        use std::fs::File;
        let mut file = File::create("derp.vxe").unwrap();
        file.write_all(&rom).unwrap();
    }*/
    
    let seed: &[_] = &[rng.gen::<usize>()];
    let mut rng = SeedableRng::from_seed(seed);
    let initial_regs = random_regs(&mut rng);

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

fn random_regs(rng: &mut StdRng) -> Vec<u32> {
    // Initial regs cover r0-r29 inclusive
    (0..30).map(|_| rng.gen::<u32>()).collect::<Vec<_>>()
}

fn test_rom<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, rom: &[u8], initial_regs: &[u32]) -> Result<(), String> {
    let rom_addr = 0x05000000 + 0x0400;

    let hw_result_regs = test_rom_on_port(hw_port, rom, rom_addr, initial_regs).map_err(|e| format!("Hardware dispatch failed: {:?}", e))?;
    let emu_result_regs = test_rom_on_port(emu_port, rom, rom_addr, initial_regs).map_err(|e| format!("Emu dispatch failed: {:?}", e))?;

    //println!("regs: {:#?},", emu_result_regs);

    assert_eq(hw_result_regs, emu_result_regs)
}

fn assert_eq<T: Debug + Eq>(a: T, b: T) -> Result<(), String> {
    if a == b {
        Ok(())
    } else {
        Err(format!("Assert equality failed\na: {:?}\nb: {:?}", a, b))
    }
}

fn test_rom_on_port<P: Read + Write>(port: &mut P, rom: &[u8], rom_addr: u32, initial_regs: &[u32]) -> Result<Vec<u32>, command::Error> {
    command::write_mem_region(port, rom_addr, &rom)?;

    let initial_regs_addr = 0x0001e000;

    let initial_regs_bytes = initial_regs.iter().flat_map(|x| {
        let mut bytes = Vec::new();
        bytes.write_u32::<LittleEndian>(*x).unwrap();
        bytes
    }).collect::<Vec<_>>();

    command::write_mem_region(port, initial_regs_addr, &initial_regs_bytes)?;

    let exec_entry = rom_addr;

    command::execute(port, exec_entry)?;

    let mut tries = 0;

    loop {
        let result_regs_addr = initial_regs_addr + 32 * 4;

        match command::read_mem_region(port, result_regs_addr, 32 * 4) {
            Ok(result_regs_bytes) => {
                let mut result_regs = Vec::new();

                for i in 0..32 {
                    let mut reg = 0;
                    for j in 0..4 {
                        reg >>= 8;
                        reg |= (result_regs_bytes[(i * 4 + j) as usize] as u32) << 24;
                    }
                    result_regs.push(reg);
                }

                return Ok(result_regs);
            }
            Err(e) => {
                tries += 1;
                if tries >= 200 {
                    return Err(e);
                }
            }
        }
    }
}
