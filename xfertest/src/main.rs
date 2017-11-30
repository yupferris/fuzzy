extern crate serialport;
extern crate rand;
extern crate time;

use serialport::prelude::*;

use rand::Rng;

//use time::precise_time_ns;

use std::io::{Read, stdout, Write};
use std::mem::transmute;
use std::thread;
use std::time::Duration;

#[derive(Eq, PartialEq)]
struct Crapsum {
    state: u32,
}

impl Crapsum {
    fn compute(data: &[u8]) -> Crapsum {
        let mut ret = Crapsum::new();
        for byte in data.iter() {
            ret.update(*byte);
        }
        ret
    }

    fn from_state(state: u32) -> Crapsum {
        Crapsum {
            state: state,
        }
    }

    fn new() -> Crapsum {
        Crapsum::from_state(0xfadebabe)
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
        let received_packet = vb_exchange_packet(&mut port, &packet);
        let elapsed_time = precise_time_ns() - start_time;
        let bytes_sec = (((packet.len() + received_packet.len()) as f64) / ((elapsed_time as f64) / 1e9)) as u32;

        let crapsum_bytes: [u8; 4] = unsafe { transmute(crapsum.state.to_le()) };

        if received_packet == crapsum_bytes {
            println!("ok {}b/s", bytes_sec);
        } else {
            panic!("crapsum didn't match! {:?}", received_packet);
        }*/

        let message = b"    * This came from the PC you guys!! *    ";
        let data = message.iter().flat_map(|x| vec![*x, 0].into_iter()).collect::<Vec<_>>();

        let mut rng = rand::thread_rng();
        let row = rng.gen::<u32>() % 28;
        let col = rng.gen::<u32>() % 48;
        let addr = 0x00020000 + (row * 64 + col) * 2;

        print!("({}) sending packet ... ", packet_index);
        stdout().flush().unwrap();

        fuzzy_write_mem_region(&mut port, addr, &data).unwrap();

        println!("ok");

        //thread::sleep(Duration::from_millis(100));

        packet_index += 1;
    }
}

enum FuzzyCommand {
    WriteMemRegion { addr: u32, data: Vec<u8> },
}

#[derive(Eq, PartialEq)]
enum FuzzyResponse {
    OkWithCrapsum(Crapsum),
}

impl FuzzyResponse {
    fn parse(data: Vec<u8>) -> Result<FuzzyResponse, FuzzyError> {
        if data.is_empty() {
            return Err(FuzzyError::InvalidResponse(data));
        }

        match data[0] {
            0x00 => {
                if data.len() != 5 {
                    return Err(FuzzyError::InvalidResponse(data));
                }

                let mut state = 0;
                for i in 1..5 {
                    state >>= 8;
                    state |= (data[i] as u32) << 24;
                }

                Ok(FuzzyResponse::OkWithCrapsum(Crapsum::from_state(state)))
            }
            _ => Err(FuzzyError::InvalidResponse(data))
        }
    }
}

#[derive(Debug)]
enum FuzzyError {
    DataEmpty,
    DataTooLarge,
    WrongCrapsum,
    InvalidResponse(Vec<u8>),
}

fn fuzzy_write_mem_region<P: Read + Write>(port: &mut P, addr: u32, data: &[u8]) -> Result<(), FuzzyError> {
    if data.is_empty() {
        return Err(FuzzyError::DataEmpty);
    }

    let mut data_offset = 0;
    loop {
        let mut packet_len = data.len() - data_offset;
        if packet_len > 256 - 5 {
            packet_len = 256 - 5;
        }

        if packet_len == 0 {
            break;
        }

        let addr = addr + (data_offset as u32);
        let data = data[data_offset..data_offset + packet_len].iter().cloned().collect::<Vec<_>>();
        
        let (response, expected_crapsum) = fuzzy_issue_command(port, FuzzyCommand::WriteMemRegion { addr: addr, data: data })?;

        match response {
            FuzzyResponse::OkWithCrapsum(crapsum) => {
                if crapsum != expected_crapsum {
                    return Err(FuzzyError::WrongCrapsum);
                }
            }
        }

        data_offset += packet_len;
    }

    // TODO: Issue status command, make sure it's OK

    Ok(())
}

/*fn fuzzy_read_mem_region<P: Read + Write>(port: &mut P, addr: u32) -> Result<Vec<u8>, FuzzyError> {
    Ok(Vec::new()) // TODO
}*/

/*fn fuzzy_call<P: Read + Write>(port: &mut P, entry: u32) {
    // TODO
}*/

fn fuzzy_issue_command<P: Read + Write>(port: &mut P, command: FuzzyCommand) -> Result<(FuzzyResponse, Crapsum), FuzzyError> {
    let packet = match command {
        FuzzyCommand::WriteMemRegion { addr, data } => {
            if data.is_empty() {
                return Err(FuzzyError::DataEmpty);
            }

            if data.len() > 256 - 5 {
                return Err(FuzzyError::DataTooLarge);
            }

            let addr_bytes: [u8; 4] = unsafe { transmute(addr.to_le()) };

            vec![0x00].iter()
                .chain(addr_bytes.iter())
                .chain(data.iter())
                .cloned()
                .collect::<Vec<_>>()
        }
    };
    let packet_crapsum = Crapsum::compute(&packet);
    FuzzyResponse::parse(vb_exchange_packet(port, &packet)).map(|response| (response, packet_crapsum))
}

fn vb_exchange_packet<P: Read + Write>(port: &mut P, packet: &[u8]) -> Vec<u8> {
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
