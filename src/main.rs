// Stage 1b: a single-threaded job server.
// Each line a client sends is a job such as "sum 1 2 3"; the server
// replies with one line: the result ("6") or "error: <message>".

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};

fn main() -> std::io::Result<()> {
    // TODO 1: Bind a TcpListener to "127.0.0.1:7878".
    //         End the call with `?` so an error (e.g. port already in use)
    //         ends the program with a message instead of being ignored.
    let listener = TcpListener::bind("127.0.0.1:7878")?;
    // TODO 2: Loop over the listener's incoming connections.
    //         Each item is a std::io::Result<TcpStream>.
    //         For each Ok(stream): print the client's address (peer_addr),
    //         then call handle_client(stream).
    //         For each Err(e): print it and keep going.
    for connection in listener.incoming() {
        match connection {
            Ok(stream) => {
                println!("client connected: {:?}", stream.peer_addr());
                if let Err(e) = handle_client(stream) {
                    eprintln!("client error: {e}");
                }
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
    // TODO 3: Make a BufReader so you can read the socket one line at a time.
    //         Hint: stream.try_clone() gives you a second handle to the same
    //         socket: one for reading (inside the BufReader), one for writing.
    let mut clone = stream.try_clone()?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
   
    // TODO 4: Loop:
    //           - read one line into a String with read_line
    //           - if read_line returns Ok(0), the client closed the
    //             connection: return Ok(())
    //           - otherwise send the line back with write_all
    //         Remember to clear the String between lines.
    while reader.read_line(&mut line)? > 0 {
        // TODO 8: Instead of echoing, call run_job(&line) and reply with
        //         one line: the number if it worked, "error: <message>" if not.
        //         Hint: writeln!(clone, "...")? writes a formatted line.
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
    // TODO 5: Split the line into words with split_whitespace().
    //         The first word is the command; if there is none, return an Err.
    let mut words = line.split_whitespace();
    let command = match words.next() {          // first word, if there is one
        Some(w) => w,
        None => return Err("empty line".to_string()),
    };
    // TODO 6: Parse each remaining word as an i64 and collect them in a Vec<i64>.
    //         If a word isn't a number, return an Err that names the word.
    let mut nums = Vec::new();
    for w in words {                          // the rest of the words
        match w.parse::<i64>() {
            Ok(n) => nums.push(n),
            Err(_) => return Err(format!("not a number: {w}")),
        }
    }
    // TODO 7: match on the command:
    //           "sum" -> the sum of the numbers (the sum of no numbers is 0)
    //           "max" -> the largest number (an Err if there are none)
    //           anything else -> an Err that names the unknown command

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