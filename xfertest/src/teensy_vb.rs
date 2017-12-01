use serialport;
use serialport::prelude::*;

use vb_serial::*;

use std::ffi::OsStr;
use std::io::Read;

pub fn connect<P: AsRef<OsStr>>(port: P) -> Box<SerialPort> {
    let mut tries = 0;
    loop {
        let mut port = serialport::open(&port).unwrap();
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
