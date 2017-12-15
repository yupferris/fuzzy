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
        test!(divs),
        test!(divus),
        test!(multi_all),
        test!(multi_all_stsr_psws),
        test!(multi_all_branches),
    ];

    let mut suite_iteration = 0;

    loop {
        println!("Suite iteration: {}", suite_iteration);

        let num_tests = tests.len();
        let mut passed_tests = 0;
        let mut failed_tests = 0;

        for (index, &(ref test_fn, test_name)) in tests.iter().enumerate() {
            print!("({}) running test `{}` ... ", index, test_name);
            stdout().flush().unwrap();
            match test_fn(&mut hw_port, &mut emu_port, suite_iteration + index) {
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

        if failed_tests > 0 {
            println!("FAILED ON SUITE ITERATION {}", suite_iteration);
            break;
        }

        println!("");

        suite_iteration += 1;
    }
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

struct AlternatingGenerator {
    generators: [Box<Generator>; 2],
    index: usize,
}

impl AlternatingGenerator {
    fn new(a: Box<Generator>, b: Box<Generator>) -> AlternatingGenerator {
        AlternatingGenerator {
            generators: [a, b],
            index: 0,
        }
    }
}

impl Generator for AlternatingGenerator {
    fn next(&mut self, buf: &mut Vec<u8>) {
        self.generators[self.index].next(buf);
        self.index = 1 - self.index;
    }
}

struct BranchingGenerator {
    block_instruction_generator: Box<Generator>,
    rng: StdRng,
}

impl BranchingGenerator {
    fn new(block_instruction_generator: Box<Generator>, rng: StdRng) -> BranchingGenerator {
        BranchingGenerator {
            block_instruction_generator: block_instruction_generator,
            rng: rng,
        }
    }
}

impl Generator for BranchingGenerator {
    fn next(&mut self, buf: &mut Vec<u8>) {
        /*

            block0 {
                ...
                bcond block1
                jr block2
            }

            block1/2 {
                ...
                jr exit
            }

enter:
            jr block0
slot0:
            [one of block0/1/2]
slot1:
            [one of block0/1/2]
slot2:
            [one of block0/1/2]
exit:

        */

        let rom_addr = 0x05000000 + 0x0400;

        // Generate blocks
        let mut blocks = Vec::new();
        for i in 0..3 {
            let mut instructions = Vec::new();
            let num_instrs = self.rng.gen::<u32>() % 3 + 1;
            for _ in 0..num_instrs {
                self.block_instruction_generator.next(&mut instructions);
            }
            let branches = if i == 0 {
                vec![Branch::random_bcond(&mut self.rng), Branch::Jr { addr: None, target: None }]
            } else {
                vec![Branch::Jr { addr: None, target: None }]
            };
            blocks.push(Block::new(instructions, branches));
        }

        // Assign blocks to available slots
        let mut slot_block_indices = [0, 1, 2];
        self.rng.shuffle(&mut slot_block_indices);
        let mut block_slot_indices = [0; 3];
        for i in 0..3 {
            block_slot_indices[slot_block_indices[i]] = i;
        }

        // Flatten blocks in their respective slots
        let enter = rom_addr + (buf.len() as u32);
        let mut enter_branch = Branch::Jr { addr: Some(enter), target: None };
        let slot0 = enter + (enter_branch.len() as u32);
        let slot1 = slot0 + (blocks[slot_block_indices[0]].len() as u32);
        let slot2 = slot1 + (blocks[slot_block_indices[1]].len() as u32);
        let exit = slot2 + (blocks[slot_block_indices[2]].len() as u32);
        blocks[slot_block_indices[0]].flatten(slot0);
        blocks[slot_block_indices[1]].flatten(slot1);
        blocks[slot_block_indices[2]].flatten(slot2);

        // Resolve branch addr's
        enter_branch.set_target(blocks[0].addr.unwrap());
        for i in 0..3 {
            if i == 0 {
                let branch_target = blocks[1].addr.unwrap();
                blocks[i].branches[0].set_target(branch_target);
                let jump_target = blocks[2].addr.unwrap();
                blocks[i].branches[1].set_target(jump_target);
            } else {
                blocks[i].branches[0].set_target(exit);
            }
        }

        // Serialize
        enter_branch.serialize(buf);
        //  Make sure blocks are serialized in slot order
        for i in 0..3 {
            blocks[slot_block_indices[i]].serialize(buf);
        }
    }
}

// TODO: Also support jmp, which requires setting a reg value first
#[derive(Debug)]
enum Branch {
    BCond { addr: Option<u32>, target: Option<u32>, cond: u32 },
    Jr { addr: Option<u32>, target: Option<u32> },
}

impl Branch {
    fn random_bcond(rng: &mut StdRng) -> Branch {
        Branch::BCond { addr: None, target: None, cond: rng.gen::<u32>() & 0x0f }
    }

    fn len(&self) -> usize {
        match self {
            &Branch::BCond { .. } => 2,
            &Branch::Jr { .. } => 4,
        }
    }

    fn set_addr(&mut self, value: u32) {
        let value = Some(value);
        match self {
            &mut Branch::BCond { ref mut addr, .. } => *addr = value,
            &mut Branch::Jr { ref mut addr, .. } => *addr = value,
        }
    }

    fn set_target(&mut self, value: u32) {
        let value = Some(value);
        match self {
            &mut Branch::BCond { ref mut target, .. } => *target = value,
            &mut Branch::Jr { ref mut target, .. } => *target = value,
        }
    }

    fn serialize(&self, buf: &mut Vec<u8>) {
        match self {
            &Branch::BCond { addr, target, cond } => {
                let op = (0b100 << 4) | cond;
                let disp9 = target.unwrap().wrapping_sub(addr.unwrap()) & 0b11111111_1;
                buf.write_u16::<LittleEndian>(((op << 9) | disp9) as u16).unwrap();
            }
            &Branch::Jr { addr, target } => {
                let op = 0b101010;
                let disp26 = target.unwrap().wrapping_sub(addr.unwrap()) & 0b11111111_11111111_11111111_11;
                buf.write_u16::<LittleEndian>(((op << 10) | (disp26 >> 16)) as u16).unwrap();
                buf.write_u16::<LittleEndian>(disp26 as u16).unwrap();
            }
        }
    }
}

#[derive(Debug)]
struct Block {
    addr: Option<u32>,
    instructions: Vec<u8>,
    branches: Vec<Branch>,
}

impl Block {
    fn new(instructions: Vec<u8>, branches: Vec<Branch>) -> Block {
        Block {
            addr: None,
            instructions: instructions,
            branches: branches,
        }
    }

    fn len(&self) -> usize {
        self.instructions.len() + self.branches.iter().map(|x| x.len()).sum::<usize>()
    }

    fn flatten(&mut self, mut addr: u32) {
        self.addr = Some(addr);
        addr += self.instructions.len() as u32;
        for branch in self.branches.iter_mut() {
            branch.set_addr(addr);
            addr += branch.len() as u32;
        }
    }

    fn serialize(&self, buf: &mut Vec<u8>) {
        buf.extend(&self.instructions);
        for branch in self.branches.iter() {
            branch.serialize(buf);
        }
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

fn single_ret<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rom = Vec::new();
    Ret.next(&mut rom);
    
    let initial_regs = random_regs(&mut build_rng(initial_seed));

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

fn muls<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

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

fn stsr_psws<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

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

fn moveas<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

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

fn movhis<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

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

fn mov_regs<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

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

fn mov_imms<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

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

fn mulus<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

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

fn nots<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

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

fn ors<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

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

fn oris<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

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

fn sar_regs<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

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

fn sar_imms<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

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

fn setfs<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

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

fn shl_regs<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

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

fn shl_imms<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

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

fn shr_regs<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

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

fn shr_imms<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

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

fn subs<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

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

fn xors<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

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

fn xoris<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

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

fn add_regs<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

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

fn add_imms<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

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

fn addis<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

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

fn ands<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

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

fn andis<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

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

fn cmp_regs<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

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

fn cmp_imms<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

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

fn mpyhws<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

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

fn revs<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

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

fn xbs<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

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

fn xhs<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

    let mut rom = Vec::new();

    let mut xh = Xh::new(build_rng(rng.gen::<usize>()));

    for _ in 0..1000 {
        xh.next(&mut rom);
    }

    Ret.next(&mut rom);
    
    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

struct Div {
    rng: StdRng,
}

impl Div {
    fn new(rng: StdRng) -> Div {
        Div {
            rng: rng,
        }
    }
}

impl Generator for Div {
    fn next(&mut self, buf: &mut Vec<u8>) {
        let op = 0b001001;
        let reg1 = self.rng.gen::<u32>() % 32;
        let reg2 = self.rng.gen::<u32>() % 31; // Don't include r31
        buf.write_u16::<LittleEndian>((op << 10) | ((reg2 as u16) << 5) | (reg1 as u16)).unwrap();
    }
}

fn divs<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

    let mut rom = Vec::new();

    let mut div = Div::new(build_rng(rng.gen::<usize>()));

    for _ in 0..1000 {
        div.next(&mut rom);
    }

    Ret.next(&mut rom);
    
    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

struct Divu {
    rng: StdRng,
}

impl Divu {
    fn new(rng: StdRng) -> Divu {
        Divu {
            rng: rng,
        }
    }
}

impl Generator for Divu {
    fn next(&mut self, buf: &mut Vec<u8>) {
        let op = 0b001011;
        let reg1 = self.rng.gen::<u32>() % 32;
        let reg2 = self.rng.gen::<u32>() % 31; // Don't include r31
        buf.write_u16::<LittleEndian>((op << 10) | ((reg2 as u16) << 5) | (reg1 as u16)).unwrap();
    }
}

fn divus<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

    let mut rom = Vec::new();

    let mut divu = Divu::new(build_rng(rng.gen::<usize>()));

    for _ in 0..1000 {
        divu.next(&mut rom);
    }

    Ret.next(&mut rom);
    
    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)
}

fn multi1<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

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

fn multi2<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

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

fn multi3<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

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

fn multi_all<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

    let mut rom = Vec::new();

    let mut gen = build_all_generator(&mut rng);

    for _ in 0..4000 {
        gen.next(&mut rom);
    }

    Ret.next(&mut rom);

    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)?;

    Ok(())
}

