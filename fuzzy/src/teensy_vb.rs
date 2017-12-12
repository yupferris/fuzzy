use serialport;
use serialport::prelude::*;

use std::ffi::OsStr;
use std::io::Read;
use std::time::Duration;

pub fn connect<P: AsRef<OsStr>>(port: P) -> Result<Box<SerialPort>, String> {
    let mut tries = 0;
    loop {
        let mut port = serialport::open(&port).map_err(|e| format!("Couldn't open serial port: {}", e))?;
        port.set_timeout(Duration::from_millis(1000)).map_err(|e| format!("Couldn't set serial port timeout: {}", e))?;
        port.write_data_terminal_ready(true).map_err(|e| format!("Couldn't set serial port DTR: {}", e))?;

        match wait_for_handshake(&mut port) {
            Ok(_) => {
                return Ok(port);
            }
            Err(e) => {
                tries += 1;
                if tries >= 5 {
                    return Err(format!("Connection failed: {}, too many retries", e));
                }
            }
        }
    }
}

fn wait_for_handshake<R: Read>(r: &mut R) -> Result<(), String> {
    let handshake = b"HANDSHAKE YO";
    let mut handshake_buf = vec![0; handshake.len()];
    if let Err(e) = r.read(&mut handshake_buf) {
        return Err(format!("Couldn't read handshake: {}", e));
    }
    if handshake_buf == handshake {
        Ok(())
    } else {
        Err("Handshake didn't match".into())
    }
}
