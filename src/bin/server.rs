//! Tokio benchmark harness server.
//!
//! Accepts a typed line protocol and routes commands to instrument-local engines.

use std::sync::Arc;

use clap::{Parser, ValueEnum};
use dashmap::DashMap;
use omer::book::service::{
    BTreeOrderBook, DashSkipOrderBook, PoolLevelOrderBook,
};
use omer::distributed_wire::{RoutedCommand, WireParseError, parse_frame};
use omer::engine::{
    AddOrderCommand, CancelByOrderIdCommand, CancelOrderCommand,
    ExecuteOrderCommand, OrderCommand, OrderMatchingEngine, OrderMatchingService,
    ReduceOrderCommand, ReplaceOrderByNewIdCommand, ReplaceOrderCommand, builder,
};
use omer::error::Result as EngineResult;
use omer::events::{BytesChannelEventSink, NoOpEventSink};
use omer::matching::PriceCrossMatchingPolicy;
use omer::self_trade::AllowAllSelfTradePolicy;
use omer::sequence::CounterSequenceGenerator;
use omer::store::service::{DenseOrderStore, HashMapOrderStore};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, default_value = "127.0.0.1:7001")]
    bind: String,
    #[arg(long, default_value_t = 4)]
    instruments: usize,
    #[arg(long, value_enum, default_value_t = ChannelKind::Tokio)]
    worker_channel: ChannelKind,
    #[arg(long, value_enum, default_value_t = PriceBookKind::Btree)]
    price_book: PriceBookKind,
    #[arg(long, value_enum, default_value_t = OrderStoreKind::HashMap)]
    order_store: OrderStoreKind,
    #[arg(long, value_enum, default_value_t = EventSinkKind::Noop)]
    event_sink: EventSinkKind,
    #[arg(long, default_value_t = 65_536)]
    event_channel_capacity: usize,
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

