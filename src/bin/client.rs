//! Tokio benchmark harness client.
//!
//! Connects to `server` binary and drives a mixed add/cancel/market workload.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use clap::Parser;
use omer::distributed_wire::{WireCommand, WireFrame, encode_frame};
use omer::types::Side;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

type DynErr = Box<dyn std::error::Error>;
type TaskErr = Box<dyn std::error::Error + Send + Sync>;
type Metrics = Arc<AtomicU64>;

#[derive(Clone)]
struct WorkerConfig {
    addr: String,
    instruments: u32,
    batch_size: usize,
    deadline: Instant,
}

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, default_value = "127.0.0.1:7001")]
    addr: String,
    #[arg(long, default_value_t = 4)]
    connections: usize,
    #[arg(long, alias = "symbols", default_value_t = 4)]
    instruments: u32,
    #[arg(long, default_value_t = 4)]
    batch_size: usize,
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
        let config = WorkerConfig {
            addr: args.addr.clone(),
            instruments: args.instruments.max(1),
            batch_size: args.batch_size.max(1),
            deadline,
        };
        let ops = Arc::clone(&total_ops);
        let err = Arc::clone(&total_err);
        let lat = Arc::clone(&total_lat_nanos);

        tasks.push(tokio::spawn(run_worker(worker_id, config, ops, err, lat)));
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
    config: WorkerConfig,
    ok_ops: Metrics,
    err_ops: Metrics,
    total_lat_nanos: Metrics,
) -> Result<(), TaskErr> {
    let stream = TcpStream::connect(&config.addr).await?;
    let (read_half, mut write_half) = stream.into_split();
    let mut lines = BufReader::new(read_half).lines();

    let mut order_id = (worker_id as u64) * 1_000_000_000 + 1;
    let mut cancel_id: Option<u64> = None;
    let mut i = 0_u64;
    while Instant::now() < config.deadline {
        let frame = workload_frame(
            i,
            order_id,
            config.instruments,
            config.batch_size,
            &mut cancel_id,
        );
        let line = encode_frame(&frame)?;
        let frame_len = frame.commands.len() as u64;
        let t0 = Instant::now();
        write_half.write_all(line.as_bytes()).await?;
        match lines.next_line().await? {
            Some(resp) if resp.starts_with("OK") => {
                ok_ops.fetch_add(frame_len, Ordering::Relaxed);
                total_lat_nanos
                    .fetch_add(t0.elapsed().as_nanos() as u64, Ordering::Relaxed);
            }
            Some(_) | None => {
                err_ops.fetch_add(frame_len, Ordering::Relaxed);
            }
        }
        order_id = order_id.wrapping_add(frame_len);
        i = i.wrapping_add(frame_len);
    }
    Ok(())
}

fn workload_frame(
    i: u64,
    order_id: u64,
    instruments: u32,
    batch_size: usize,
    cancel_id: &mut Option<u64>,
) -> WireFrame {
    let mut commands = Vec::with_capacity(batch_size);
    for offset in 0..batch_size {
        let current = order_id.wrapping_add(offset as u64);
        let seq = i.wrapping_add(offset as u64);
        let instrument_id = (current as u32 % instruments) + 1;
        let cmd = match seq % 10 {
            0..=5 => {
                *cancel_id = Some(current);
                WireCommand::Add {
                    id: current,
                    participant_id: 100,
                    instrument_id,
                    side: Side::Buy,
                    price: 100 + (current as i64 % 50),
                    quantity: 1,
                }
            }
            6 | 7 => {
                let oid = cancel_id.unwrap_or(current);
                WireCommand::CancelById {
                    order_id: oid,
                    instrument_id,
                }
            }
            8 => WireCommand::Add {
                id: current,
                participant_id: 101,
                instrument_id,
                side: Side::Sell,
                price: 250,
                quantity: 5,
            },
            _ => WireCommand::Market {
                id: current,
                participant_id: 102,
                instrument_id,
                side: Side::Buy,
                quantity: 5,
            },
        };
        commands.push(cmd);
    }
    WireFrame { commands }
}
