// src/ipc/pipe.rs -- anonymous kernel pipe (ring buffer, no blocking yet)

use alloc::vec::Vec;
use spin::Mutex;

pub const PIPE_BUF: usize = 4096;

pub struct Pipe {
    inner: Mutex<PipeInner>,
}

struct PipeInner {
    buf:  Vec<u8>,
    read_pos:  usize,
    write_pos: usize,
    filled:    usize,
}

impl Pipe {
    pub fn new() -> Self {
        let mut buf = Vec::with_capacity(PIPE_BUF);
        buf.resize(PIPE_BUF, 0);
        Self {
            inner: Mutex::new(PipeInner {
                buf, read_pos: 0, write_pos: 0, filled: 0,
            }),
        }
    }

    /// write bytes into the pipe; returns number of bytes written
    pub fn write(&self, data: &[u8]) -> usize {
        let mut inner = self.inner.lock();
        let space = PIPE_BUF - inner.filled;
        let n     = data.len().min(space);

        for i in 0..n {
            inner.buf[inner.write_pos] = data[i];
            inner.write_pos = (inner.write_pos + 1) % PIPE_BUF;
        }
        inner.filled += n;
        n
    }

    /// read bytes from the pipe; returns number of bytes read
    pub fn read(&self, buf: &mut [u8]) -> usize {
        let mut inner = self.inner.lock();
        let n = buf.len().min(inner.filled);

        for i in 0..n {
            buf[i] = inner.buf[inner.read_pos];
            inner.read_pos = (inner.read_pos + 1) % PIPE_BUF;
        }
        inner.filled -= n;
        n
    }

    pub fn available(&self) -> usize { self.inner.lock().filled }
    pub fn is_empty(&self)  -> bool  { self.available() == 0 }
}

impl Default for Pipe {
    fn default() -> Self { Self::new() }
}
