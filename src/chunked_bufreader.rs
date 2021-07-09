use std::pin::Pin;

use async_std::prelude::*;
use async_std::io::BufReader;
use async_std::io::Read;
use futures::task::{Context, Poll};
use pin_project::pin_project;

const CHUNK_SIZE: usize = 4096;

#[pin_project]
pub struct ChunkedBufReader<T>
    where T: Read + Unpin {
    #[pin]
    reader: BufReader<T>,
}

impl<T> ChunkedBufReader<T>
    where T: Read + Unpin {
    pub fn new(reader: BufReader<T>) -> Self {
        Self {
            reader,
        }
    }
}

impl<T> Stream for ChunkedBufReader<T>
    where T: Read + Unpin {
    type Item = Vec<u8>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        // This is quite wasteful, but perfomance is fine and the only real optimization (other
        // than some zero-copy shenanigans) would be to not initialize this vec, with unsafe{}
        let mut chunk = vec![0; CHUNK_SIZE];
        match this.reader.poll_read(cx, &mut chunk[0..(CHUNK_SIZE - 1)]) {
            Poll::Ready(Ok(size)) => {
                if size == 0 {
                    Poll::Ready(None)
                } else {
                    chunk.truncate(size);
                    Poll::Ready(Some(chunk))
                }
            },
            Poll::Ready(Err(_)) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}
