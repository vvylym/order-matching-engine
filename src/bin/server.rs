//! Tokio benchmark harness server.
//!
//! Accepts a typed line protocol and routes commands to instrument-local engines.

use std::collections::HashMap;
use std::sync::Arc;

use clap::{Parser, ValueEnum};
use omer::book::service::BTreeOrderBook;
use omer::distributed_wire::{RoutedCommand, WireParseError, parse_frame};
use omer::engine::{OrderCommand, OrderMatchingService, builder};
use omer::events::NoOpEventSink;
use omer::matching::PriceCrossMatchingPolicy;
use omer::self_trade::AllowAllSelfTradePolicy;
use omer::sequence::CounterSequenceGenerator;
use omer::store::service::HashMapOrderStore;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{RwLock, mpsc};

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, default_value = "127.0.0.1:7001")]
    bind: String,
    #[arg(long, default_value_t = 4)]
    instruments: usize,
    #[arg(long, value_enum, default_value_t = ChannelKind::Tokio)]
    worker_channel: ChannelKind,
}

/// How routed matcher commands are delivered in this harness.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
enum ChannelKind {
    /// Deliver through [`tokio::sync::mpsc::unbounded_channel`] receivers polled by async worker tasks.
    #[value(name = "tokio")]
    #[default]
    Tokio,
    /// Deliver through [`crossbeam_channel::unbounded`] receivers polled by dedicated [`std::thread`] workers.
    #[value(name = "crossbeam")]
    Crossbeam,
}

#[derive(Debug)]
enum WorkerCmd {
    Add {
        instrument_id: u32,
        add: omer::engine::AddOrderCommand,
    },
    CancelById {
        instrument_id: u32,
        cancel: omer::engine::CancelByOrderIdCommand,
    },
}

type RouterIndex = Arc<RwLock<HashMap<u64, usize>>>;
type DynErr = Box<dyn std::error::Error>;
type TokioWorkerTx = mpsc::UnboundedSender<WorkerCmd>;
type CrossbeamWorkerTx = crossbeam_channel::Sender<WorkerCmd>;

/// Handle for dispatching to instrument workers; clones cheaply when new clients connect.
#[derive(Clone)]
enum WorkerSenders {
    Tokio(Vec<TokioWorkerTx>),
    Crossbeam(Vec<CrossbeamWorkerTx>),
}

impl WorkerSenders {
    fn len(&self) -> usize {
        match self {
            Self::Tokio(v) => v.len(),
            Self::Crossbeam(v) => v.len(),
        }
    }

