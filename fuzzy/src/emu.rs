use rustual_boy_core::rom::*;
use rustual_boy_core::sinks::*;
use rustual_boy_core::sram::*;
use rustual_boy_core::virtual_boy::VirtualBoy;

use rustual_boy_middleware::{Anaglyphizer, GammaAdjustSink, MostRecentSink};

use minifb::{WindowOptions, Window, Scale};

use std::collections::VecDeque;
use std::io::{self, Read, Write};

pub struct EmulatedVbSerialPort {
    window: Window,

    virtual_boy: VirtualBoy,
    emulated_time_ns: u64,

    response_buffer: VecDeque<u8>,
}

impl EmulatedVbSerialPort {
    pub fn new() -> EmulatedVbSerialPort {
        let rom = Rom::load("../loader/build/loader.vb").expect("Couldn't load loader ROM for emulated VB");
        let sram = Sram::new();
        let virtual_boy = VirtualBoy::new(rom, sram);

        let mut ret = EmulatedVbSerialPort {
            window: Window::new("Rustual Boy", 384, 224, WindowOptions {
                borderless: false,
                title: true,
                resize: false,
                scale: Scale::X2,
            }).unwrap(),

            virtual_boy: virtual_boy,
            emulated_time_ns: 0,

            response_buffer: VecDeque::new(),
        };

        // Step VB 1s emulated time to let it boot into the test ROM before use
        ret.step_ns(1_000_000_000);

        ret
    }

    fn step_ns(&mut self, ns: u64) {
        const CPU_CYCLE_TIME_NS: u64 = 50;

        let most_recent_sink = MostRecentSink::new();
        let gamma_adjust_sink = GammaAdjustSink::new(most_recent_sink, 2.2);
        let mut video_frame_sink = Anaglyphizer::new(
            gamma_adjust_sink,
            (1.0, 0.0, 0.0).into(),
            (0.0, 1.0, 1.0).into(),
        );

        let target_emulated_time_ns = self.emulated_time_ns + ns;
        while self.emulated_time_ns < target_emulated_time_ns {
            let (emulated_cycles, _) = self.virtual_boy.step(&mut video_frame_sink, &mut NullAudioFrameSink);
            self.emulated_time_ns += (emulated_cycles as u64) * CPU_CYCLE_TIME_NS;
        }

        if let Some(frame) = video_frame_sink.into_inner().into_inner().into_inner() {
            let frame: Vec<u32> = frame.into_iter().map(|x| x.into()).collect();
            self.window.update_with_buffer(&frame);
        }
    }

    fn transfer_byte(&mut self, send_byte: u8) -> u8 {
        let mut received_byte = 0;

        // Transfer 8 bits
        for i in 0..8 {
            // Bring clock low
            //  This is just a delay in this case, since data will be latched on the rising edge
            self.step_ns(5_500);
            // Write out/read in next bit
            received_byte <<= 1;
            received_byte |= self.virtual_boy.interconnect.link_port.transfer_slave_clock_bit(((send_byte as u32) >> (7 - i)) & 1) as u8;
            // Bring clock high (again just a delay)
            self.step_ns(5_500);
        }

        received_byte
    }
}

impl Read for EmulatedVbSerialPort {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut ret = 0;
        for output_byte in buf.iter_mut() {
            match self.response_buffer.pop_front() {
                Some(b) => {
                    *output_byte = b;
                    ret += 1;
                }
                _ => break,
            }
        }
        Ok(ret)
    }
}

impl Write for EmulatedVbSerialPort {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Here we assume that buf contains an entire packet only.
        //  This could be more general, but should be good enough for our use case.
        let send_packet_len = (buf[0] as u32) + 1;
        let send_packet = &buf[1..(1 + send_packet_len) as usize];

        // Perform exchange with emulated VB
        const MAX_HANDSHAKE_TRIES: u32 = 20;

        // Send packet
        {
            //  Send handshake byte until we receive echo back
            let handshake = 0xaa;
            let mut handshake_tries = 0;
            loop {
                let received_byte = self.transfer_byte(handshake);
                if received_byte == handshake {
                    break;
                }

                handshake_tries += 1;
                if handshake_tries >= MAX_HANDSHAKE_TRIES {
                    return Err(io::Error::new(io::ErrorKind::Other, "Emulated VB didn't respond to send handshake"));
                }
            }

            //  Send packet length
            self.transfer_byte((send_packet_len - 1) as u8);

            //  Send data bytes
            for byte in send_packet {
                self.transfer_byte(*byte);
            }
        }

        // Need to wait a small period before reading the receive packet in order to let the VB prepare its response
        //  Note that we can't wait too long, or our exchange will time out
        self.step_ns(100_000);

        // Receive packet
        {
            let handshake = 0x55;
            let mut handshake_tries = 0;
            loop {
                let received_byte = self.transfer_byte(handshake);
                if received_byte == handshake {
                    break;
                }

                handshake_tries += 1;
                if handshake_tries >= MAX_HANDSHAKE_TRIES {
                    return Err(io::Error::new(io::ErrorKind::Other, "Emulated VB didn't respond to receive handshake"));
                }
            }

            //  Receive length
            let received_packet_len = (self.transfer_byte(handshake) as u32) + 1;

            //  Push length byte to response buffer
            self.response_buffer.push_back((received_packet_len - 1) as u8);

            //  Receive data bytes
            for _ in 0..received_packet_len {
                let b = self.transfer_byte(0x00);
                self.response_buffer.push_back(b);
            }
        }

        // Step VB a bit to make sure it's ready for upcoming commands
        self.step_ns(10_000_000);

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/*struct NullVideoFrameSink;

impl Sink<VideoFrame> for NullVideoFrameSink {
    fn append(&mut self, _value: VideoFrame) {
        // Do nothing
    }
}*/

struct NullAudioFrameSink;

impl Sink<AudioFrame> for NullAudioFrameSink {
    fn append(&mut self, _value: AudioFrame) {
        // Do nothing
    }
}