/// Price book backend used by each matcher worker.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
enum PriceBookKind {
    #[value(name = "btree")]
    #[default]
    Btree,
    #[value(name = "dash_skip")]
    DashSkip,
    #[value(name = "pool_level")]
    PoolLevel,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
enum OrderStoreKind {
    #[value(name = "hash_map")]
    #[default]
    HashMap,
    #[value(name = "dense")]
    Dense,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
enum EventSinkKind {
    #[value(name = "noop")]
    #[default]
    Noop,
    #[value(name = "bytes_channel")]
    BytesChannel,
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

type RouterIndex = Arc<DashMap<u64, usize>>;
type DynErr = Box<dyn std::error::Error>;
type TokioWorkerTx = mpsc::UnboundedSender<WorkerCmd>;
type CrossbeamWorkerTx = crossbeam_channel::Sender<WorkerCmd>;
type EventBytesTx = crossbeam_channel::Sender<Vec<u8>>;

type MatcherEngineBtreeHashNoop = OrderMatchingEngine<
    CounterSequenceGenerator,
    BTreeOrderBook,
    HashMapOrderStore,
    PriceCrossMatchingPolicy,
    AllowAllSelfTradePolicy,
    NoOpEventSink,
>;

type MatcherEngineDashSkipHashNoop = OrderMatchingEngine<
    CounterSequenceGenerator,
    DashSkipOrderBook,
    HashMapOrderStore,
    PriceCrossMatchingPolicy,
    AllowAllSelfTradePolicy,
    NoOpEventSink,
>;

type MatcherEnginePoolLevelHashNoop = OrderMatchingEngine<
    CounterSequenceGenerator,
    PoolLevelOrderBook,
    HashMapOrderStore,
    PriceCrossMatchingPolicy,
    AllowAllSelfTradePolicy,
    NoOpEventSink,
>;

type MatcherEngineBtreeHashBytes = OrderMatchingEngine<
    CounterSequenceGenerator,
    BTreeOrderBook,
    HashMapOrderStore,
    PriceCrossMatchingPolicy,
    AllowAllSelfTradePolicy,
    BytesChannelEventSink,
>;

type MatcherEngineDashSkipHashBytes = OrderMatchingEngine<
    CounterSequenceGenerator,
    DashSkipOrderBook,
    HashMapOrderStore,
    PriceCrossMatchingPolicy,
    AllowAllSelfTradePolicy,
    BytesChannelEventSink,
>;

type MatcherEnginePoolLevelHashBytes = OrderMatchingEngine<
    CounterSequenceGenerator,
    PoolLevelOrderBook,
    HashMapOrderStore,
    PriceCrossMatchingPolicy,
    AllowAllSelfTradePolicy,
    BytesChannelEventSink,
>;

type MatcherEngineBtreeDenseNoop = OrderMatchingEngine<
    CounterSequenceGenerator,
    BTreeOrderBook,
    DenseOrderStore,
    PriceCrossMatchingPolicy,
    AllowAllSelfTradePolicy,
    NoOpEventSink,
>;

type MatcherEngineDashSkipDenseNoop = OrderMatchingEngine<
    CounterSequenceGenerator,
    DashSkipOrderBook,
    DenseOrderStore,
    PriceCrossMatchingPolicy,
    AllowAllSelfTradePolicy,
    NoOpEventSink,
>;

type MatcherEnginePoolLevelDenseNoop = OrderMatchingEngine<
    CounterSequenceGenerator,
    PoolLevelOrderBook,
    DenseOrderStore,
    PriceCrossMatchingPolicy,
    AllowAllSelfTradePolicy,
    NoOpEventSink,
>;

type MatcherEngineBtreeDenseBytes = OrderMatchingEngine<
    CounterSequenceGenerator,
    BTreeOrderBook,
    DenseOrderStore,
    PriceCrossMatchingPolicy,
    AllowAllSelfTradePolicy,
    BytesChannelEventSink,
>;

type MatcherEngineDashSkipDenseBytes = OrderMatchingEngine<
    CounterSequenceGenerator,
    DashSkipOrderBook,
    DenseOrderStore,
    PriceCrossMatchingPolicy,
    AllowAllSelfTradePolicy,
    BytesChannelEventSink,
>;

type MatcherEnginePoolLevelDenseBytes = OrderMatchingEngine<
    CounterSequenceGenerator,
    PoolLevelOrderBook,
    DenseOrderStore,
    PriceCrossMatchingPolicy,
    AllowAllSelfTradePolicy,
    BytesChannelEventSink,
>;

/// Fixed set of price-book backends for the harness: avoids `dyn` dispatch (vtable) on the hot path.
///
/// Variants differ a lot in size (`DashSkipOrderBook` is large). Each worker holds exactly one
/// variant, so this is not a multi-megabyte stack allocation in practice; boxing would add pointer
/// indirection without shrinking total matcher memory.
#[allow(clippy::large_enum_variant)]
enum MatcherEngine {
    BtreeHashNoop(MatcherEngineBtreeHashNoop),
    DashSkipHashNoop(MatcherEngineDashSkipHashNoop),
    PoolLevelHashNoop(MatcherEnginePoolLevelHashNoop),
    BtreeHashBytes(MatcherEngineBtreeHashBytes),
    DashSkipHashBytes(MatcherEngineDashSkipHashBytes),
    PoolLevelHashBytes(MatcherEnginePoolLevelHashBytes),
    BtreeDenseNoop(MatcherEngineBtreeDenseNoop),
    DashSkipDenseNoop(MatcherEngineDashSkipDenseNoop),
    PoolLevelDenseNoop(MatcherEnginePoolLevelDenseNoop),
    BtreeDenseBytes(MatcherEngineBtreeDenseBytes),
    DashSkipDenseBytes(MatcherEngineDashSkipDenseBytes),
    PoolLevelDenseBytes(MatcherEnginePoolLevelDenseBytes),
}

impl OrderMatchingService for MatcherEngine {
    fn add(&mut self, cmd: AddOrderCommand) -> EngineResult<()> {
        match self {
            Self::BtreeHashNoop(e) => e.add(cmd),
            Self::DashSkipHashNoop(e) => e.add(cmd),
            Self::PoolLevelHashNoop(e) => e.add(cmd),
            Self::BtreeHashBytes(e) => e.add(cmd),
            Self::DashSkipHashBytes(e) => e.add(cmd),
            Self::PoolLevelHashBytes(e) => e.add(cmd),
            Self::BtreeDenseNoop(e) => e.add(cmd),
            Self::DashSkipDenseNoop(e) => e.add(cmd),
            Self::PoolLevelDenseNoop(e) => e.add(cmd),
            Self::BtreeDenseBytes(e) => e.add(cmd),
            Self::DashSkipDenseBytes(e) => e.add(cmd),
            Self::PoolLevelDenseBytes(e) => e.add(cmd),
        }
    }

