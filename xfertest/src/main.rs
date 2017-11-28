extern crate serialport;
extern crate rand;
extern crate time;

use serialport::prelude::*;

use rand::Rng;

use time::precise_time_ns;

use std::io::{Read, stdout, Write};
use std::mem::transmute;
//use std::thread;
//use std::time::Duration;

struct Crapsum {
    state: u32,
}

impl Crapsum {
    fn new() -> Crapsum {
        Crapsum {
            state: 0xfadebabe,
        }
    }

    fn update(&mut self, byte: u8) {
        self.state = (self.state << 3) | (self.state >> 29) ^ (byte as u32);
    }
}

fn main() {
    let mut port = vb_connect();

    // Go!
    let mut packet_index = 0;
    loop {
        // Test echo
        //let packet = vec![0xfa, 0xde, 0xba, 0xbe];
        let mut rng = rand::thread_rng();
        let len = (rng.gen::<u8>() as usize) + 1;
        let mut packet = Vec::with_capacity(len);
        let mut crapsum = Crapsum::new();
        for _ in 0..len {
            let byte = rng.gen::<u8>();
            packet.push(byte);
            crapsum.update(byte);
        }

        print!("({}) sending packet ({} bytes, 0x{:08x}) ... ", packet_index, packet.len(), crapsum.state);
        stdout().flush().unwrap();

        let start_time = precise_time_ns();
        let received_packet = vb_transfer_packet(&mut port, &packet);
        let elapsed_time = precise_time_ns() - start_time;
        let bytes_sec = (((packet.len() + received_packet.len()) as f64) / ((elapsed_time as f64) / 1e9)) as u32;

        let crapsum_bytes: [u8; 4] = unsafe { transmute(crapsum.state.to_le()) };

        if received_packet == crapsum_bytes {
            println!("ok {}b/s", bytes_sec);
        } else {
            panic!("crapsum didn't match! {:?}", received_packet);
        }

        packet_index += 1;

        //thread::sleep(Duration::from_millis(100));
    }
}

fn vb_transfer_packet<P: Read + Write>(port: &mut P, packet: &[u8]) -> Vec<u8> {
    if packet.len() == 0 {
        panic!("Can't send 0-length packets");
    } else if packet.len() > 256 {
        panic!("Can't send packets larger than 256 bytes");
    }

    // Send packet
    let packet_len = (packet.len() - 1) as u8;
    let packet_buf = [packet_len].iter().chain(packet.iter()).cloned().collect::<Vec<_>>();
    port.write_all(&packet_buf).unwrap();

    // Receive packet
    //  Receive length
    let received_len = (blocking_read_byte(port) as usize) + 1;

    //  Receive data bytes
    let mut received_packet = vec![0; received_len];
    blocking_read(port, &mut received_packet);

    received_packet
}

fn vb_connect() -> Box<SerialPort> {
    let mut tries = 0;
    loop {
        let mut port = serialport::open("COM4").unwrap();
        port.write_data_terminal_ready(true).unwrap();

        match wait_for_handshake(&mut port) {
            Ok(_) => {
                return port;
            }
            Err(e) => {
                tries += 1;
                if tries >= 5 {
                    panic!("Connection failed: {}, too many retries", e);
                }
            }
        }
    }
}

fn wait_for_handshake<R: Read>(r: &mut R) -> Result<(), String> {
    let handshake = b"HANDSHAKE YO";
    let mut handshake_buf = vec![0; handshake.len()];
    blocking_read(r, &mut handshake_buf);
    if handshake_buf == handshake {
        Ok(())
    } else {
        Err("Handshake didn't match".into())
    }
}

fn blocking_read_byte<R: Read>(r: &mut R) -> u8 {
    let mut buf = vec![0];
    blocking_read(r, &mut buf);
    buf[0]
}

fn blocking_read<R: Read>(r: &mut R, buf: &mut [u8]) {
    let mut read_offset = 0;

    loop {
        if let Ok(num_bytes) = r.read(&mut buf[read_offset..]) {
            read_offset += num_bytes;
            if read_offset == buf.len() {
                break;
            }
        }
        //thread::sleep(Duration::from_millis(1));
    }
}
