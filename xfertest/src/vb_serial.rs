use std::io::{self, Read, Write};

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    DataEmpty,
    DataTooLarge,
}

pub fn exchange_packet<P: Read + Write>(port: &mut P, packet: &[u8]) -> Result<Vec<u8>, Error> {
    if packet.len() == 0 {
        return Err(Error::DataEmpty);
    } else if packet.len() > 256 {
        return Err(Error::DataTooLarge);
    }

    // Send packet
    let packet_len = (packet.len() - 1) as u8;
    let packet_buf = [packet_len].iter().chain(packet.iter()).cloned().collect::<Vec<_>>();
    port.write_all(&packet_buf).unwrap();

    // Receive packet
    //  Receive length
    let received_len = (read_byte(port)? as usize) + 1;

    //  Receive data bytes
    let mut received_packet = vec![0; received_len];
    port.read(&mut received_packet)
        .map(|_| received_packet)
        .map_err(|e| Error::Io(e))
}

fn read_byte<R: Read>(r: &mut R) -> Result<u8, Error> {
    let mut buf = vec![0];
    r.read(&mut buf)
        .map(|_| buf[0])
        .map_err(|e| Error::Io(e))
}
