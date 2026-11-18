//! Background reader: a thread owns the pipe and the UI drains whatever has
//! arrived on each frame tick. No extra event-loop wakeup source, so an idle
//! panel stays idle — the price is one frame of latency on the first delta of
//! an answer, which is a 16 ms tick.

use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;

/// How many bytes may wait for the UI before the reader stops reading. The pipe
/// then pushes back on the child: an answer the panel is not drawing, because a
/// frame is late or the reply cap is reached, cannot grow this queue without bound.
pub const FEED_CAP: usize = 1024 * 1024;

/// Bytes read so far, plus whether the reader has finished.
pub struct Feed {
    arrived: Mutex<Vec<u8>>,
    /// Wakes the reader when the UI drains or the request ends.
    space: Condvar,
    /// No more is wanted: the request is finished or cancelled.
    closed: AtomicBool,
    eof: AtomicBool,
    reader: Mutex<Option<JoinHandle<()>>>,
}

impl Feed {
    /// Read `r` on a thread until it ends. The thread dies with the process;
    /// there is nothing to join, because nothing is persisted.
    pub fn spawn(mut r: impl Read + Send + 'static) -> Arc<Feed> {
        let feed = Arc::new(Feed {
            arrived: Mutex::new(Vec::new()),
            space: Condvar::new(),
            closed: AtomicBool::new(false),
            eof: AtomicBool::new(false),
            reader: Mutex::new(None),
        });
        let writer = Arc::clone(&feed);
        let handle = std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match r.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let n = n.min(FEED_CAP);
                        let mut queued = writer.arrived.lock().unwrap();
                        while queued.len() + n > FEED_CAP && !writer.closed.load(Ordering::Acquire)
                        {
                            queued = writer.space.wait(queued).unwrap();
                        }
                        // Closed with the queue full: nobody will drain it, and
                        // waiting for room would keep the child from exiting.
                        if writer.closed.load(Ordering::Acquire) && queued.len() + n > FEED_CAP {
                            break;
                        }
                        queued.extend_from_slice(&buf[..n]);
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(_) => break,
                }
            }
            writer.eof.store(true, Ordering::Release);
        });
        *feed.reader.lock().unwrap() = Some(handle);
        feed
    }

    /// Take everything that has arrived since the last call, in order.
    pub fn take(&self) -> Vec<u8> {
        let mut queued = self.arrived.lock().unwrap();
        let taken = std::mem::take(&mut *queued);
        if !taken.is_empty() {
            self.space.notify_all();
        }
        taken
    }

    /// Tell the reader no more is wanted and wake it if it is waiting for room.
    pub fn close(&self) {
        self.closed.store(true, Ordering::Release);
        self.space.notify_all();
    }

    /// Wait for the reader to stop. Only after `close` and after the child is
    /// reaped: the reader can be in a blocking read until its pipe reaches EOF.
    pub fn join(&self) {
        if let Some(handle) = self.reader.lock().unwrap().take() {
            let _ = handle.join();
        }
    }

    /// True once the reader has hit end of file (or failed). The panel does not
    /// need it — the child exiting is what ends a request — but the reader's
    /// contract is worth asserting.
    #[cfg(test)]
    pub fn eof(&self) -> bool {
        self.eof.load(Ordering::Acquire)
    }

    #[cfg(test)]
    fn queued(&self) -> usize {
        self.arrived.lock().unwrap().len()
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

    /// A source with no end: the reader must stop at the cap rather than buffer
    /// the whole stream, and `close` must be able to end it.
    struct Flood;

    impl Read for Flood {
        fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
            out.fill(b'x');
            Ok(out.len())
        }
    }

    #[test]
    fn the_queue_is_bounded_and_close_ends_the_reader() {
        let feed = Feed::spawn(Flood);
        let deadline = Instant::now() + Duration::from_secs(5);
        while feed.queued() < FEED_CAP && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(feed.queued(), FEED_CAP, "the reader stops at the cap");
        std::thread::sleep(Duration::from_millis(20));
        assert_eq!(feed.queued(), FEED_CAP, "and does not grow past it");
        assert!(!feed.take().is_empty(), "the UI can drain it");
        feed.close();
        feed.join();
        assert!(feed.eof(), "close lets the reader finish");
    }
}