    fn send_cmd(&self, worker: usize, cmd: WorkerCmd) {
        match self {
            Self::Tokio(v) => {
                let _ = v[worker].send(cmd);
            }
            Self::Crossbeam(v) => {
                let _ = v[worker].send(cmd);
            }
        }
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), DynErr> {
    let args = Args::parse();
    let listener = TcpListener::bind(&args.bind).await?;
    let instruments = args.instruments.max(1);
    let senders = build_worker_senders(instruments, args.worker_channel);

    let router_index: RouterIndex = Arc::new(RwLock::new(HashMap::new()));
    println!(
        "server listening on {} with instruments={} worker_channel={:?}",
        args.bind, instruments, args.worker_channel
    );

    loop {
        let (stream, _) = listener.accept().await?;
        let senders = senders.clone();
        let index = Arc::clone(&router_index);
        tokio::spawn(async move {
            if let Err(err) = handle_client(stream, senders, index).await {
                eprintln!("client error: {err}");
            }
        });
    }
}

async fn handle_client(
    stream: TcpStream,
    senders: WorkerSenders,
    index: RouterIndex,
) -> Result<(), DynErr> {
    let (read_half, mut write_half) = stream.into_split();
    let mut lines = BufReader::new(read_half).lines();

    while let Some(line) = lines.next_line().await? {
        write_response(
            &mut write_half,
            route_and_dispatch(&line, &senders, &index).await,
        )
        .await?;
    }
    Ok(())
}

async fn write_response(
    writer: &mut tokio::net::tcp::OwnedWriteHalf,
    result: Result<RouteResult, WireParseError>,
) -> Result<(), DynErr> {
    match result {
        Ok(RouteResult::Ok) => writer.write_all(b"OK\n").await?,
        Ok(RouteResult::UnknownOrder) => {
            writer.write_all(b"ERR unknown_order\n").await?
        }
        Err(_) => writer.write_all(b"ERR parse\n").await?,
    }
    Ok(())
}

enum RouteResult {
    Ok,
    UnknownOrder,
}

async fn route_and_dispatch(
    line: &str,
    senders: &WorkerSenders,
    index: &RouterIndex,
) -> Result<RouteResult, WireParseError> {
    let instruments = senders.len();
    let frame = parse_frame(line)?;
    for cmd in frame.commands {
        let route =
            dispatch_command(cmd.into_routed(), instruments, senders, index)
                .await?;
        if matches!(route, RouteResult::UnknownOrder) {
            return Ok(RouteResult::UnknownOrder);
        }
    }
    Ok(RouteResult::Ok)
}

fn build_worker_senders(instruments: usize, kind: ChannelKind) -> WorkerSenders {
    match kind {
        ChannelKind::Tokio => {
            WorkerSenders::Tokio(spawn_tokio_workers(instruments))
        }
        ChannelKind::Crossbeam => {
            WorkerSenders::Crossbeam(spawn_crossbeam_workers(instruments))
        }
    }
}

fn spawn_tokio_workers(instruments: usize) -> Vec<TokioWorkerTx> {
    let mut senders = Vec::with_capacity(instruments);
    for worker_idx in 0..instruments {
        let worker_instrument = (worker_idx + 1) as u32;
        let (tx, mut rx) = mpsc::unbounded_channel::<WorkerCmd>();
        senders.push(tx);
        tokio::spawn(async move {
            let mut engine = matcher_engine();
            while let Some(cmd) = rx.recv().await {
                apply_worker_command(worker_instrument, &mut engine, cmd);
            }
            eprintln!("tokio worker {worker_instrument} exited");
        });
    }
    senders
}

fn spawn_crossbeam_workers(instruments: usize) -> Vec<CrossbeamWorkerTx> {
    let mut senders = Vec::with_capacity(instruments);
    for worker_idx in 0..instruments {
        let worker_instrument = (worker_idx + 1) as u32;
        let (tx, rx) = crossbeam_channel::unbounded::<WorkerCmd>();
        senders.push(tx);
        std::thread::Builder::new()
            .name(format!("omer-matcher-{worker_instrument}"))
            .spawn(move || {
                let mut engine = matcher_engine();
                while let Ok(cmd) = rx.recv() {
                    apply_worker_command(worker_instrument, &mut engine, cmd);
                }
                eprintln!("crossbeam worker {worker_instrument} exited");
            })
            .expect("spawn crossbeam matcher thread");
    }
    senders
}

fn matcher_engine() -> impl OrderMatchingService {
    builder()
        .with_sequence_generator(CounterSequenceGenerator::new())
        .with_price_book(BTreeOrderBook::new())
        .with_order_store(HashMapOrderStore::new())
        .with_matching_policy(PriceCrossMatchingPolicy)
        .with_self_trade_policy(AllowAllSelfTradePolicy)
        .with_event_sink(NoOpEventSink)
        .build()
}

fn apply_worker_command(
    worker_instrument: u32,
    engine: &mut impl OrderMatchingService,
    cmd: WorkerCmd,
) {
    match cmd {
        WorkerCmd::Add { instrument_id, add }
            if add.symbol_id == worker_instrument
                && instrument_id == worker_instrument =>
        {
            let _ = engine.process(OrderCommand::Add(add));
        }
        WorkerCmd::CancelById {
            instrument_id,
            cancel,
        } if instrument_id == worker_instrument => {
            let _ = engine.process(OrderCommand::CancelByOrderId(cancel));
        }
        _ => {}
    }
}

async fn dispatch_command(
    cmd: RoutedCommand,
    instruments: usize,
    senders: &WorkerSenders,
    index: &RouterIndex,
) -> Result<RouteResult, WireParseError> {
    match cmd {
        RoutedCommand::Add { instrument_id, add } => {
            let worker = route_worker(instrument_id, instruments)?;
            index.write().await.insert(add.id, worker);
            senders.send_cmd(worker, WorkerCmd::Add { instrument_id, add });
            Ok(RouteResult::Ok)
        }
        RoutedCommand::CancelById {
            instrument_id,
            cancel,
        } => {
            dispatch_cancel(instrument_id, cancel, instruments, senders, index)
                .await
        }
    }
}

async fn dispatch_cancel(
    instrument_id: u32,
    cancel: omer::engine::CancelByOrderIdCommand,
    instruments: usize,
    senders: &WorkerSenders,
    index: &RouterIndex,
) -> Result<RouteResult, WireParseError> {
    let expected_worker = route_worker(instrument_id, instruments)?;
    let route = index.write().await.remove(&cancel.order_id);
    let Some(worker) = route else {
        return Ok(RouteResult::UnknownOrder);
    };
    if worker != expected_worker {
        return Ok(RouteResult::UnknownOrder);
    }
    senders.send_cmd(
        worker,
        WorkerCmd::CancelById {
            instrument_id,
            cancel,
        },
    );
    Ok(RouteResult::Ok)
}

fn route_worker(
    instrument_id: u32,
    instruments: usize,
) -> Result<usize, WireParseError> {
    if instrument_id == 0 || (instrument_id as usize) > instruments {
        return Err(WireParseError::InvalidInstrument);
    }
    Ok((instrument_id - 1) as usize)
}
