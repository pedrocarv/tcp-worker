// Load-test client: opens N connections to the job server, holds them
// all open, sends one job on each, and counts how many get answered.
//
// Usage (with the server already running):
//   cargo run --example flood -- <N> [delay_us]
// delay_us is an optional pause between connection attempts, in
// microseconds, to slow the arrival rate (default 0: as fast as possible).

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let n: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(100);
    let delay = Duration::from_micros(args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0));

    // Open the connections and keep every one of them open.
    let mut conns = Vec::new();
    for i in 0..n {
        if i > 0 && (i % 1000) == 0 {
            println!("{i} connections open");
        }
        sleep(delay);
        match TcpStream::connect("127.0.0.1:7878") {
            Ok(s) => conns.push(s),
            Err(e) => {
                eprintln!("connection {i} failed: {e}");
                break;
            }
        }
    }
    println!("opened {} connections", conns.len());

    // Send a job on every connection, give the server a moment, then
    // collect replies. A connection that isn't being served times out.
    for s in &mut conns {
        let _ = writeln!(s, "sum 1 2");
    }
    sleep(Duration::from_secs(1));

    let (mut served, mut busy) = (0, 0);
    for s in &conns {
        let _ = s.set_read_timeout(Some(Duration::from_millis(50)));
        let mut reply = String::new();
        if BufReader::new(s).read_line(&mut reply).is_ok() {
            match reply.trim() {
                "3" => served += 1,
                "error: server busy" => busy += 1,
                _ => {}
            }
        }
    }
    let total = conns.len();
    println!("{served} of {total} connections were served");
    println!("{busy} were told the server is busy");
    println!("{} got no reply", total - served - busy);

    println!("holding the connections open; press Enter to close them");
    let _ = std::io::stdin().read_line(&mut String::new());
}
