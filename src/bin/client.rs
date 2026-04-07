//! Tokio benchmark harness client.
//!
//! Connects to `server` binary and drives a mixed add/cancel/market workload.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use clap::Parser;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

type DynErr = Box<dyn std::error::Error>;
type TaskErr = Box<dyn std::error::Error + Send + Sync>;
type Metrics = Arc<AtomicU64>;

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, default_value = "127.0.0.1:7001")]
    addr: String,
    #[arg(long, default_value_t = 4)]
    connections: usize,
    #[arg(long, default_value_t = 1)]
    symbols: u32,
    #[arg(long, default_value_t = 10)]
    duration_secs: u64,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), DynErr> {
    let args = Args::parse();
    let deadline = Instant::now() + Duration::from_secs(args.duration_secs);
    let total_ops = Arc::new(AtomicU64::new(0));
    let total_err = Arc::new(AtomicU64::new(0));
    let total_lat_nanos = Arc::new(AtomicU64::new(0));

    let mut tasks = Vec::with_capacity(args.connections);
    for worker_id in 0..args.connections {
        let addr = args.addr.clone();
        let ops = Arc::clone(&total_ops);
        let err = Arc::clone(&total_err);
        let lat = Arc::clone(&total_lat_nanos);
        let symbols = args.symbols.max(1);

        tasks.push(tokio::spawn(run_worker(
            worker_id, addr, symbols, deadline, ops, err, lat,
        )));
    }

    for t in tasks {
        if let Err(join_err) = t.await {
            eprintln!("worker task join error: {join_err}");
        }
    }

    let elapsed = args.duration_secs as f64;
    let ops = total_ops.load(Ordering::Relaxed);
    let errs = total_err.load(Ordering::Relaxed);
    let avg_lat_ns = if ops == 0 {
        0.0
    } else {
        total_lat_nanos.load(Ordering::Relaxed) as f64 / ops as f64
    };

    println!(
        "connections={} duration_s={} ok_ops={} err_ops={} throughput_ops_s={:.2} avg_latency_ns={:.2}",
        args.connections,
        args.duration_secs,
        ops,
        errs,
        ops as f64 / elapsed,
        avg_lat_ns
    );

    Ok(())
}

async fn run_worker(
    worker_id: usize,
    addr: String,
    symbols: u32,
    deadline: Instant,
    ok_ops: Metrics,
    err_ops: Metrics,
    total_lat_nanos: Metrics,
) -> Result<(), TaskErr> {
    let stream = TcpStream::connect(&addr).await?;
    let (read_half, mut write_half) = stream.into_split();
    let mut lines = BufReader::new(read_half).lines();

    let mut order_id = (worker_id as u64) * 1_000_000_000 + 1;
    let mut cancel_id: Option<u64> = None;
    let mut i = 0_u64;
    while Instant::now() < deadline {
        let line = workload_line(i, order_id, symbols, &mut cancel_id);
        let t0 = Instant::now();
        write_half.write_all(line.as_bytes()).await?;
        match lines.next_line().await? {
            Some(resp) if resp.starts_with("OK") => {
                ok_ops.fetch_add(1, Ordering::Relaxed);
                total_lat_nanos
                    .fetch_add(t0.elapsed().as_nanos() as u64, Ordering::Relaxed);
            }
            Some(_) | None => {
                err_ops.fetch_add(1, Ordering::Relaxed);
            }
        }
        order_id = order_id.wrapping_add(1);
        i = i.wrapping_add(1);
    }
    Ok(())
}

fn workload_line(
    i: u64,
    order_id: u64,
    symbols: u32,
    cancel_id: &mut Option<u64>,
) -> String {
    let symbol = (order_id as u32 % symbols) + 1;
    match i % 10 {
        0..=5 => {
            *cancel_id = Some(order_id);
            format!(
                "ADD {order_id} 100 {symbol} B {} 1\n",
                100 + (order_id as i64 % 50)
            )
        }
        6 | 7 => {
            let oid = cancel_id.unwrap_or(order_id);
            format!("CANCELID {oid}\n")
        }
        8 => format!("ADD {order_id} 101 {symbol} S 250 5\n"),
        _ => format!("MARKET {order_id} 102 {symbol} B 5\n"),
    }
}
