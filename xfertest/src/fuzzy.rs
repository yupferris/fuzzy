use crapsum::*;
use vb_serial::{self, exchange_packet};

use std::io::{Read, Write};
use std::mem::transmute;

#[derive(Debug)]
pub enum Error {
    Serial(vb_serial::Error),
    DataEmpty,
    DataTooLarge,
    ZeroLength,
    ProtocolViolation,
    WrongCrapsum,
    InvalidResponse(Vec<u8>),
}

enum Command {
    CheckStatus,
    WriteMemRegion { addr: u32, data: Vec<u8> },
    ReadMemRegion { addr: u32, length: u32 },
    ReadMemRegionData,
}

#[derive(Eq, PartialEq)]
enum Response {
    UnexpectedCommand,
    OkWithCrapsum(Crapsum),
    ReadMemRegionData(Vec<u8>),
}

impl Response {
    fn parse(data: Vec<u8>) -> Result<Response, Error> {
        if data.is_empty() {
            return Err(Error::InvalidResponse(data));
        }

        match data[0] {
            0x00 => {
                if data.len() != 1 {
                    return Err(Error::InvalidResponse(data));
                }

                Ok(Response::UnexpectedCommand)
            }
            0x01 => {
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
            0x02 => {
                if data.len() < 2 {
                    return Err(Error::InvalidResponse(data));
                }

                Ok(Response::ReadMemRegionData(data[1..].iter().cloned().collect()))
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
            _ => {
                return Err(Error::ProtocolViolation);
            }
        }

        let mut status_tries = 0;
        loop {
            if let Ok((response, expected_crapsum)) = issue_command(port, Command::CheckStatus) {
                match response {
                    Response::OkWithCrapsum(crapsum) => {
                        if crapsum != expected_crapsum {
                            return Err(Error::WrongCrapsum);
                        }

                        break;
                    }
                    _ => {
                        return Err(Error::ProtocolViolation);
                    }
                }
            }

            status_tries += 1;
            if status_tries >= 5 {
                return Err(Error::ProtocolViolation);
            }
        }

        data_offset += packet_len;
    }

    Ok(())
}

pub fn read_mem_region<P: Read + Write>(port: &mut P, addr: u32, length: u32) -> Result<Vec<u8>, Error> {
    if length == 0 {
        return Err(Error::ZeroLength);
    }

    let mut ret = Vec::new();

    let mut data_offset = 0;
    loop {
        let mut packet_len = (length as usize) - data_offset;
        if packet_len > 256 - 1 {
            packet_len = 256 - 1;
        }

        if packet_len == 0 {
            break;
        }

        let addr = addr + (data_offset as u32);

        let (response, expected_crapsum) = issue_command(port, Command::ReadMemRegion { addr: addr, length: packet_len as u32 })?;

        match response {
            Response::OkWithCrapsum(crapsum) => {
                if crapsum != expected_crapsum {
                    return Err(Error::WrongCrapsum);
                }
            }
            _ => {
                return Err(Error::ProtocolViolation);
            }
        }

        let mut read_data_tries = 0;
        loop {
            if let Ok((response, _)) = issue_command(port, Command::ReadMemRegionData) {
                match response {
                    Response::ReadMemRegionData(data) => {
                        if data.len() != packet_len {
                            return Err(Error::ProtocolViolation);
                        }

                        ret.extend(data);
                        break;
                    }
                    _ => {
                        return Err(Error::ProtocolViolation);
                    }
                }
            }

            read_data_tries += 1;
            if read_data_tries >= 5 {
                return Err(Error::ProtocolViolation);
            }
        }

        data_offset += packet_len;
    }

    Ok(ret)
}

fn issue_command<P: Read + Write>(port: &mut P, command: Command) -> Result<(Response, Crapsum), Error> {
    let packet = match command {
        Command::CheckStatus => vec![0x00],
        Command::WriteMemRegion { addr, data } => {
            if data.is_empty() {
                return Err(Error::DataEmpty);
            }

            if data.len() > 256 - 5 {
                return Err(Error::DataTooLarge);
            }

            let addr_bytes: [u8; 4] = unsafe { transmute(addr.to_le()) };

            vec![0x01].iter()
                .chain(addr_bytes.iter())
                .chain(data.iter())
                .cloned()
                .collect::<Vec<_>>()
        }
        Command::ReadMemRegion { addr, length } => {
            if length == 0 {
                return Err(Error::ZeroLength);
            }

            let addr_bytes: [u8; 4] = unsafe { transmute(addr.to_le()) };

            vec![0x02].iter()
                .chain(addr_bytes.iter())
                .chain(vec![(length - 1) as u8].iter())
                .cloned()
                .collect::<Vec<_>>()
        }
        Command::ReadMemRegionData => vec![0x03],
    };
    let packet_crapsum = Crapsum::compute(&packet);
    let received_packet = exchange_packet(port, &packet).map_err(|e| Error::Serial(e))?;
    Response::parse(received_packet).map(|response| (response, packet_crapsum))
}
