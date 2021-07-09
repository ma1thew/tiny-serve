use async_std::prelude::*;
use async_std::io::BufReader;
use async_std::io::Read;

pub struct PeekableBufReader<T>
    where T: Read + Unpin {
    reader: BufReader<T>,
    buffer: [u8; 1],
    peeked_last: bool,
}

impl<T> PeekableBufReader<T>
    where T: Read + Unpin {
    pub fn new(reader: BufReader<T>) -> Self {
        Self {
            reader,
            buffer: [0],
            peeked_last: false,
        }
    }

    pub async fn next(&mut self) -> Option<u8> {
        if self.peeked_last {
            self.peeked_last = false;
            Some(self.buffer[0])
        } else {
            match self.reader.read(&mut self.buffer).await.ok()? {
                1 => Some(self.buffer[0]),
                _ => None,
            }
        }
    }

    pub async fn peek(&mut self) -> Option<&u8> {
        if self.peeked_last {
            Some(&self.buffer[0])
        } else {
            match self.reader.read(&mut self.buffer).await.ok()? {
                1 => {
                    self.peeked_last = true;
                    Some(&self.buffer[0])
                },
                _ => None,
            }
        }
    }
}
