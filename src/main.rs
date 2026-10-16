// Stage 4: a job server with a fixed pool of worker threads that sheds
// load when busy and drops idle clients.
// Each line a client sends is a job such as "sum 1 2 3"; the server
// replies with one line: the result ("6") or "error: <message>".

use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{self, Receiver, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Number of worker threads: the most clients served at the same time.
const WORKERS: usize = 4;
/// Most accepted connections allowed to wait for a free worker.
const QUEUE_SIZE: usize = 16;
/// How long a client may stay silent before the server disconnects it.
const IDLE_TIMEOUT: Duration = Duration::from_secs(10);

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:7878")?;

    // Accepted connections wait here for a free worker. The channel is
    // bounded; when it is full, new clients are turned away (see below).
    let (sender, receiver) = mpsc::sync_channel::<TcpStream>(QUEUE_SIZE);
    let receiver = Arc::new(Mutex::new(receiver));

    for id in 0..WORKERS {
        let receiver = Arc::clone(&receiver);
        thread::spawn(move || worker(id, receiver));
    }

    for connection in listener.incoming() {
        match connection {
            Ok(stream) => {
                println!("client connected: {:?}", stream.peer_addr());
                // Shed load: if the queue is full, tell the client and close
                // the connection instead of making the accept loop wait.
                match sender.try_send(stream) {
                    Ok(()) => {}
                    Err(TrySendError::Full(mut stream)) => {
                        let _ = writeln!(stream, "error: server busy");
                    }
                    Err(TrySendError::Disconnected(_)) => {
                        eprintln!("all workers have exited; shutting down");
                        break;
                    }
                }
            }
            Err(e) => {
                eprintln!("failed to accept connection: {e}");
            }
        }
    }
    Ok(())
}

/// One worker thread: take the next waiting connection, serve that client
/// until it disconnects, then go back for another.
fn worker(id: usize, receiver: Arc<Mutex<Receiver<TcpStream>>>) {
    loop {
        // The lock guard is a temporary, dropped at the end of this
        // statement, so the lock is released before the client is served.
        // (`while let Ok(stream) = receiver.lock()...` would hold it for the
        // whole loop body and let only one worker run at a time.)
        let message = receiver.lock().unwrap().recv();
        match message {
            Ok(stream) => {
                if let Err(e) = handle_client(stream) {
                    eprintln!("worker {id}: client error: {e}");
                }
            }
            Err(_) => break, // channel closed: no more connections will come
        }
    }
}

/// Serve one client until it disconnects.
fn handle_client(stream: TcpStream) -> std::io::Result<()> {
    // A read that waits longer than IDLE_TIMEOUT fails instead of blocking
    // forever, so a silent client can't hold a worker indefinitely.
    stream.set_read_timeout(Some(IDLE_TIMEOUT))?;
    let mut clone = stream.try_clone()?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();

    loop {
        match reader.read_line(&mut line) {
            // The client closed the connection.
            Ok(0) => return Ok(()),
            // A full line arrived: run the job and reply.
            Ok(_) => {
                match run_job(&line) {
                    Ok(n) => writeln!(clone, "{n}")?,
                    Err(e) => writeln!(clone, "error: {e}")?,
                }
                line.clear();
            }
            // The client was silent for IDLE_TIMEOUT. A timed-out read is
            // WouldBlock on macOS and Linux, TimedOut on Windows.
            Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
                let _ = writeln!(clone, "error: idle timeout");
                return Ok(());
            }
            // Any other error is a real failure: report it.
            Err(e) => return Err(e),
        }
    }
}

/// Run one job and return its result.
///
/// A job is a command followed by whole numbers, separated by whitespace:
///   "sum 1 2 3" -> Ok(6)
///   "max 4 9 2" -> Ok(9)
/// Anything else is an Err with a message for the client.
fn run_job(line: &str) -> Result<i64, String> {
    let mut words = line.split_whitespace();
    let command = match words.next() {          // first word, if there is one
        Some(w) => w,
        None => return Err("empty line".to_string()),
    };

    let mut nums = Vec::new();
    for w in words {                          // the rest of the words
        match w.parse::<i64>() {
            Ok(n) => nums.push(n),
            Err(_) => return Err(format!("not a number: {w}")),
        }
    }

    match command {
        "sum" => {
                let mut total: i64 = 0;
                for n in &nums {
                    total = match total.checked_add(*n) {
                        Some(t) => t,
                        None => return Err("The sum would lead to integer overflow.".to_string()),
                    };
                }
                Ok(total)
            },
        "max"  => match nums.iter().max(){
            Some(m) => Ok(*m),
            None => return Err("max needs at least one number".to_string()),
        },
        _ => Err(format!("Unknown command: {command}")),
    }
}

// The spec for run_job, as tests. Run them with `cargo test`.
#[cfg(test)]
mod tests {
    use super::run_job;

    #[test]
    fn sum_adds_numbers() {
        assert_eq!(run_job("sum 1 2 3"), Ok(6));
    }

    #[test]
    fn sum_of_nothing_is_zero() {
        assert_eq!(run_job("sum"), Ok(0));
    }

    #[test]
    fn max_finds_largest() {
        assert_eq!(run_job("max 4 9 2"), Ok(9));
    }

    #[test]
    fn negative_numbers_work() {
        assert_eq!(run_job("max -5 -2 -9"), Ok(-2));
    }

    #[test]
    fn extra_whitespace_and_newline_are_ignored() {
        assert_eq!(run_job("  sum   1  2 \n"), Ok(3));
    }

    #[test]
    fn max_of_nothing_is_an_error() {
        assert!(run_job("max").is_err());
    }

    #[test]
    fn unknown_command_is_an_error() {
        assert!(run_job("mul 2 3").is_err());
    }

    #[test]
    fn non_number_is_an_error() {
        assert!(run_job("sum 1 two 3").is_err());
    }

    #[test]
    fn empty_line_is_an_error() {
        assert!(run_job("\n").is_err());
    }

    #[test]
    fn sum_of_large_numbers_is_an_error() {
        assert!(run_job("sum 9223372036854775807 1").is_err());
    }
}