fn multi_all_stsr_psws<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

    let mut rom = Vec::new();

    let all = build_all_generator(&mut rng);
    let stsr_psw = StsrPsw::new(build_rng(rng.gen::<usize>()));
    let mut gen = AlternatingGenerator::new(Box::new(all), Box::new(stsr_psw));

    for _ in 0..4000 {
        gen.next(&mut rom);
    }

    Ret.next(&mut rom);

    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)?;

    Ok(())
}

fn multi_all_branches<HwP: Read + Write, EmuP: Read + Write>(hw_port: &mut HwP, emu_port: &mut EmuP, initial_seed: usize) -> Result<(), String> {
    let mut rng = build_rng(initial_seed);

    let mut rom = Vec::new();

    let all = build_all_generator(&mut rng);
    let mut gen = BranchingGenerator::new(Box::new(all), build_rng(rng.gen::<usize>()));

    // Less iterations, as the branching generator will generate full blocks, not invididual instrs
    for _ in 0..1000 {
        gen.next(&mut rom);
    }

    Ret.next(&mut rom);

    let initial_regs = random_regs(&mut build_rng(rng.gen::<usize>()));

    test_rom(hw_port, emu_port, &rom, &initial_regs)?;

    Ok(())
}

fn build_all_generator(rng: &mut StdRng) -> MultiGenerator {
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
    let div = Div::new(build_rng(rng.gen::<usize>()));
    let divu = Divu::new(build_rng(rng.gen::<usize>()));
    MultiGenerator::new(vec![
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
        Box::new(div),
        Box::new(divu),
    ], build_rng(rng.gen::<usize>()))
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

    /*{
        use std::fs::File;
        let mut file = File::create("derp.vxe").unwrap();
        file.write_all(&rom).unwrap();
    }*/

    let hw_result_regs = test_rom_on_port(hw_port, rom, rom_addr, initial_regs).map_err(|e| format!("Hardware dispatch failed: {:?}", e))?;
    let emu_result_regs = test_rom_on_port(emu_port, rom, rom_addr, initial_regs).map_err(|e| format!("Emu dispatch failed: {:?}", e))?;

    if hw_result_regs == emu_result_regs {
        Ok(())
    } else {
        Err(
            String::from("regs (hw, emu): [") +
            &hw_result_regs.iter().zip(emu_result_regs.iter()).fold(String::new(), |acc, (hw_reg, emu_reg)| {
                acc + &format!("    (0x{:08x}, 0x{:08x}, {})", hw_reg, emu_reg, if hw_reg == emu_reg { "match" } else { "mismatch!" })
            }) +
            "],")
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
