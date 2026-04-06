//! Buffered reader for ITCH stream (contiguous slices for parsing).
#![allow(missing_docs)]

use std::io::Read;

use crate::error::FeedError;

/// Buffered reader: fills from inner `Read`, exposes contiguous slice via `get(offset)`.
#[derive(Debug)]
pub struct BufferedReader<R> {
    inner: R,
    buf: Vec<u8>,
    len: usize,
    limit: usize,
    pos: usize,
}

impl<R: Read> BufferedReader<R> {
    pub fn new(capacity: usize, inner: R) -> Self {
        Self {
            inner,
            buf: vec![0; capacity],
            len: capacity,
            limit: 0,
            pos: 0,
        }
    }

    #[inline]
    pub fn get(&self, offset: usize) -> &[u8] {
        let start = self.pos + offset;
        &self.buf[start..self.limit]
    }

    #[inline]
    pub fn available(&self) -> usize {
        self.limit - self.pos
    }

    #[inline]
    pub fn available_n(&self, n: usize) -> bool {
        self.pos + n <= self.limit
    }

    pub fn advance(&mut self, bytes: usize) {
        assert!(self.pos + bytes <= self.limit);
        self.pos += bytes;
    }

    fn discard_to_pos(&mut self) {
        if self.pos > 0 && self.pos < self.limit {
            let copy_len = self.limit - self.pos;
            self.buf.copy_within(self.pos..self.limit, 0);
            self.limit = copy_len;
            self.pos = 0;
        } else if self.pos >= self.limit {
            self.limit = 0;
            self.pos = 0;
        }
    }

    pub fn read(&mut self) -> std::io::Result<usize> {
        if self.pos + (self.len - self.limit) > self.len {
            self.discard_to_pos();
        }
        let n = self.inner.read(&mut self.buf[self.limit..])?;
        self.limit += n;
        Ok(n)
    }

    /// Ensure at least `n` bytes available. Returns Err on I/O error or EOF when more needed.
    pub fn ensure(&mut self, n: usize) -> Result<(), FeedError> {
        if n > self.len {
            return Err(FeedError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("ensure({}) exceeds capacity {}", n, self.len),
            )));
        }
        if self.available_n(n) {
            return Ok(());
        }
        if self.pos + n > self.len {
            self.discard_to_pos();
        }
        while self.available() < n {
            let bytes = self.read().map_err(FeedError::Io)?;
            if bytes == 0 {
                return Err(FeedError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "eof while reading",
                )));
            }
        }
        Ok(())
    }
}
