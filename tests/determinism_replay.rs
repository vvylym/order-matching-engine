//! IX. Determinism & Replay
//!
//! Same command sequence yields same trades/events/book; snapshot + log replay.

mod common;

use common::{
    EventSnapshot, add_cmd, engine_with_memory, engine_with_shared_state,
};
use omer::engine::{OrderCommand, OrderMatchingService};
use omer::store::OrderStore;
use omer::types::{OrderType, Side, TimeInForce};

/// IX.1 Deterministic execution: same sequence yields same events and outcome.
#[test]
fn deterministic_execution() {
    let (mut engine1, sink1) = engine_with_memory();
    let (mut engine2, sink2) = engine_with_memory();

    let cmds: Vec<OrderCommand> = vec![
        OrderCommand::Add(add_cmd(
            1,
            100,
            Side::Sell,
            OrderType::Limit,
            Some(50),
            10,
            TimeInForce::Gtc,
        )),
        OrderCommand::Add(add_cmd(
            2,
            101,
            Side::Buy,
            OrderType::Limit,
            Some(50),
            5,
            TimeInForce::Gtc,
        )),
        OrderCommand::Add(add_cmd(
            3,
            100,
            Side::Buy,
            OrderType::Limit,
            Some(49),
            3,
            TimeInForce::Gtc,
        )),
    ];

    for cmd in cmds.iter().cloned() {
        let r1 = engine1.process(cmd.clone());
        let r2 = engine2.process(cmd);
        assert_eq!(r1.is_ok(), r2.is_ok());
    }

    let ev1 = sink1.snapshot();
    let ev2 = sink2.snapshot();
    assert_eq!(ev1.len(), ev2.len());
    for (a, b) in ev1.iter().zip(ev2.iter()) {
        match (&a.event, &b.event) {
            (EventSnapshot::Accepted(id1), EventSnapshot::Accepted(id2)) => {
                assert_eq!(id1, id2)
            }
            (EventSnapshot::Trade { .. }, EventSnapshot::Trade { .. }) => {}
            (EventSnapshot::Canceled(id1), EventSnapshot::Canceled(id2)) => {
                assert_eq!(id1, id2)
            }
            (EventSnapshot::Rejected(_), EventSnapshot::Rejected(_)) => {}
            _ => panic!("event mismatch: {:?} vs {:?}", a.event, b.event),
        }
    }
}

/// IX.2 Snapshot replay: given the same command sequence run on two engines with observable state,
/// when both complete, then the event logs and final book state (depth) must be identical.
#[test]
fn snapshot_replay_state_matches() {
    let (mut engine1, sink1, book1, store1) = engine_with_shared_state();
    let (mut engine2, sink2, book2, store2) = engine_with_shared_state();

    let cmds: Vec<OrderCommand> = vec![
        OrderCommand::Add(add_cmd(
            1,
            100,
            Side::Sell,
            OrderType::Limit,
            Some(50),
            10,
            TimeInForce::Gtc,
        )),
        OrderCommand::Add(add_cmd(
            2,
            101,
            Side::Buy,
            OrderType::Limit,
            Some(50),
            5,
            TimeInForce::Gtc,
        )),
        OrderCommand::Add(add_cmd(
            3,
            100,
            Side::Buy,
            OrderType::Limit,
            Some(49),
            3,
            TimeInForce::Gtc,
        )),
        OrderCommand::Cancel(omer::engine::CancelOrderCommand {
            order_id: 3,
            participant_id: 100,
            sequence: 2,
        }),
    ];

    for cmd in cmds.iter().cloned() {
        let r1 = engine1.process(cmd.clone());
        let r2 = engine2.process(cmd);
        assert_eq!(
            r1.is_ok(),
            r2.is_ok(),
            "same command must succeed or fail on both"
        );
    }

    let ev1 = sink1.snapshot();
    let ev2 = sink2.snapshot();
    assert_eq!(ev1.len(), ev2.len(), "event log length must match");
    for (a, b) in ev1.iter().zip(ev2.iter()) {
        match (&a.event, &b.event) {
            (EventSnapshot::Accepted(id1), EventSnapshot::Accepted(id2)) => {
                assert_eq!(id1, id2)
            }
            (EventSnapshot::Trade { .. }, EventSnapshot::Trade { .. }) => {}
            (EventSnapshot::Canceled(id1), EventSnapshot::Canceled(id2)) => {
                assert_eq!(id1, id2)
            }
            (EventSnapshot::Rejected(_), EventSnapshot::Rejected(_)) => {}
            _ => panic!("event mismatch: {:?} vs {:?}", a.event, b.event),
        }
    }

    let depth1 = book1
        .borrow()
        .total_depth(|id| store1.borrow().get(&id).map(|o| o.leaves_quantity));
    let depth2 = book2
        .borrow()
        .total_depth(|id| store2.borrow().get(&id).map(|o| o.leaves_quantity));
    assert_eq!(depth1, depth2, "final book depth must match after replay");
}
