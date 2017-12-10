extern crate serialport;
extern crate rand;
extern crate time;
extern crate byteorder;

mod crapsum;
mod fuzzy;
mod teensy_vb;
mod transport;

//use rand::Rng;

//use time::precise_time_ns;

use byteorder::{LittleEndian, WriteBytesExt};

//use std::f64::consts::PI;
use std::fs::File;
use std::io::{stdout, Read, Write};
//use std::thread;
//use std::time::Duration;

fn main() {
    let mut port = teensy_vb::connect("COM4").expect("Couldn't connect to teensy");

    let rom = {
        let mut file = File::open("../flatrom/build/testrom.vxe").expect("Couldn't open ROM file");
        let mut rom = Vec::new();
        file.read_to_end(&mut rom).expect("Couldn't read ROM");
        rom
    };

    // Total overkill using this kind of timer but I already have it in scope and whatnot.. :D
    //let start_time = precise_time_ns();

    // Go!
    let mut packet_index = 0;
    loop {
        // Test echo
        //let packet = vec![0xfa, 0xde, 0xba, 0xbe];
        /*let mut rng = rand::thread_rng();
        let len = (rng.gen::<u8>() as usize) + 1;
        let mut packet = Vec::with_capacity(len);
        for _ in 0..len {
            packet.push(rng.gen::<u8>());
        }
        let crapsum = Crapsum::compute(&packet);

        print!("({}) sending packet ({} bytes, 0x{:08x}) ... ", packet_index, packet.len(), crapsum.state);
        stdout().flush().unwrap();

        let start_time = precise_time_ns();
        let received_packet = teensy_vb_exchange_packet(&mut port, &packet);
        let elapsed_time = precise_time_ns() - start_time;
        let bytes_sec = (((packet.len() + received_packet.len()) as f64) / ((elapsed_time as f64) / 1e9)) as u32;

        let crapsum_bytes: [u8; 4] = unsafe { transmute(crapsum.state.to_le()) };

        if received_packet == crapsum_bytes {
            println!("ok {}b/s", bytes_sec);
        } else {
            panic!("crapsum didn't match! {:?}", received_packet);
        }*/

        /*let time = (precise_time_ns().wrapping_sub(start_time) as f64) / 1e9;
        let x_flow_time = time * 0.3;
        let y_flow_time = time * -0.24 + 1.0;
        let blocks =
            (0..28).flat_map(|y| {
                let fy = (y as f64) / 20.0;
                (0..64).map(move |x| {
                    let fx = (x as f64) / 20.0;
                    let x_sinus = ((fx + x_flow_time).sin() + (-(fx * 2.0 * PI + x_flow_time)).sin() * 0.3).sin() * 2.0 * PI;
                    let y_sinus = ((fy + y_flow_time).sin() + (-(fy * 2.0 * PI + y_flow_time)).sin() * 0.3).sin() * 2.0 * PI;
                    let chars = [0, 183, 149, 111, 79, 48, 19, 18];
                    let mut char_index = ((((x_sinus + y_sinus) * 2.0).sin() * 0.5 + 0.5) * (chars.len() as f64)) as usize;
                    if char_index >= chars.len() {
                        char_index = chars.len() - 1;
                    }
                    chars[char_index as usize]
                })
            })
            .collect::<Vec<_>>();
        let data = blocks.iter().flat_map(|x| vec![*x, 0].into_iter()).collect::<Vec<_>>();

        let addr = 0x00020000;

        print!("({}) issuing write command ... ", packet_index);
        stdout().flush().unwrap();

        match fuzzy::write_mem_region(&mut port, addr, &data) {
            Ok(_) => {
                println!("ok");
            }
            Err(e) => {
                println!("error: {:?}", e);
            }
        }

        print!("({}) issuing read command ... ", packet_index);
        stdout().flush().unwrap();

        match fuzzy::read_mem_region(&mut port, addr, data.len() as u32) {
            Ok(read_data) => {
                if read_data == data {
                    println!("ok");
                } else {
                    println!("error: read data did not match");
                }
            }
            Err(e) => {
                println!("error: {:?}", e);
            }
        }*/

        let rom_addr = 0x05000000 + 0x2000;

        if let Err(e) = test_rom(&mut port, &rom, rom_addr, packet_index) {
            println!("error: {:?}", e);
        }

        //thread::sleep(Duration::from_millis(100));

        packet_index += 1;
    }
}

fn test_rom<P: Read + Write>(port: &mut P, rom: &[u8], rom_addr: u32, packet_index: u32) -> Result<(), fuzzy::Error> {
    print!("({}) issuing rom write command ... ", packet_index);
    stdout().flush().unwrap();

    fuzzy::write_mem_region(port, rom_addr, &rom)?;

    println!("ok");

    let initial_regs_addr = 0x0001e000;
    let initial_regs = [0xdeadbeef; 30]; // Initial regs cover r0-r29 inclusive
    let initial_regs_bytes = initial_regs.iter().flat_map(|x| {
        let mut bytes = Vec::new();
        bytes.write_u32::<LittleEndian>(*x).unwrap();
        bytes
    }).collect::<Vec<_>>();

    print!("({}) issuing initial reg write command ... ", packet_index);
    stdout().flush().unwrap();

    fuzzy::write_mem_region(port, initial_regs_addr, &initial_regs_bytes)?;

    println!("ok");

    let exec_entry = rom_addr;

    print!("({}) issuing execute command ... ", packet_index);
    stdout().flush().unwrap();

    fuzzy::execute(port, exec_entry)?;

    println!("ok");

    let result_regs_addr = initial_regs_addr + 32 * 4;

    print!("({}) issuing read result regs command ... ", packet_index);
    stdout().flush().unwrap();

    let result_regs_bytes = fuzzy::read_mem_region(port, result_regs_addr, 32 * 4)?;

    println!("ok, result regs: [");
    for i in 0..32 {
        let mut reg = 0;
        for j in 0..4 {
            reg >>= 8;
            reg |= (result_regs_bytes[(i * 4 + j) as usize] as u32) << 24;
        }
        print!("    ");
        if i < 31 { print!("r{}", i) } else { print!("psw") };
        println!(": 0x{:08x}", reg);
    }
    println!("]");

    Ok(())
}
