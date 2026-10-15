// Stage 2: a multi-threaded job server.
// Each line a client sends is a job such as "sum 1 2 3"; the server
// replies with one line: the result ("6") or "error: <message>".

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:7878")?;

    for connection in listener.incoming() {
        match connection {
            Ok(stream) => {
                println!("client connected: {:?}", stream.peer_addr());
                thread::spawn(move || { 
                    if let Err(e) = handle_client(stream) {
                        eprintln!("client error: {e}");
                    }
                });
            }
            Err(e) => {
                eprintln!("failed to accept connection: {e}");
            }
        }
    }
    Ok(())
}

/// Serve one client until it disconnects.
fn handle_client(stream: TcpStream) -> std::io::Result<()> {
    let mut clone = stream.try_clone()?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();

    while reader.read_line(&mut line)? > 0 {
        match run_job(&line) {
            Ok(n) => writeln!(clone, "{n}")?,
            Err(e) => writeln!(clone, "error: {e}")?,
        }
        line.clear();
    }
    Ok(())
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