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
        test!(stsr_psws),
        test!(multi1),
        test!(moveas),
        test!(multi2),
        test!(movhis),
        test!(multi3),
        test!(mov_regs),
        test!(mov_imms),
        test!(mulus),
        test!(nots),
        test!(ors),
        test!(oris),
        test!(sar_regs),
        test!(sar_imms),
        test!(setfs),
        test!(shl_regs),
        test!(shl_imms),
        test!(shr_regs),
        test!(shr_imms),
        test!(subs),
        test!(xors),
        test!(xoris),
        test!(add_regs),
        test!(add_imms),
        test!(addis),
        test!(ands),
        test!(andis),
        test!(cmp_regs),
        test!(cmp_imms),
        test!(mpyhws),
        test!(revs),
        test!(xbs),
        test!(xhs),
        test!(multi_all),
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

struct MultiGenerator {
    generators: Vec<Box<Generator>>,
    rng: StdRng,
}

impl MultiGenerator {
    fn new(generators: Vec<Box<Generator>>, rng: StdRng) -> MultiGenerator {
        MultiGenerator {
            generators: generators,
            rng: rng,
        }
    }
}

impl Generator for MultiGenerator {
    fn next(&mut self, buf: &mut Vec<u8>) {
        let index = self.rng.gen::<usize>() % self.generators.len();
        self.generators[index].next(buf);
    }
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
    
    let initial_regs = random_regs(&mut build_rng(test_index));

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
    let mut rng = build_rng(test_index);

    let mut rom = Vec::new();

    let mut mul = Mul::new(build_rng(rng.gen::<usize>()));

    // We use a particularly low number here, as otherwise values from r0 will propagate to all other regs eventually.
    //  Need to make sure we test mul more thoroughly among other instr's as well in other tests.
    for _ in 0..100 {
        mul.next(&mut rom);
    }

