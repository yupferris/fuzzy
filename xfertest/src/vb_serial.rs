use std::io::{Read, Write};

pub fn blocking_read<R: Read>(r: &mut R, buf: &mut [u8]) {
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

pub fn exchange_packet<P: Read + Write>(port: &mut P, packet: &[u8]) -> Vec<u8> {
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

fn blocking_read_byte<R: Read>(r: &mut R) -> u8 {
    let mut buf = vec![0];
    blocking_read(r, &mut buf);
    buf[0]
}