    fn cancel(&mut self, cmd: CancelOrderCommand) -> EngineResult<()> {
        match self {
            Self::BtreeHashNoop(e) => e.cancel(cmd),
            Self::DashSkipHashNoop(e) => e.cancel(cmd),
            Self::PoolLevelHashNoop(e) => e.cancel(cmd),
            Self::BtreeHashBytes(e) => e.cancel(cmd),
            Self::DashSkipHashBytes(e) => e.cancel(cmd),
            Self::PoolLevelHashBytes(e) => e.cancel(cmd),
            Self::BtreeDenseNoop(e) => e.cancel(cmd),
            Self::DashSkipDenseNoop(e) => e.cancel(cmd),
            Self::PoolLevelDenseNoop(e) => e.cancel(cmd),
            Self::BtreeDenseBytes(e) => e.cancel(cmd),
            Self::DashSkipDenseBytes(e) => e.cancel(cmd),
            Self::PoolLevelDenseBytes(e) => e.cancel(cmd),
        }
    }

    fn replace(&mut self, cmd: ReplaceOrderCommand) -> EngineResult<()> {
        match self {
            Self::BtreeHashNoop(e) => e.replace(cmd),
            Self::DashSkipHashNoop(e) => e.replace(cmd),
            Self::PoolLevelHashNoop(e) => e.replace(cmd),
            Self::BtreeHashBytes(e) => e.replace(cmd),
            Self::DashSkipHashBytes(e) => e.replace(cmd),
            Self::PoolLevelHashBytes(e) => e.replace(cmd),
            Self::BtreeDenseNoop(e) => e.replace(cmd),
            Self::DashSkipDenseNoop(e) => e.replace(cmd),
            Self::PoolLevelDenseNoop(e) => e.replace(cmd),
            Self::BtreeDenseBytes(e) => e.replace(cmd),
            Self::DashSkipDenseBytes(e) => e.replace(cmd),
            Self::PoolLevelDenseBytes(e) => e.replace(cmd),
        }
    }

    fn cancel_by_order_id(
        &mut self,
        cmd: CancelByOrderIdCommand,
    ) -> EngineResult<()> {
        match self {
            Self::BtreeHashNoop(e) => e.cancel_by_order_id(cmd),
            Self::DashSkipHashNoop(e) => e.cancel_by_order_id(cmd),
            Self::PoolLevelHashNoop(e) => e.cancel_by_order_id(cmd),
            Self::BtreeHashBytes(e) => e.cancel_by_order_id(cmd),
            Self::DashSkipHashBytes(e) => e.cancel_by_order_id(cmd),
            Self::PoolLevelHashBytes(e) => e.cancel_by_order_id(cmd),
            Self::BtreeDenseNoop(e) => e.cancel_by_order_id(cmd),
            Self::DashSkipDenseNoop(e) => e.cancel_by_order_id(cmd),
            Self::PoolLevelDenseNoop(e) => e.cancel_by_order_id(cmd),
            Self::BtreeDenseBytes(e) => e.cancel_by_order_id(cmd),
            Self::DashSkipDenseBytes(e) => e.cancel_by_order_id(cmd),
            Self::PoolLevelDenseBytes(e) => e.cancel_by_order_id(cmd),
        }
    }

    fn reduce(&mut self, cmd: ReduceOrderCommand) -> EngineResult<()> {
        match self {
            Self::BtreeHashNoop(e) => e.reduce(cmd),
            Self::DashSkipHashNoop(e) => e.reduce(cmd),
            Self::PoolLevelHashNoop(e) => e.reduce(cmd),
            Self::BtreeHashBytes(e) => e.reduce(cmd),
            Self::DashSkipHashBytes(e) => e.reduce(cmd),
            Self::PoolLevelHashBytes(e) => e.reduce(cmd),
            Self::BtreeDenseNoop(e) => e.reduce(cmd),
            Self::DashSkipDenseNoop(e) => e.reduce(cmd),
            Self::PoolLevelDenseNoop(e) => e.reduce(cmd),
            Self::BtreeDenseBytes(e) => e.reduce(cmd),
            Self::DashSkipDenseBytes(e) => e.reduce(cmd),
            Self::PoolLevelDenseBytes(e) => e.reduce(cmd),
        }
    }