    Ret.next(&mut rom);
    
    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

struct StsrPsw {
    rng: StdRng,
}

impl StsrPsw {
    fn new(rng: StdRng) -> StsrPsw {
        StsrPsw {
            rng: rng,
        }
    }
}

impl Generator for StsrPsw {
    fn next(&mut self, buf: &mut Vec<u8>) {
        let op = 0b011101;
        let reg2 = self.rng.gen::<u32>() % 31; // Don't include r31
        let imm5 = 5;
        buf.write_u16::<LittleEndian>((op << 10) | ((reg2 as u16) << 5) | imm5).unwrap();
    }
}

fn stsr_psws<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, test_index: usize) -> Result<(), String> {
    let mut rng = build_rng(test_index);

    let mut rom = Vec::new();

    let mut stsr_psw = StsrPsw::new(build_rng(rng.gen::<usize>()));

    for _ in 0..1000 {
        stsr_psw.next(&mut rom);
    }

    Ret.next(&mut rom);
    
    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

struct Movea {
    rng: StdRng,
}

impl Movea {
    fn new(rng: StdRng) -> Movea {
        Movea {
            rng: rng,
        }
    }
}

impl Generator for Movea {
    fn next(&mut self, buf: &mut Vec<u8>) {
        let op = 0b101000;
        let reg1 = self.rng.gen::<u32>() % 32;
        let reg2 = self.rng.gen::<u32>() % 31; // Don't include r31
        let imm16 = self.rng.gen::<i16>();
        buf.write_u16::<LittleEndian>((op << 10) | ((reg2 as u16) << 5) | (reg1 as u16)).unwrap();
        buf.write_u16::<LittleEndian>(imm16 as u16).unwrap();
    }
}

fn moveas<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, test_index: usize) -> Result<(), String> {
    let mut rng = build_rng(test_index);

    let mut rom = Vec::new();

    let mut movea = Movea::new(build_rng(rng.gen::<usize>()));

    for _ in 0..1000 {
        movea.next(&mut rom);
    }

    Ret.next(&mut rom);
    
    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

struct Movhi {
    rng: StdRng,
}

impl Movhi {
    fn new(rng: StdRng) -> Movhi {
        Movhi {
            rng: rng,
        }
    }
}

impl Generator for Movhi {
    fn next(&mut self, buf: &mut Vec<u8>) {
        let op = 0b101111;
        let reg1 = self.rng.gen::<u32>() % 32;
        let reg2 = self.rng.gen::<u32>() % 31; // Don't include r31
        let imm16 = self.rng.gen::<i16>();
        buf.write_u16::<LittleEndian>((op << 10) | ((reg2 as u16) << 5) | (reg1 as u16)).unwrap();
        buf.write_u16::<LittleEndian>(imm16 as u16).unwrap();
    }
}

fn movhis<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, test_index: usize) -> Result<(), String> {
    let mut rng = build_rng(test_index);

    let mut rom = Vec::new();

    let mut movhi = Movhi::new(build_rng(rng.gen::<usize>()));

    for _ in 0..1000 {
        movhi.next(&mut rom);
    }

    Ret.next(&mut rom);
    
    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

struct MovReg {
    rng: StdRng,
}

impl MovReg {
    fn new(rng: StdRng) -> MovReg {
        MovReg {
            rng: rng,
        }
    }
}

impl Generator for MovReg {
    fn next(&mut self, buf: &mut Vec<u8>) {
        let op = 0b000000;
        let reg1 = self.rng.gen::<u32>() % 32;
        let reg2 = self.rng.gen::<u32>() % 31; // Don't include r31
        buf.write_u16::<LittleEndian>((op << 10) | ((reg2 as u16) << 5) | (reg1 as u16)).unwrap();
    }
}

fn mov_regs<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, test_index: usize) -> Result<(), String> {
    let mut rng = build_rng(test_index);

    let mut rom = Vec::new();

    let mut mov_reg = MovReg::new(build_rng(rng.gen::<usize>()));

    for _ in 0..1000 {
        mov_reg.next(&mut rom);
    }

    Ret.next(&mut rom);
    
    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

struct MovImm {
    rng: StdRng,
}

impl MovImm {
    fn new(rng: StdRng) -> MovImm {
        MovImm {
            rng: rng,
        }
    }
}

impl Generator for MovImm {
    fn next(&mut self, buf: &mut Vec<u8>) {
        let op = 0b010000;
        let imm5 = self.rng.gen::<u32>() % 32;
        let reg2 = self.rng.gen::<u32>() % 31; // Don't include r31
        buf.write_u16::<LittleEndian>((op << 10) | ((reg2 as u16) << 5) | (imm5 as u16)).unwrap();
    }
}

fn mov_imms<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, test_index: usize) -> Result<(), String> {
    let mut rng = build_rng(test_index);

    let mut rom = Vec::new();

    let mut mov_imm = MovImm::new(build_rng(rng.gen::<usize>()));

    for _ in 0..1000 {
        mov_imm.next(&mut rom);
    }

    Ret.next(&mut rom);
    
    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

struct Mulu {
    rng: StdRng,
}

impl Mulu {
    fn new(rng: StdRng) -> Mulu {
        Mulu {
            rng: rng,
        }
    }
}

impl Generator for Mulu {
    fn next(&mut self, buf: &mut Vec<u8>) {
        let op = 0b001010;
        let reg1 = self.rng.gen::<u32>() % 32;
        let reg2 = self.rng.gen::<u32>() % 31; // Don't include r31
        buf.write_u16::<LittleEndian>((op << 10) | ((reg2 as u16) << 5) | (reg1 as u16)).unwrap();
    }
}

fn mulus<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, test_index: usize) -> Result<(), String> {
    let mut rng = build_rng(test_index);

    let mut rom = Vec::new();

    let mut mulu = Mulu::new(build_rng(rng.gen::<usize>()));

    // We use a particularly low number here, as otherwise values from r0 will propagate to all other regs eventually.
    //  Need to make sure we test mulu more thoroughly among other instr's as well in other tests.
    for _ in 0..100 {
        mulu.next(&mut rom);
    }

    Ret.next(&mut rom);
    
    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

struct Not {
    rng: StdRng,
}

impl Not {
    fn new(rng: StdRng) -> Not {
        Not {
            rng: rng,
        }
    }
}

impl Generator for Not {
    fn next(&mut self, buf: &mut Vec<u8>) {
        let op = 0b001111;
        let reg1 = self.rng.gen::<u32>() % 32;
        let reg2 = self.rng.gen::<u32>() % 31; // Don't include r31
        buf.write_u16::<LittleEndian>((op << 10) | ((reg2 as u16) << 5) | (reg1 as u16)).unwrap();
    }
}

fn nots<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, test_index: usize) -> Result<(), String> {
    let mut rng = build_rng(test_index);

    let mut rom = Vec::new();

    let mut not = Not::new(build_rng(rng.gen::<usize>()));

    for _ in 0..1000 {
        not.next(&mut rom);
    }

    Ret.next(&mut rom);
    
    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

struct Or {
    rng: StdRng,
}

impl Or {
    fn new(rng: StdRng) -> Or {
        Or {
            rng: rng,
        }
    }
}

impl Generator for Or {
    fn next(&mut self, buf: &mut Vec<u8>) {
        let op = 0b001100;
        let reg1 = self.rng.gen::<u32>() % 32;
        let reg2 = self.rng.gen::<u32>() % 31; // Don't include r31
        buf.write_u16::<LittleEndian>((op << 10) | ((reg2 as u16) << 5) | (reg1 as u16)).unwrap();
    }
}

fn ors<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, test_index: usize) -> Result<(), String> {
    let mut rng = build_rng(test_index);

    let mut rom = Vec::new();

    let mut or = Or::new(build_rng(rng.gen::<usize>()));

    for _ in 0..1000 {
        or.next(&mut rom);
    }

    Ret.next(&mut rom);
    
    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

struct Ori {
    rng: StdRng,
}

impl Ori {
    fn new(rng: StdRng) -> Ori {
        Ori {
            rng: rng,
        }
    }
}

impl Generator for Ori {
    fn next(&mut self, buf: &mut Vec<u8>) {
        let op = 0b101100;
        let reg1 = self.rng.gen::<u32>() % 32;
        let reg2 = self.rng.gen::<u32>() % 31; // Don't include r31
        let imm16 = self.rng.gen::<u16>();
        buf.write_u16::<LittleEndian>((op << 10) | ((reg2 as u16) << 5) | (reg1 as u16)).unwrap();
        buf.write_u16::<LittleEndian>(imm16).unwrap();
    }
}

fn oris<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, test_index: usize) -> Result<(), String> {
    let mut rng = build_rng(test_index);

    let mut rom = Vec::new();

    let mut ori = Ori::new(build_rng(rng.gen::<usize>()));

    for _ in 0..1000 {
        ori.next(&mut rom);
    }

    Ret.next(&mut rom);
    
    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

struct SarReg {
    rng: StdRng,
}

impl SarReg {
    fn new(rng: StdRng) -> SarReg {
        SarReg {
            rng: rng,
        }
    }
}

impl Generator for SarReg {
    fn next(&mut self, buf: &mut Vec<u8>) {
        let op = 0b000111;
        let reg1 = self.rng.gen::<u32>() % 32;
        let reg2 = self.rng.gen::<u32>() % 31; // Don't include r31
        buf.write_u16::<LittleEndian>((op << 10) | ((reg2 as u16) << 5) | (reg1 as u16)).unwrap();
    }
}

fn sar_regs<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, test_index: usize) -> Result<(), String> {
    let mut rng = build_rng(test_index);

    let mut rom = Vec::new();

    let mut sar_reg = SarReg::new(build_rng(rng.gen::<usize>()));

    for _ in 0..1000 {
        sar_reg.next(&mut rom);
    }

    Ret.next(&mut rom);
    
    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

struct SarImm {
    rng: StdRng,
}

impl SarImm {
    fn new(rng: StdRng) -> SarImm {
        SarImm {
            rng: rng,
        }
    }
}

impl Generator for SarImm {
    fn next(&mut self, buf: &mut Vec<u8>) {
        let op = 0b010111;
        let imm5 = self.rng.gen::<u32>() % 32;
        let reg2 = self.rng.gen::<u32>() % 31; // Don't include r31
        buf.write_u16::<LittleEndian>((op << 10) | ((reg2 as u16) << 5) | (imm5 as u16)).unwrap();
    }
}

fn sar_imms<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, test_index: usize) -> Result<(), String> {
    let mut rng = build_rng(test_index);

    let mut rom = Vec::new();

    let mut sar_imm = SarImm::new(build_rng(rng.gen::<usize>()));

    for _ in 0..1000 {
        sar_imm.next(&mut rom);
    }

    Ret.next(&mut rom);
    
    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

struct Setf {
    rng: StdRng,
}

impl Setf {
    fn new(rng: StdRng) -> Setf {
        Setf {
            rng: rng,
        }
    }
}

impl Generator for Setf {
    fn next(&mut self, buf: &mut Vec<u8>) {
        let op = 0b010010;
        let imm5 = self.rng.gen::<u32>() % 32;
        let reg2 = self.rng.gen::<u32>() % 31; // Don't include r31
        buf.write_u16::<LittleEndian>((op << 10) | ((reg2 as u16) << 5) | (imm5 as u16)).unwrap();
    }
}

fn setfs<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, test_index: usize) -> Result<(), String> {
    let mut rng = build_rng(test_index);

    let mut rom = Vec::new();

    let mut setf = Setf::new(build_rng(rng.gen::<usize>()));

    for _ in 0..1000 {
        setf.next(&mut rom);
    }

    Ret.next(&mut rom);
    
    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

struct ShlReg {
    rng: StdRng,
}

impl ShlReg {
    fn new(rng: StdRng) -> ShlReg {
        ShlReg {
            rng: rng,
        }
    }
}

impl Generator for ShlReg {
    fn next(&mut self, buf: &mut Vec<u8>) {
        let op = 0b000100;
        let reg1 = self.rng.gen::<u32>() % 32;
        let reg2 = self.rng.gen::<u32>() % 31; // Don't include r31
        buf.write_u16::<LittleEndian>((op << 10) | ((reg2 as u16) << 5) | (reg1 as u16)).unwrap();
    }
}

fn shl_regs<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, test_index: usize) -> Result<(), String> {
    let mut rng = build_rng(test_index);

    let mut rom = Vec::new();

    let mut shl_reg = ShlReg::new(build_rng(rng.gen::<usize>()));

    for _ in 0..1000 {
        shl_reg.next(&mut rom);
    }

    Ret.next(&mut rom);
    
    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

struct ShlImm {
    rng: StdRng,
}

impl ShlImm {
    fn new(rng: StdRng) -> ShlImm {
        ShlImm {
            rng: rng,
        }
    }
}

impl Generator for ShlImm {
    fn next(&mut self, buf: &mut Vec<u8>) {
        let op = 0b010100;
        let imm5 = self.rng.gen::<u32>() % 32;
        let reg2 = self.rng.gen::<u32>() % 31; // Don't include r31
        buf.write_u16::<LittleEndian>((op << 10) | ((reg2 as u16) << 5) | (imm5 as u16)).unwrap();
    }
}

fn shl_imms<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, test_index: usize) -> Result<(), String> {
    let mut rng = build_rng(test_index);

    let mut rom = Vec::new();

    let mut shl_imm = ShlImm::new(build_rng(rng.gen::<usize>()));

    for _ in 0..1000 {
        shl_imm.next(&mut rom);
    }

    Ret.next(&mut rom);
    
    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

struct ShrReg {
    rng: StdRng,
}

impl ShrReg {
    fn new(rng: StdRng) -> ShrReg {
        ShrReg {
            rng: rng,
        }
    }
}

impl Generator for ShrReg {
    fn next(&mut self, buf: &mut Vec<u8>) {
        let op = 0b000101;
        let reg1 = self.rng.gen::<u32>() % 32;
        let reg2 = self.rng.gen::<u32>() % 31; // Don't include r31
        buf.write_u16::<LittleEndian>((op << 10) | ((reg2 as u16) << 5) | (reg1 as u16)).unwrap();
    }
}

fn shr_regs<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, test_index: usize) -> Result<(), String> {
    let mut rng = build_rng(test_index);

    let mut rom = Vec::new();

    let mut shr_reg = ShrReg::new(build_rng(rng.gen::<usize>()));

    for _ in 0..1000 {
        shr_reg.next(&mut rom);
    }

    Ret.next(&mut rom);
    
    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

struct ShrImm {
    rng: StdRng,
}

impl ShrImm {
    fn new(rng: StdRng) -> ShrImm {
        ShrImm {
            rng: rng,
        }
    }
}

impl Generator for ShrImm {
    fn next(&mut self, buf: &mut Vec<u8>) {
        let op = 0b010101;
        let imm5 = self.rng.gen::<u32>() % 32;
        let reg2 = self.rng.gen::<u32>() % 31; // Don't include r31
        buf.write_u16::<LittleEndian>((op << 10) | ((reg2 as u16) << 5) | (imm5 as u16)).unwrap();
    }
}

fn shr_imms<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, test_index: usize) -> Result<(), String> {
    let mut rng = build_rng(test_index);

    let mut rom = Vec::new();

    let mut shr_imm = ShrImm::new(build_rng(rng.gen::<usize>()));

    for _ in 0..1000 {
        shr_imm.next(&mut rom);
    }

    Ret.next(&mut rom);
    
    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

struct Sub {
    rng: StdRng,
}

impl Sub {
    fn new(rng: StdRng) -> Sub {
        Sub {
            rng: rng,
        }
    }
}

impl Generator for Sub {
    fn next(&mut self, buf: &mut Vec<u8>) {
        let op = 0b000010;
        let reg1 = self.rng.gen::<u32>() % 32;
        let reg2 = self.rng.gen::<u32>() % 31; // Don't include r31
        buf.write_u16::<LittleEndian>((op << 10) | ((reg2 as u16) << 5) | (reg1 as u16)).unwrap();
    }
}

fn subs<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, test_index: usize) -> Result<(), String> {
    let mut rng = build_rng(test_index);

    let mut rom = Vec::new();

    let mut sub = Sub::new(build_rng(rng.gen::<usize>()));

    for _ in 0..1000 {
        sub.next(&mut rom);
    }

    Ret.next(&mut rom);
    
    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

struct Xor {
    rng: StdRng,
}

impl Xor {
    fn new(rng: StdRng) -> Xor {
        Xor {
            rng: rng,
        }
    }
}

impl Generator for Xor {
    fn next(&mut self, buf: &mut Vec<u8>) {
        let op = 0b001110;
        let reg1 = self.rng.gen::<u32>() % 32;
        let reg2 = self.rng.gen::<u32>() % 31; // Don't include r31
        buf.write_u16::<LittleEndian>((op << 10) | ((reg2 as u16) << 5) | (reg1 as u16)).unwrap();
    }
}

fn xors<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, test_index: usize) -> Result<(), String> {
    let mut rng = build_rng(test_index);

    let mut rom = Vec::new();

    let mut xor = Xor::new(build_rng(rng.gen::<usize>()));

    for _ in 0..1000 {
        xor.next(&mut rom);
    }

    Ret.next(&mut rom);
    
    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

struct Xori {
    rng: StdRng,
}

impl Xori {
    fn new(rng: StdRng) -> Xori {
        Xori {
            rng: rng,
        }
    }
}

impl Generator for Xori {
    fn next(&mut self, buf: &mut Vec<u8>) {
        let op = 0b101110;
        let reg1 = self.rng.gen::<u32>() % 32;
        let reg2 = self.rng.gen::<u32>() % 31; // Don't include r31
        let imm16 = self.rng.gen::<u16>();
        buf.write_u16::<LittleEndian>((op << 10) | ((reg2 as u16) << 5) | (reg1 as u16)).unwrap();
        buf.write_u16::<LittleEndian>(imm16).unwrap();
    }
}

fn xoris<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, test_index: usize) -> Result<(), String> {
    let mut rng = build_rng(test_index);

    let mut rom = Vec::new();

    let mut xori = Xori::new(build_rng(rng.gen::<usize>()));

    for _ in 0..1000 {
        xori.next(&mut rom);
    }

    Ret.next(&mut rom);
    
    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

struct AddReg {
    rng: StdRng,
}

impl AddReg {
    fn new(rng: StdRng) -> AddReg {
        AddReg {
            rng: rng,
        }
    }
}

impl Generator for AddReg {
    fn next(&mut self, buf: &mut Vec<u8>) {
        let op = 0b000001;
        let reg1 = self.rng.gen::<u32>() % 32;
        let reg2 = self.rng.gen::<u32>() % 31; // Don't include r31
        buf.write_u16::<LittleEndian>((op << 10) | ((reg2 as u16) << 5) | (reg1 as u16)).unwrap();
    }
}

fn add_regs<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, test_index: usize) -> Result<(), String> {
    let mut rng = build_rng(test_index);

    let mut rom = Vec::new();

    let mut add_reg = AddReg::new(build_rng(rng.gen::<usize>()));

    for _ in 0..1000 {
        add_reg.next(&mut rom);
    }

    Ret.next(&mut rom);
    
    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

struct AddImm {
    rng: StdRng,
}

impl AddImm {
    fn new(rng: StdRng) -> AddImm {
        AddImm {
            rng: rng,
        }
    }
}

impl Generator for AddImm {
    fn next(&mut self, buf: &mut Vec<u8>) {
        let op = 0b010001;
        let imm5 = self.rng.gen::<u32>() % 32;
        let reg2 = self.rng.gen::<u32>() % 31; // Don't include r31
        buf.write_u16::<LittleEndian>((op << 10) | ((reg2 as u16) << 5) | (imm5 as u16)).unwrap();
    }
}

fn add_imms<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, test_index: usize) -> Result<(), String> {
    let mut rng = build_rng(test_index);

    let mut rom = Vec::new();

    let mut add_imm = AddImm::new(build_rng(rng.gen::<usize>()));

    for _ in 0..1000 {
        add_imm.next(&mut rom);
    }

    Ret.next(&mut rom);
    
    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

struct AddI {
    rng: StdRng,
}

impl AddI {
    fn new(rng: StdRng) -> AddI {
        AddI {
            rng: rng,
        }
    }
}

impl Generator for AddI {
    fn next(&mut self, buf: &mut Vec<u8>) {
        let op = 0b101001;
        let reg1 = self.rng.gen::<u32>() % 32;
        let reg2 = self.rng.gen::<u32>() % 31; // Don't include r31
        let imm16 = self.rng.gen::<i16>();
        buf.write_u16::<LittleEndian>((op << 10) | ((reg2 as u16) << 5) | (reg1 as u16)).unwrap();
        buf.write_u16::<LittleEndian>(imm16 as u16).unwrap();
    }
}

fn addis<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, test_index: usize) -> Result<(), String> {
    let mut rng = build_rng(test_index);

    let mut rom = Vec::new();

    let mut addi = AddI::new(build_rng(rng.gen::<usize>()));

    for _ in 0..1000 {
        addi.next(&mut rom);
    }

    Ret.next(&mut rom);
    
    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

struct And {
    rng: StdRng,
}

impl And {
    fn new(rng: StdRng) -> And {
        And {
            rng: rng,
        }
    }
}

impl Generator for And {
    fn next(&mut self, buf: &mut Vec<u8>) {
        let op = 0b001101;
        let reg1 = self.rng.gen::<u32>() % 32;
        let reg2 = self.rng.gen::<u32>() % 31; // Don't include r31
        buf.write_u16::<LittleEndian>((op << 10) | ((reg2 as u16) << 5) | (reg1 as u16)).unwrap();
    }
}

fn ands<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, test_index: usize) -> Result<(), String> {
    let mut rng = build_rng(test_index);

    let mut rom = Vec::new();

    let mut and = And::new(build_rng(rng.gen::<usize>()));

    for _ in 0..1000 {
        and.next(&mut rom);
    }

    Ret.next(&mut rom);
    
    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

struct AndI {
    rng: StdRng,
}

impl AndI {
    fn new(rng: StdRng) -> AndI {
        AndI {
            rng: rng,
        }
    }
}

impl Generator for AndI {
    fn next(&mut self, buf: &mut Vec<u8>) {
        let op = 0b101101;
        let reg1 = self.rng.gen::<u32>() % 32;
        let reg2 = self.rng.gen::<u32>() % 31; // Don't include r31
        let imm16 = self.rng.gen::<u16>();
        buf.write_u16::<LittleEndian>((op << 10) | ((reg2 as u16) << 5) | (reg1 as u16)).unwrap();
        buf.write_u16::<LittleEndian>(imm16).unwrap();
    }
}

fn andis<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, test_index: usize) -> Result<(), String> {
    let mut rng = build_rng(test_index);

    let mut rom = Vec::new();

    let mut andi = AndI::new(build_rng(rng.gen::<usize>()));

    for _ in 0..1000 {
        andi.next(&mut rom);
    }

    Ret.next(&mut rom);
    
    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

struct CmpReg {
    rng: StdRng,
}

impl CmpReg {
    fn new(rng: StdRng) -> CmpReg {
        CmpReg {
            rng: rng,
        }
    }
}

impl Generator for CmpReg {
    fn next(&mut self, buf: &mut Vec<u8>) {
        let op = 0b000011;
        let reg1 = self.rng.gen::<u32>() % 32;
        let reg2 = self.rng.gen::<u32>() % 31; // Don't include r31
        buf.write_u16::<LittleEndian>((op << 10) | ((reg2 as u16) << 5) | (reg1 as u16)).unwrap();
    }
}

fn cmp_regs<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, test_index: usize) -> Result<(), String> {
    let mut rng = build_rng(test_index);

    let mut rom = Vec::new();

    let mut cmp_reg = CmpReg::new(build_rng(rng.gen::<usize>()));

    for _ in 0..1000 {
        cmp_reg.next(&mut rom);
    }

    Ret.next(&mut rom);
    
    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

struct CmpImm {
    rng: StdRng,
}

impl CmpImm {
    fn new(rng: StdRng) -> CmpImm {
        CmpImm {
            rng: rng,
        }
    }
}

impl Generator for CmpImm {
    fn next(&mut self, buf: &mut Vec<u8>) {
        let op = 0b010011;
        let imm5 = self.rng.gen::<u32>() % 32;
        let reg2 = self.rng.gen::<u32>() % 31; // Don't include r31
        buf.write_u16::<LittleEndian>((op << 10) | ((reg2 as u16) << 5) | (imm5 as u16)).unwrap();
    }
}

fn cmp_imms<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, test_index: usize) -> Result<(), String> {
    let mut rng = build_rng(test_index);

    let mut rom = Vec::new();

    let mut cmp_imm = CmpImm::new(build_rng(rng.gen::<usize>()));

    for _ in 0..1000 {
        cmp_imm.next(&mut rom);
    }

    Ret.next(&mut rom);
    
    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

struct Mpyhw {
    rng: StdRng,
}

impl Mpyhw {
    fn new(rng: StdRng) -> Mpyhw {
        Mpyhw {
            rng: rng,
        }
    }
}

impl Generator for Mpyhw {
    fn next(&mut self, buf: &mut Vec<u8>) {
        let op = 0b111110;
        let reg1 = self.rng.gen::<u32>() % 32;
        let reg2 = self.rng.gen::<u32>() % 31; // Don't include r31
        let subop = 0b001100;
        buf.write_u16::<LittleEndian>((op << 10) | ((reg2 as u16) << 5) | (reg1 as u16)).unwrap();
        buf.write_u16::<LittleEndian>(subop << 10).unwrap();
    }
}

fn mpyhws<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, test_index: usize) -> Result<(), String> {
    let mut rng = build_rng(test_index);

    let mut rom = Vec::new();

    let mut mpyhw = Mpyhw::new(build_rng(rng.gen::<usize>()));

    for _ in 0..1000 {
        mpyhw.next(&mut rom);
    }

    Ret.next(&mut rom);
    
    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

struct Rev {
    rng: StdRng,
}

impl Rev {
    fn new(rng: StdRng) -> Rev {
        Rev {
            rng: rng,
        }
    }
}

impl Generator for Rev {
    fn next(&mut self, buf: &mut Vec<u8>) {
        let op = 0b111110;
        let reg1 = self.rng.gen::<u32>() % 32;
        let reg2 = self.rng.gen::<u32>() % 31; // Don't include r31
        let subop = 0b001010;
        buf.write_u16::<LittleEndian>((op << 10) | ((reg2 as u16) << 5) | (reg1 as u16)).unwrap();
        buf.write_u16::<LittleEndian>(subop << 10).unwrap();
    }
}

fn revs<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, test_index: usize) -> Result<(), String> {
    let mut rng = build_rng(test_index);

    let mut rom = Vec::new();

    let mut rev = Rev::new(build_rng(rng.gen::<usize>()));

    for _ in 0..1000 {
        rev.next(&mut rom);
    }

    Ret.next(&mut rom);
    
    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

struct Xb {
    rng: StdRng,
}

impl Xb {
    fn new(rng: StdRng) -> Xb {
        Xb {
            rng: rng,
        }
    }
}

impl Generator for Xb {
    fn next(&mut self, buf: &mut Vec<u8>) {
        let op = 0b111110;
        let reg1 = self.rng.gen::<u32>() % 32;
        let reg2 = self.rng.gen::<u32>() % 31; // Don't include r31
        let subop = 0b001000;
        buf.write_u16::<LittleEndian>((op << 10) | ((reg2 as u16) << 5) | (reg1 as u16)).unwrap();
        buf.write_u16::<LittleEndian>(subop << 10).unwrap();
    }
}

fn xbs<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, test_index: usize) -> Result<(), String> {
    let mut rng = build_rng(test_index);

    let mut rom = Vec::new();

    let mut xb = Xb::new(build_rng(rng.gen::<usize>()));

    for _ in 0..1000 {
        xb.next(&mut rom);
    }

    Ret.next(&mut rom);
    
    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

struct Xh {
    rng: StdRng,
}

impl Xh {
    fn new(rng: StdRng) -> Xh {
        Xh {
            rng: rng,
        }
    }
}

impl Generator for Xh {
    fn next(&mut self, buf: &mut Vec<u8>) {
        let op = 0b111110;
        let reg1 = self.rng.gen::<u32>() % 32;
        let reg2 = self.rng.gen::<u32>() % 31; // Don't include r31
        let subop = 0b001001;
        buf.write_u16::<LittleEndian>((op << 10) | ((reg2 as u16) << 5) | (reg1 as u16)).unwrap();
        buf.write_u16::<LittleEndian>(subop << 10).unwrap();
    }
}

fn xhs<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, test_index: usize) -> Result<(), String> {
    let mut rng = build_rng(test_index);

    let mut rom = Vec::new();

    let mut xh = Xh::new(build_rng(rng.gen::<usize>()));

    for _ in 0..1000 {
        xh.next(&mut rom);
    }

    Ret.next(&mut rom);
    
    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

fn multi1<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, test_index: usize) -> Result<(), String> {
    let mut rng = build_rng(test_index);

    let mut rom = Vec::new();

    let mul = Mul::new(build_rng(rng.gen::<usize>()));
    let stsr_psw = StsrPsw::new(build_rng(rng.gen::<usize>()));
    let mut gen = MultiGenerator::new(vec![Box::new(mul), Box::new(stsr_psw)], build_rng(rng.gen::<usize>()));

    // We use a particularly low number here, as otherwise values from r0 will propagate to all other regs eventually.
    //  Need to make sure we test mul more thoroughly among other instr's as well in other tests.
    for _ in 0..200 {
        gen.next(&mut rom);
    }

    Ret.next(&mut rom);

    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

fn multi2<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, test_index: usize) -> Result<(), String> {
    let mut rng = build_rng(test_index);

    let mut rom = Vec::new();

    let mul = Mul::new(build_rng(rng.gen::<usize>()));
    let stsr_psw = StsrPsw::new(build_rng(rng.gen::<usize>()));
    let movea = Movea::new(build_rng(rng.gen::<usize>()));
    let mut gen = MultiGenerator::new(vec![
        Box::new(mul), 
        Box::new(stsr_psw),
        Box::new(movea),
    ], build_rng(rng.gen::<usize>()));

    for _ in 0..1000 {
        gen.next(&mut rom);
    }

    Ret.next(&mut rom);

    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

fn multi3<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, test_index: usize) -> Result<(), String> {
    let mut rng = build_rng(test_index);

    let mut rom = Vec::new();

    let mul = Mul::new(build_rng(rng.gen::<usize>()));
    let stsr_psw = StsrPsw::new(build_rng(rng.gen::<usize>()));
    let movea = Movea::new(build_rng(rng.gen::<usize>()));
    let movhi = Movhi::new(build_rng(rng.gen::<usize>()));
    let mut gen = MultiGenerator::new(vec![
        Box::new(mul), 
        Box::new(stsr_psw),
        Box::new(movea),
        Box::new(movhi),
    ], build_rng(rng.gen::<usize>()));

    for _ in 0..1000 {
        gen.next(&mut rom);
    }

    Ret.next(&mut rom);

    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

fn multi_all<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, test_index: usize) -> Result<(), String> {
    let mut rng = build_rng(test_index);

    let mut rom = Vec::new();

    let mul = Mul::new(build_rng(rng.gen::<usize>()));
    let stsr_psw = StsrPsw::new(build_rng(rng.gen::<usize>()));
    let movea = Movea::new(build_rng(rng.gen::<usize>()));
    let movhi = Movhi::new(build_rng(rng.gen::<usize>()));
    let mov_reg = MovReg::new(build_rng(rng.gen::<usize>()));
    let mov_imm = MovImm::new(build_rng(rng.gen::<usize>()));
    let mulu = Mulu::new(build_rng(rng.gen::<usize>()));
    let not = Not::new(build_rng(rng.gen::<usize>()));
    let or = Or::new(build_rng(rng.gen::<usize>()));
    let ori = Ori::new(build_rng(rng.gen::<usize>()));
    let sar_reg = SarReg::new(build_rng(rng.gen::<usize>()));
    let sar_imm = SarImm::new(build_rng(rng.gen::<usize>()));
    let setf = Setf::new(build_rng(rng.gen::<usize>()));
    let shl_reg = ShlReg::new(build_rng(rng.gen::<usize>()));
    let shl_imm = ShlImm::new(build_rng(rng.gen::<usize>()));
    let shr_reg = ShrReg::new(build_rng(rng.gen::<usize>()));
    let shr_imm = ShrImm::new(build_rng(rng.gen::<usize>()));
    let sub = Sub::new(build_rng(rng.gen::<usize>()));
    let xor = Xor::new(build_rng(rng.gen::<usize>()));
    let xori = Xori::new(build_rng(rng.gen::<usize>()));
    let add_reg = AddReg::new(build_rng(rng.gen::<usize>()));
    let add_imm = AddImm::new(build_rng(rng.gen::<usize>()));
    let addi = AddI::new(build_rng(rng.gen::<usize>()));
    let and = And::new(build_rng(rng.gen::<usize>()));
    let andi = AndI::new(build_rng(rng.gen::<usize>()));
    let cmp_reg = CmpReg::new(build_rng(rng.gen::<usize>()));
    let cmp_imm = CmpImm::new(build_rng(rng.gen::<usize>()));
    let mpyhw = Mpyhw::new(build_rng(rng.gen::<usize>()));
    let rev = Rev::new(build_rng(rng.gen::<usize>()));
    let xb = Xb::new(build_rng(rng.gen::<usize>()));
    let xh = Xh::new(build_rng(rng.gen::<usize>()));
    let mut gen = MultiGenerator::new(vec![
        Box::new(mul),
        Box::new(stsr_psw),
        Box::new(movea),
        Box::new(movhi),
        Box::new(mov_reg),
        Box::new(mov_imm),
        Box::new(mulu),
        Box::new(not),
        Box::new(or),
        Box::new(ori),
        Box::new(sar_reg),
        Box::new(sar_imm),
        Box::new(setf),
        Box::new(shl_reg),
        Box::new(shl_imm),
        Box::new(shr_reg),
        Box::new(shr_imm),
        Box::new(sub),
        Box::new(xor),
        Box::new(xori),
        Box::new(add_reg),
        Box::new(add_imm),
        Box::new(addi),
        Box::new(and),
        Box::new(andi),
        Box::new(cmp_reg),
        Box::new(cmp_imm),
        Box::new(mpyhw),
        Box::new(rev),
        Box::new(xb),
        Box::new(xh),
    ], build_rng(rng.gen::<usize>()));

    for _ in 0..4000 {
        gen.next(&mut rom);
    }

    Ret.next(&mut rom);

    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

fn build_rng(seed: usize) -> StdRng {
    let seed: &[_] = &[seed];
    SeedableRng::from_seed(seed)
}

fn random_regs(rng: &mut StdRng) -> Vec<u32> {
    // Initial regs cover r0-r29 inclusive
    (0..30).map(|_| rng.gen::<u32>()).collect::<Vec<_>>()
}

fn test_rom<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, rom: &[u8], initial_regs: &[u32]) -> Result<(), String> {
    let rom_addr = 0x05000000 + 0x0400;

    {
        use std::fs::File;
        let mut file = File::create("derp.vxe").unwrap();
        file.write_all(&rom).unwrap();
    }

    let hw_result_regs = test_rom_on_port(hw_port, rom, rom_addr, initial_regs).map_err(|e| format!("Hardware dispatch failed: {:?}", e))?;
    let emu_result_regs = test_rom_on_port(emu_port, rom, rom_addr, initial_regs).map_err(|e| format!("Emu dispatch failed: {:?}", e))?;

    println!("regs: [");
    for reg in emu_result_regs.iter() {
        println!("    0x{:08x},", reg);
    }
    println!("],");

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
