#[derive(Eq, PartialEq)]
pub struct Crapsum {
    pub state: u32,
}

impl Crapsum {
    pub fn compute(data: &[u8]) -> Crapsum {
        let mut ret = Crapsum::new();
        for byte in data.iter() {
            ret.update(*byte);
        }
        ret
    }

    pub fn from_state(state: u32) -> Crapsum {
        Crapsum {
            state: state,
        }
    }

    pub fn new() -> Crapsum {
        Crapsum::from_state(0xfadebabe)
    }

    pub fn update(&mut self, byte: u8) {
        self.state = (self.state << 3) | (self.state >> 29) ^ (byte as u32);
    }
}
