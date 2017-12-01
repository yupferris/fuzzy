extern crate serialport;
extern crate rand;
extern crate time;

mod crapsum;
mod fuzzy;
mod teensy_vb;
mod vb_serial;

//use rand::Rng;

use time::precise_time_ns;

use std::f64::consts::PI;
use std::io::{stdout, Write};
//use std::thread;
//use std::time::Duration;

fn main() {
    let mut port = teensy_vb::connect("COM4");

    // Total overkill using this kind of timer but I already have it in scope and whatnot.. :D
    let start_time = precise_time_ns();

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

        /*let message = b"    * This came from the PC you guys!! *    ";
        let data = message.iter().flat_map(|x| vec![*x, 0].into_iter()).collect::<Vec<_>>();*/

        let time = (precise_time_ns().wrapping_sub(start_time) as f64) / 1e9;
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

        /*let mut rng = rand::thread_rng();
        let row = rng.gen::<u32>() % 28;
        let col = rng.gen::<u32>() % 48;*/
        let addr = 0x00020000;// + (row * 64 + col) * 2;

        print!("({}) sending packet ... ", packet_index);
        stdout().flush().unwrap();

        fuzzy::write_mem_region(&mut port, addr, &data).unwrap();

        println!("ok");

        //thread::sleep(Duration::from_millis(100));

        packet_index += 1;
    }
}
