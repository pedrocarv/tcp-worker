# tcp-worker

A small TCP job server in Rust, built in four stages to study concurrency trade-offs and to measure where each design breaks.

Clients send one job per line and get one line back:

```
sum 1 2 3   →  6
max 4 9 2   →  9
sum 1 two   →  error: not a number: two
```

## Running it

Start the server on `127.0.0.1:7878`:

```bash
cargo run
```

Connect and send jobs:

```bash
nc 127.0.0.1 7878
```

Run the unit tests for the job parser:

```bash
cargo test
```

Load-test the server with N connections, optionally waiting a number of microseconds between connection attempts:

```bash
cargo run --example flood -- 10000 500
```

The flood client opens every connection, holds them all open, sends one job on each, and reports how many were served, told the server is busy, or got no reply.

## The four stages

Each stage is a commit on `main`.

| Stage | Design | What it showed |
|---|---|---|
| 1a | Single thread, echo server | One connected client blocks every other client. A second client's connection still completes, because the kernel finishes the handshake and queues it in the listen backlog. |
| 1b | Job parsing in `run_job`, separate from the socket code | Ten unit tests, including an overflow case: `sum` uses `checked_add`, so a huge input returns an error instead of panicking and taking the server down. |
| 2 | One thread per connection | An unthrottled flood overflowed the listen backlog at about 1,800–1,900 connections. Throttled to one connection every 500 µs, it reached macOS's limit of 8,192 threads per process, `thread::spawn` panicked in `main`, and every client was dropped. |
| 3 | Fixed pool of 4 workers fed by a bounded queue of 16 (`Arc<Mutex<Receiver>>`) | The server survives the same flood with 5 threads. About 200 connections were accepted and 4 served: idle clients occupy every worker. |
| 4 | 10-second idle timeout, and load shedding with `try_send` | A silent client is disconnected. When the queue is full, new clients get `error: server busy` immediately. A flood of 100 connections: 4 served, 16 queued, 80 told busy. |

The numbers were measured on a MacBook with the flood client. They depend on operating-system limits (here a listen backlog of 128 and 8,192 threads per process).

## Known limits

- The idle timeout applies to each read, so a client that trickles one byte at a time never trips it. A per-line deadline would fix that.
- Lines have no maximum length, so a client can grow a line until memory runs out.
- Each busy connection still occupies a worker thread. An event loop (`epoll` or `kqueue`) with a worker pool for CPU-heavy jobs would let idle connections cost almost nothing.
