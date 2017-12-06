use crapsum::*;
use vb_serial::{self, exchange_packet};

use std::io::{Read, Write};
use std::mem::transmute;

#[derive(Debug)]
pub enum Error {
    Serial(vb_serial::Error),
    DataEmpty,
    DataTooLarge,
    WrongCrapsum,
    InvalidResponse(Vec<u8>),
}

enum Command {
    WriteMemRegion { addr: u32, data: Vec<u8> },
}

#[derive(Eq, PartialEq)]
enum Response {
    OkWithCrapsum(Crapsum),
}

impl Response {
    fn parse(data: Vec<u8>) -> Result<Response, Error> {
        if data.is_empty() {
            return Err(Error::InvalidResponse(data));
        }

        match data[0] {
            0x00 => {
                if data.len() != 5 {
                    return Err(Error::InvalidResponse(data));
                }

                let mut state = 0;
                for i in 1..5 {
                    state >>= 8;
                    state |= (data[i] as u32) << 24;
                }

                Ok(Response::OkWithCrapsum(Crapsum::from_state(state)))
            }
            _ => Err(Error::InvalidResponse(data))
        }
    }
}

pub fn write_mem_region<P: Read + Write>(port: &mut P, addr: u32, data: &[u8]) -> Result<(), Error> {
    if data.is_empty() {
        return Err(Error::DataEmpty);
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
        
        let (response, expected_crapsum) = issue_command(port, Command::WriteMemRegion { addr: addr, data: data })?;

        match response {
            Response::OkWithCrapsum(crapsum) => {
                if crapsum != expected_crapsum {
                    return Err(Error::WrongCrapsum);
                }
            }
        }

        data_offset += packet_len;
    }

    // TODO: Issue status command, make sure it's OK (THIS IS ACTUALLY NECESSARY, WE'VE SEEN IT BREAK NOW!)

    Ok(())
}

fn issue_command<P: Read + Write>(port: &mut P, command: Command) -> Result<(Response, Crapsum), Error> {
    let packet = match command {
        Command::WriteMemRegion { addr, data } => {
            if data.is_empty() {
                return Err(Error::DataEmpty);
            }

            if data.len() > 256 - 5 {
                return Err(Error::DataTooLarge);
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
    let received_packet = exchange_packet(port, &packet).map_err(|e| Error::Serial(e))?;
    Response::parse(received_packet).map(|response| (response, packet_crapsum))
}
