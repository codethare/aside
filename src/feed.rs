//! Background reader: a thread owns the pipe and the UI drains whatever has
//! arrived on each frame tick. No extra event-loop wakeup source, so an idle
//! panel stays idle — the price is one frame of latency on the first delta of
//! an answer, which is a 16 ms tick.

use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Bytes read so far, plus whether the reader has finished.
pub struct Feed {
    arrived: Mutex<Vec<u8>>,
    eof: AtomicBool,
}

impl Feed {
    /// Read `r` on a thread until it ends. The thread dies with the process;
    /// there is nothing to join, because nothing is persisted.
    pub fn spawn(mut r: impl Read + Send + 'static) -> Arc<Feed> {
        let feed = Arc::new(Feed {
            arrived: Mutex::new(Vec::new()),
            eof: AtomicBool::new(false),
        });
        let writer = Arc::clone(&feed);
        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match r.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => writer.arrived.lock().unwrap().extend_from_slice(&buf[..n]),
                    Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(_) => break,
                }
            }
            writer.eof.store(true, Ordering::Release);
        });
        feed
    }

    /// Take everything that has arrived since the last call, in order.
    pub fn take(&self) -> Vec<u8> {
        std::mem::take(&mut self.arrived.lock().unwrap())
    }

    /// True once the reader has hit end of file (or failed). The panel does not
    /// need it — the child exiting is what ends a request — but the reader's
    /// contract is worth asserting.
    #[cfg(test)]
    pub fn eof(&self) -> bool {
        self.eof.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    struct Chunks(Vec<&'static [u8]>);

    impl Read for Chunks {
        fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
            if self.0.is_empty() {
                return Ok(0);
            }
            let next = self.0.remove(0);
            out[..next.len()].copy_from_slice(next);
            std::thread::sleep(Duration::from_millis(2));
            Ok(next.len())
        }
    }

    #[test]
    fn chunks_arrive_in_order_and_the_flag_follows_the_end() {
        let feed = Feed::spawn(Chunks(vec![b"data: one\n", b"data: two\n", b"[DONE]\n"]));
        let mut seen = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            seen.extend_from_slice(&feed.take());
            if feed.eof() {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        seen.extend_from_slice(&feed.take());
        assert_eq!(seen, b"data: one\ndata: two\n[DONE]\n");
        assert!(feed.eof(), "the reader must report the end of the stream");
    }

    #[test]
    fn taking_drains_so_a_second_call_returns_nothing() {
        let feed = Feed::spawn(std::io::Cursor::new(b"abc".to_vec()));
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut first = Vec::new();
        while first.is_empty() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
            first = feed.take();
        }
        assert_eq!(first, b"abc");
        while !feed.eof() {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(feed.take().is_empty());
    }
}