    fn execute(&mut self, cmd: ExecuteOrderCommand) -> EngineResult<()> {
        match self {
            Self::BtreeHashNoop(e) => e.execute(cmd),
            Self::DashSkipHashNoop(e) => e.execute(cmd),
            Self::PoolLevelHashNoop(e) => e.execute(cmd),
            Self::BtreeHashBytes(e) => e.execute(cmd),
            Self::DashSkipHashBytes(e) => e.execute(cmd),
            Self::PoolLevelHashBytes(e) => e.execute(cmd),
            Self::BtreeDenseNoop(e) => e.execute(cmd),
            Self::DashSkipDenseNoop(e) => e.execute(cmd),
            Self::PoolLevelDenseNoop(e) => e.execute(cmd),
            Self::BtreeDenseBytes(e) => e.execute(cmd),
            Self::DashSkipDenseBytes(e) => e.execute(cmd),
            Self::PoolLevelDenseBytes(e) => e.execute(cmd),
        }
    }

    fn replace_by_new_id(
        &mut self,
        cmd: ReplaceOrderByNewIdCommand,
    ) -> EngineResult<()> {
        match self {
            Self::BtreeHashNoop(e) => e.replace_by_new_id(cmd),
            Self::DashSkipHashNoop(e) => e.replace_by_new_id(cmd),
            Self::PoolLevelHashNoop(e) => e.replace_by_new_id(cmd),
            Self::BtreeHashBytes(e) => e.replace_by_new_id(cmd),
            Self::DashSkipHashBytes(e) => e.replace_by_new_id(cmd),
            Self::PoolLevelHashBytes(e) => e.replace_by_new_id(cmd),
            Self::BtreeDenseNoop(e) => e.replace_by_new_id(cmd),
            Self::DashSkipDenseNoop(e) => e.replace_by_new_id(cmd),
            Self::PoolLevelDenseNoop(e) => e.replace_by_new_id(cmd),
            Self::BtreeDenseBytes(e) => e.replace_by_new_id(cmd),
            Self::DashSkipDenseBytes(e) => e.replace_by_new_id(cmd),
            Self::PoolLevelDenseBytes(e) => e.replace_by_new_id(cmd),
        }
    }
}

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
    let (event_tx, event_rx) =
        crossbeam_channel::bounded::<Vec<u8>>(args.event_channel_capacity.max(1));
    if args.event_sink == EventSinkKind::BytesChannel {
        std::thread::Builder::new()
            .name("omer-events-drain".to_string())
            .spawn(move || {
                while let Ok(_bytes) = event_rx.recv() {
                    // Drain: in production, this is where publishing would happen.
                }
            })
            .expect("spawn event drain thread");
    }
    let senders = build_worker_senders(
        instruments,
        args.worker_channel,
        args.price_book,
        args.order_store,
        args.event_sink,
        event_tx,
    );

