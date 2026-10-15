// Stage 1a: a single-threaded echo server.
// Every line a client sends is written straight back to it.

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
        clone.write_all(line.as_bytes())?;
        line.clear();
    } 
    Ok(())
}
