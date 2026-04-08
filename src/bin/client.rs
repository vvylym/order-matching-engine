//! Tokio benchmark harness client.
//!
//! Connects to `server` binary and drives a mixed add/cancel/market workload.
//!
//! Stop condition:
//! - If `--target-ok-ops N` is set, runs until the **sum of successful command slots**
//!   (`ok_ops`) reaches at least `N`, then reports **measured** wall time (not a fixed duration).
//! - Otherwise stops after `--duration-secs` (time-bounded run).

use std::collections::VecDeque;
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use clap::Parser;
use omer::distributed_wire::{
    WireCommand, WireCommandBuffer, WireFrame, encode_frame,
};
use omer::types::Side;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

type DynErr = Box<dyn std::error::Error>;
type TaskErr = Box<dyn std::error::Error + Send + Sync>;
type Metrics = Arc<AtomicU64>;
type OpenOrders = Vec<VecDeque<u64>>;
type CancelTarget = (u32, u64);
type CancelPick = Option<CancelTarget>;

#[derive(Clone)]
struct WorkerConfig {
    addr: String,
    instruments: u32,
    batch_size: usize,
    deadline: Option<Instant>,
    target_ok_ops: Option<u64>,
    random: bool,
    rng_seed: u64,
}

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, default_value = "127.0.0.1:7001")]
    addr: String,
    #[arg(long, default_value_t = 4)]
    connections: usize,
    #[arg(long, alias = "symbols", default_value_t = 32)]
    instruments: u32,
    #[arg(long, default_value_t = 8)]
    batch_size: usize,
    /// When set, ignore wall-clock duration and stop once total `ok_ops` (across all
    /// connections) reaches at least this value. Reports measured elapsed seconds.
    #[arg(long)]
    target_ok_ops: Option<u64>,
    /// Used only when `--target-ok-ops` is **not** set (time-bounded mode).
    #[arg(long, default_value_t = 10)]
    duration_secs: u64,
    /// Pick command type, instrument, price band, and quantity with RNG (still valid for wire).
    #[arg(long, default_value_t = false)]
    random: bool,
    /// RNG seed for `--random` (default: entropy from the OS).
    #[arg(long)]
    seed: Option<u64>,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), DynErr> {
    let args = Args::parse();
    let deadline = if args.target_ok_ops.is_none() {
        Some(Instant::now() + Duration::from_secs(args.duration_secs))
    } else {
        None
    };
    let total_ops = Arc::new(AtomicU64::new(0));
    let total_err = Arc::new(AtomicU64::new(0));
    let total_lat_nanos = Arc::new(AtomicU64::new(0));
    let target_ops = args.target_ok_ops;

    let wall_start = Instant::now();
    let base_seed = args.seed.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0xFACEFEED)
    });

    let mut tasks = Vec::with_capacity(args.connections);
    for worker_id in 0..args.connections {
        let seed = base_seed ^ (worker_id as u64).wrapping_shl(32);
        let config = WorkerConfig {
            addr: args.addr.clone(),
            instruments: args.instruments.max(1),
            batch_size: args.batch_size.max(1),
            deadline,
            target_ok_ops: target_ops,
            random: args.random,
            rng_seed: seed,
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

    let wall_elapsed = wall_start.elapsed();
    let wall_secs = wall_elapsed.as_secs_f64();

    let ops = total_ops.load(Ordering::Relaxed);
    let errs = total_err.load(Ordering::Relaxed);
    let avg_lat_ns = if ops == 0 {
        0.0
    } else {
        total_lat_nanos.load(Ordering::Relaxed) as f64 / ops as f64
    };

    let mode = if target_ops.is_some() {
        "target_ok_ops"
    } else {
        "duration_secs"
    };

    println!(
        "mode={} connections={} instruments={} batch_size={} random={} target_ok_ops={:?} duration_secs={} wall_time_s={:.6} ok_ops={} err_ops={} throughput_ok_ops_s={:.2} avg_latency_ns={:.2}",
        mode,
        args.connections,
        args.instruments,
        args.batch_size,
        args.random,
        target_ops,
        args.duration_secs,
        wall_secs,
        ops,
        errs,
        ops as f64 / wall_secs,
        avg_lat_ns
    );
    let _ = std::io::stdout().lock().flush();

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

    let mut order_id = (worker_id as u64)
        .saturating_mul(1_000_000_000)
        .saturating_add(1);
    let mut cancel_id: Option<u64> = None;
    let mut i = 0_u64;

    let mut rng = SmallRng::seed_from_u64(
        config.rng_seed ^ (worker_id as u64).wrapping_shl(48),
    );
    let instrument_count = config.instruments as usize;
    let mut open_by_instrument: OpenOrders =
        (0..instrument_count).map(|_| VecDeque::new()).collect();

    loop {
        if should_stop(&config, &ok_ops) {
            break;
        }

        let frame = next_frame(
            &config,
            &mut rng,
            &mut order_id,
            &mut open_by_instrument,
            &mut cancel_id,
            &mut i,
            worker_id,
        );

        let frame_len = frame.commands.len() as u64;
        let line = encode_frame(&frame)?;
        let t0 = Instant::now();
        write_half.write_all(line.as_bytes()).await?;
        let resp = lines.next_line().await?;
        if apply_response(
            resp.as_deref(),
            frame_len,
            &config,
            &ok_ops,
            &err_ops,
            &total_lat_nanos,
            t0,
        ) {
            break;
        }
    }
    Ok(())
}

fn should_stop(config: &WorkerConfig, ok_ops: &Metrics) -> bool {
    if let Some(dl) = config.deadline
        && Instant::now() >= dl
    {
        return true;
    }
    if let Some(target) = config.target_ok_ops
        && ok_ops.load(Ordering::Relaxed) >= target
    {
        return true;
    }
    false
}

fn next_frame(
    config: &WorkerConfig,
    rng: &mut SmallRng,
    order_id: &mut u64,
    open_by_instrument: &mut OpenOrders,
    cancel_id: &mut Option<u64>,
    i: &mut u64,
    worker_id: usize,
) -> WireFrame {
    if config.random {
        return workload_frame_random(
            rng,
            order_id,
            open_by_instrument,
            config.instruments,
            config.batch_size,
            worker_id,
        );
    }
    let frame = workload_frame_deterministic(
        *i,
        *order_id,
        config.instruments,
        config.batch_size,
        cancel_id,
    );
    *order_id = order_id.wrapping_add(frame.commands.len() as u64);
    *i = i.wrapping_add(frame.commands.len() as u64);
    frame
}

fn apply_response(
    resp: Option<&str>,
    frame_len: u64,
    config: &WorkerConfig,
    ok_ops: &Metrics,
    err_ops: &Metrics,
    total_lat_nanos: &Metrics,
    t0: Instant,
) -> bool {
    match resp {
        Some(r) if r.starts_with("OK") => {
            let prev = ok_ops.fetch_add(frame_len, Ordering::Relaxed);
            total_lat_nanos
                .fetch_add(t0.elapsed().as_nanos() as u64, Ordering::Relaxed);
            if let Some(target) = config.target_ok_ops
                && prev.saturating_add(frame_len) >= target
            {
                return true;
            }
        }
        Some(_) | None => {
            err_ops.fetch_add(frame_len, Ordering::Relaxed);
        }
    }
    false
}

fn workload_frame_deterministic(
    i: u64,
    order_id: u64,
    instruments: u32,
    batch_size: usize,
    cancel_id: &mut Option<u64>,
) -> WireFrame {
    let mut commands = WireCommandBuffer::new();
    commands.reserve(batch_size);
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

fn workload_frame_random(
    rng: &mut SmallRng,
    next_id: &mut u64,
    open_by_instrument: &mut OpenOrders,
    instruments: u32,
    batch_size: usize,
    worker_id: usize,
) -> WireFrame {
    let mut commands = WireCommandBuffer::new();
    commands.reserve(batch_size);
    let ins = instruments.max(1);

    for _ in 0..batch_size {
        let roll: u8 = rng.gen_range(0..100);
        let instrument_id = rng.gen_range(1..=ins);
        let idx = (instrument_id - 1) as usize;

        let cmd = if roll < 52 {
            // Limit add (resting); track for later cancels
            let id = bump_next_id(next_id, worker_id);
            let side = if rng.gen_bool(0.52) {
                Side::Buy
            } else {
                Side::Sell
            };
            let price = rng.gen_range(50_i64..=320);
            let quantity = rng.gen_range(1_i64..=32);
            open_by_instrument[idx].push_back(id);
            WireCommand::Add {
                id,
                participant_id: rng.gen_range(1..=999_u64),
                instrument_id,
                side,
                price,
                quantity,
            }
        } else if roll < 68 {
            // Cancel by id if we have any resting id on some instrument
            if let Some((c_inst, oid)) =
                pick_cancel_target(rng, open_by_instrument)
            {
                WireCommand::CancelById {
                    order_id: oid,
                    instrument_id: c_inst,
                }
            } else {
                let id = bump_next_id(next_id, worker_id);
                let side = if rng.gen_bool(0.5) {
                    Side::Buy
                } else {
                    Side::Sell
                };
                let price = rng.gen_range(50_i64..=320);
                let quantity = rng.gen_range(1_i64..=16);
                open_by_instrument[idx].push_back(id);
                WireCommand::Add {
                    id,
                    participant_id: rng.gen_range(1..=999_u64),
                    instrument_id,
                    side,
                    price,
                    quantity,
                }
            }
        } else if roll < 84 {
            // Market order (may not rest — do not track for cancel)
            let id = bump_next_id(next_id, worker_id);
            WireCommand::Market {
                id,
                participant_id: rng.gen_range(1..=999_u64),
                instrument_id,
                side: if rng.gen_bool(0.5) {
                    Side::Buy
                } else {
                    Side::Sell
                },
                quantity: rng.gen_range(1_i64..=24),
            }
        } else {
            // Another limit leg for depth
            let id = bump_next_id(next_id, worker_id);
            open_by_instrument[idx].push_back(id);
            WireCommand::Add {
                id,
                participant_id: rng.gen_range(1..=999_u64),
                instrument_id,
                side: Side::Sell,
                price: rng.gen_range(180_i64..=400),
                quantity: rng.gen_range(1_i64..=20),
            }
        };
        commands.push(cmd);
    }
    WireFrame { commands }
}

fn bump_next_id(next_id: &mut u64, worker_id: usize) -> u64 {
    let id = *next_id;
    *next_id = next_id.wrapping_add(1);
    if id == 0 {
        *next_id =
            ((worker_id as u64).saturating_mul(1_000_000_000)).saturating_add(2);
    }
    id
}

fn pick_cancel_target(
    rng: &mut SmallRng,
    open_by_instrument: &mut OpenOrders,
) -> CancelPick {
    let indices: Vec<usize> = open_by_instrument
        .iter()
        .enumerate()
        .filter_map(|(ix, q)| if q.is_empty() { None } else { Some(ix) })
        .collect();
    if indices.is_empty() {
        return None;
    }
    let qi = indices[rng.gen_range(0..indices.len())];
    let pos = rng.gen_range(0..open_by_instrument[qi].len());
    let oid = open_by_instrument[qi]
        .remove(pos)
        .expect("index chosen within deque length");
    Some(((qi + 1) as u32, oid))
}