    let router_index: RouterIndex = Arc::new(DashMap::new());
    println!(
        "server listening on {} with instruments={} worker_channel={:?} price_book={:?}",
        args.bind, instruments, args.worker_channel, args.price_book
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

fn build_worker_senders(
    instruments: usize,
    kind: ChannelKind,
    price_book: PriceBookKind,
    order_store: OrderStoreKind,
    event_sink: EventSinkKind,
    event_tx: EventBytesTx,
) -> WorkerSenders {
    match kind {
        ChannelKind::Tokio => WorkerSenders::Tokio(spawn_tokio_workers(
            instruments,
            price_book,
            order_store,
            event_sink,
            event_tx,
        )),
        ChannelKind::Crossbeam => {
            WorkerSenders::Crossbeam(spawn_crossbeam_workers(
                instruments,
                price_book,
                order_store,
                event_sink,
                event_tx,
            ))
        }
    }
}

fn spawn_tokio_workers(
    instruments: usize,
    price_book: PriceBookKind,
    order_store: OrderStoreKind,
    event_sink: EventSinkKind,
    event_tx: EventBytesTx,
) -> Vec<TokioWorkerTx> {
    let mut senders = Vec::with_capacity(instruments);
    for worker_idx in 0..instruments {
        let worker_instrument = (worker_idx + 1) as u32;
        let (tx, mut rx) = mpsc::unbounded_channel::<WorkerCmd>();
        senders.push(tx);
        let event_tx = event_tx.clone();
        tokio::spawn(async move {
            let mut engine =
                matcher_engine(price_book, order_store, event_sink, &event_tx);
            while let Some(cmd) = rx.recv().await {
                apply_worker_command(worker_instrument, &mut engine, cmd);
            }
            eprintln!("tokio worker {worker_instrument} exited");
        });
    }
    senders
}

fn spawn_crossbeam_workers(
    instruments: usize,
    price_book: PriceBookKind,
    order_store: OrderStoreKind,
    event_sink: EventSinkKind,
    event_tx: EventBytesTx,
) -> Vec<CrossbeamWorkerTx> {
    let mut senders = Vec::with_capacity(instruments);
    for worker_idx in 0..instruments {
        let worker_instrument = (worker_idx + 1) as u32;
        let (tx, rx) = crossbeam_channel::unbounded::<WorkerCmd>();
        senders.push(tx);
        let event_tx = event_tx.clone();
        std::thread::Builder::new()
            .name(format!("omer-matcher-{worker_instrument}"))
            .spawn(move || {
                let mut engine = matcher_engine(
                    price_book,
                    order_store,
                    event_sink,
                    &event_tx,
                );
                while let Ok(cmd) = rx.recv() {
                    apply_worker_command(worker_instrument, &mut engine, cmd);
                }
                eprintln!("crossbeam worker {worker_instrument} exited");
            })
            .expect("spawn crossbeam matcher thread");
    }
    senders
}

fn matcher_engine(
    price_book: PriceBookKind,
    order_store: OrderStoreKind,
    event_sink: EventSinkKind,
    event_tx: &EventBytesTx,
) -> MatcherEngine {
    let sink_noop = || NoOpEventSink;
    let sink_bytes = || BytesChannelEventSink::new(event_tx.clone());

    match (price_book, order_store, event_sink) {
        (PriceBookKind::Btree, OrderStoreKind::HashMap, EventSinkKind::Noop) => {
            let e = builder()
                .with_sequence_generator(CounterSequenceGenerator::new())
                .with_price_book(BTreeOrderBook::new())
                .with_order_store(HashMapOrderStore::new())
                .with_matching_policy(PriceCrossMatchingPolicy)
                .with_self_trade_policy(AllowAllSelfTradePolicy)
                .with_event_sink(sink_noop())
                .build();
            MatcherEngine::BtreeHashNoop(e)
        }
        (
            PriceBookKind::DashSkip,
            OrderStoreKind::HashMap,
            EventSinkKind::Noop,
        ) => {
            let e = builder()
                .with_sequence_generator(CounterSequenceGenerator::new())
                .with_price_book(DashSkipOrderBook::new())
                .with_order_store(HashMapOrderStore::new())
                .with_matching_policy(PriceCrossMatchingPolicy)
                .with_self_trade_policy(AllowAllSelfTradePolicy)
                .with_event_sink(sink_noop())
                .build();
            MatcherEngine::DashSkipHashNoop(e)
        }
        (
            PriceBookKind::PoolLevel,
            OrderStoreKind::HashMap,
            EventSinkKind::Noop,
        ) => {
            let e = builder()
                .with_sequence_generator(CounterSequenceGenerator::new())
                .with_price_book(PoolLevelOrderBook::new())
                .with_order_store(HashMapOrderStore::new())
                .with_matching_policy(PriceCrossMatchingPolicy)
                .with_self_trade_policy(AllowAllSelfTradePolicy)
                .with_event_sink(sink_noop())
                .build();
            MatcherEngine::PoolLevelHashNoop(e)
        }
        (
            PriceBookKind::Btree,
            OrderStoreKind::HashMap,
            EventSinkKind::BytesChannel,
        ) => {
            let e = builder()
                .with_sequence_generator(CounterSequenceGenerator::new())
                .with_price_book(BTreeOrderBook::new())
                .with_order_store(HashMapOrderStore::new())
                .with_matching_policy(PriceCrossMatchingPolicy)
                .with_self_trade_policy(AllowAllSelfTradePolicy)
                .with_event_sink(sink_bytes())
                .build();
            MatcherEngine::BtreeHashBytes(e)
        }
        (
            PriceBookKind::DashSkip,
            OrderStoreKind::HashMap,
            EventSinkKind::BytesChannel,
        ) => {
            let e = builder()
                .with_sequence_generator(CounterSequenceGenerator::new())
                .with_price_book(DashSkipOrderBook::new())
                .with_order_store(HashMapOrderStore::new())
                .with_matching_policy(PriceCrossMatchingPolicy)
                .with_self_trade_policy(AllowAllSelfTradePolicy)
                .with_event_sink(sink_bytes())
                .build();
            MatcherEngine::DashSkipHashBytes(e)
        }
        (
            PriceBookKind::PoolLevel,
            OrderStoreKind::HashMap,
            EventSinkKind::BytesChannel,
        ) => {
            let e = builder()
                .with_sequence_generator(CounterSequenceGenerator::new())
                .with_price_book(PoolLevelOrderBook::new())
                .with_order_store(HashMapOrderStore::new())
                .with_matching_policy(PriceCrossMatchingPolicy)
                .with_self_trade_policy(AllowAllSelfTradePolicy)
                .with_event_sink(sink_bytes())
                .build();
            MatcherEngine::PoolLevelHashBytes(e)
        }
        (PriceBookKind::Btree, OrderStoreKind::Dense, EventSinkKind::Noop) => {
            let e = builder()
                .with_sequence_generator(CounterSequenceGenerator::new())
                .with_price_book(BTreeOrderBook::new())
                .with_order_store(DenseOrderStore::new())
                .with_matching_policy(PriceCrossMatchingPolicy)
                .with_self_trade_policy(AllowAllSelfTradePolicy)
                .with_event_sink(sink_noop())
                .build();
            MatcherEngine::BtreeDenseNoop(e)
        }
        (PriceBookKind::DashSkip, OrderStoreKind::Dense, EventSinkKind::Noop) => {
            let e = builder()
                .with_sequence_generator(CounterSequenceGenerator::new())
                .with_price_book(DashSkipOrderBook::new())
                .with_order_store(DenseOrderStore::new())
                .with_matching_policy(PriceCrossMatchingPolicy)
                .with_self_trade_policy(AllowAllSelfTradePolicy)
                .with_event_sink(sink_noop())
                .build();
            MatcherEngine::DashSkipDenseNoop(e)
        }
        (
            PriceBookKind::PoolLevel,
            OrderStoreKind::Dense,
            EventSinkKind::Noop,
        ) => {
            let e = builder()
                .with_sequence_generator(CounterSequenceGenerator::new())
                .with_price_book(PoolLevelOrderBook::new())
                .with_order_store(DenseOrderStore::new())
                .with_matching_policy(PriceCrossMatchingPolicy)
                .with_self_trade_policy(AllowAllSelfTradePolicy)
                .with_event_sink(sink_noop())
                .build();
            MatcherEngine::PoolLevelDenseNoop(e)
        }
        (
            PriceBookKind::Btree,
            OrderStoreKind::Dense,
            EventSinkKind::BytesChannel,
        ) => {
            let e = builder()
                .with_sequence_generator(CounterSequenceGenerator::new())
                .with_price_book(BTreeOrderBook::new())
                .with_order_store(DenseOrderStore::new())
                .with_matching_policy(PriceCrossMatchingPolicy)
                .with_self_trade_policy(AllowAllSelfTradePolicy)
                .with_event_sink(sink_bytes())
                .build();
            MatcherEngine::BtreeDenseBytes(e)
        }
        (
            PriceBookKind::DashSkip,
            OrderStoreKind::Dense,
            EventSinkKind::BytesChannel,
        ) => {
            let e = builder()
                .with_sequence_generator(CounterSequenceGenerator::new())
                .with_price_book(DashSkipOrderBook::new())
                .with_order_store(DenseOrderStore::new())
                .with_matching_policy(PriceCrossMatchingPolicy)
                .with_self_trade_policy(AllowAllSelfTradePolicy)
                .with_event_sink(sink_bytes())
                .build();
            MatcherEngine::DashSkipDenseBytes(e)
        }
        (
            PriceBookKind::PoolLevel,
            OrderStoreKind::Dense,
            EventSinkKind::BytesChannel,
        ) => {
            let e = builder()
                .with_sequence_generator(CounterSequenceGenerator::new())
                .with_price_book(PoolLevelOrderBook::new())
                .with_order_store(DenseOrderStore::new())
                .with_matching_policy(PriceCrossMatchingPolicy)
                .with_self_trade_policy(AllowAllSelfTradePolicy)
                .with_event_sink(sink_bytes())
                .build();
            MatcherEngine::PoolLevelDenseBytes(e)
        }
    }
}

fn apply_worker_command(
    worker_instrument: u32,
    engine: &mut MatcherEngine,
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
            index.insert(add.id, worker);
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
    let Some((_, worker)) = index.remove(&cancel.order_id) else {
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
