//! Tokio benchmark harness server.
//!
//! Accepts a simple line protocol and routes commands to shard-local engines.

use std::collections::HashMap;
use std::sync::Arc;

use clap::Parser;
use omer::book::service::BTreeOrderBook;
use omer::engine::{
    AddOrderCommand, CancelByOrderIdCommand, OrderCommand, OrderMatchingService,
    builder,
};
use omer::events::NoOpEventSink;
use omer::matching::PriceCrossMatchingPolicy;
use omer::self_trade::AllowAllSelfTradePolicy;
use omer::sequence::CounterSequenceGenerator;
use omer::store::service::HashMapOrderStore;
use omer::types::{OrderType, Side, TimeInForce};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{RwLock, mpsc};

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, default_value = "127.0.0.1:7001")]
    bind: String,
    #[arg(long, default_value_t = 8)]
    shards: usize,
}

#[derive(Debug)]
enum WorkerCmd {
    Add(AddOrderCommand),
    CancelById(CancelByOrderIdCommand),
}

type RouterIndex = Arc<RwLock<HashMap<u64, usize>>>;
type WorkerSender = mpsc::UnboundedSender<WorkerCmd>;
type DynErr = Box<dyn std::error::Error>;
type ParseErr = &'static str;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), DynErr> {
    let args = Args::parse();
    let listener = TcpListener::bind(&args.bind).await?;
    let shards = args.shards.max(1);

    let mut senders = Vec::with_capacity(shards);
    for shard_id in 0..shards {
        let (tx, mut rx) = mpsc::unbounded_channel::<WorkerCmd>();
        senders.push(tx);
        tokio::spawn(async move {
            let mut engine = builder()
                .with_sequence_generator(CounterSequenceGenerator::new())
                .with_price_book(BTreeOrderBook::new())
                .with_order_store(HashMapOrderStore::new())
                .with_matching_policy(PriceCrossMatchingPolicy)
                .with_self_trade_policy(AllowAllSelfTradePolicy)
                .with_event_sink(NoOpEventSink)
                .build();

            while let Some(cmd) = rx.recv().await {
                match cmd {
                    WorkerCmd::Add(add) => {
                        let _ = engine.process(OrderCommand::Add(add));
                    }
                    WorkerCmd::CancelById(cancel) => {
                        let _ =
                            engine.process(OrderCommand::CancelByOrderId(cancel));
                    }
                }
            }

            eprintln!("worker {shard_id} exited");
        });
    }

    let router_index: RouterIndex = Arc::new(RwLock::new(HashMap::new()));
    println!("server listening on {}", args.bind);

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
    senders: Vec<WorkerSender>,
    index: RouterIndex,
) -> Result<(), DynErr> {
    let shards = senders.len();
    let (read_half, mut write_half) = stream.into_split();
    let mut lines = BufReader::new(read_half).lines();

    while let Some(line) = lines.next_line().await? {
        write_response(
            &mut write_half,
            route_and_dispatch(&line, shards, &senders, &index).await,
        )
        .await?;
    }
    Ok(())
}

async fn write_response(
    writer: &mut tokio::net::tcp::OwnedWriteHalf,
    result: Result<RouteResult, ParseErr>,
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
    shards: usize,
    senders: &[WorkerSender],
    index: &RouterIndex,
) -> Result<RouteResult, ParseErr> {
    match parse_command(line)? {
        WireCmd::Add(add) => {
            let shard = (add.symbol_id as usize) % shards;
            index.write().await.insert(add.id, shard);
            let _ = senders[shard].send(WorkerCmd::Add(add));
            Ok(RouteResult::Ok)
        }
        WireCmd::Market(mut add) => {
            let shard = (add.symbol_id as usize) % shards;
            add.order_type = OrderType::Market;
            add.price = None;
            add.time_in_force = TimeInForce::Ioc;
            let _ = senders[shard].send(WorkerCmd::Add(add));
            Ok(RouteResult::Ok)
        }
        WireCmd::CancelById(order_id) => {
            let route = index.write().await.remove(&order_id);
            if let Some(shard) = route {
                let _ = senders[shard].send(WorkerCmd::CancelById(
                    CancelByOrderIdCommand { order_id },
                ));
                Ok(RouteResult::Ok)
            } else {
                Ok(RouteResult::UnknownOrder)
            }
        }
    }
}

enum WireCmd {
    Add(AddOrderCommand),
    Market(AddOrderCommand),
    CancelById(u64),
}

fn parse_command(line: &str) -> Result<WireCmd, ParseErr> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.is_empty() {
        return Err("empty");
    }

    match parts[0] {
        "ADD" if parts.len() == 7 => parse_add(parts.as_slice()),
        "MARKET" if parts.len() == 6 => parse_market(parts.as_slice()),
        "CANCELID" if parts.len() == 2 => {
            let order_id = parts[1].parse().map_err(|_| "order_id")?;
            Ok(WireCmd::CancelById(order_id))
        }
        _ => Err("unknown"),
    }
}

fn parse_side(s: &str) -> Result<Side, ParseErr> {
    match s {
        "B" | "BUY" => Ok(Side::Buy),
        "S" | "SELL" => Ok(Side::Sell),
        _ => Err("side"),
    }
}

fn parse_add(parts: &[&str]) -> Result<WireCmd, ParseErr> {
    let id = parts[1].parse().map_err(|_| "id")?;
    let participant_id = parts[2].parse().map_err(|_| "pid")?;
    let symbol_id = parts[3].parse().map_err(|_| "symbol")?;
    let side = parse_side(parts[4])?;
    let price = parts[5].parse().map_err(|_| "price")?;
    let quantity = parts[6].parse().map_err(|_| "qty")?;
    Ok(WireCmd::Add(AddOrderCommand {
        id,
        participant_id,
        symbol_id,
        side,
        order_type: OrderType::Limit,
        price: Some(price),
        quantity,
        time_in_force: TimeInForce::Gtc,
        stop_price: None,
        max_visible_quantity: None,
        slippage: None,
        trailing_distance: None,
        trailing_step: None,
    }))
}

fn parse_market(parts: &[&str]) -> Result<WireCmd, ParseErr> {
    let id = parts[1].parse().map_err(|_| "id")?;
    let participant_id = parts[2].parse().map_err(|_| "pid")?;
    let symbol_id = parts[3].parse().map_err(|_| "symbol")?;
    let side = parse_side(parts[4])?;
    let quantity = parts[5].parse().map_err(|_| "qty")?;
    Ok(WireCmd::Market(AddOrderCommand {
        id,
        participant_id,
        symbol_id,
        side,
        order_type: OrderType::Market,
        price: None,
        quantity,
        time_in_force: TimeInForce::Ioc,
        stop_price: None,
        max_visible_quantity: None,
        slippage: None,
        trailing_distance: None,
        trailing_step: None,
    }))
}
