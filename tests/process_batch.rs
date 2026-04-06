//! [`OrderMatchingEngine::process_batch`] matches sequential [`OrderMatchingService::process`] calls.

use omer::engine::{OrderCommand, OrderMatchingService};
use omer::harness::{add_cmd, engine_with_memory};
use omer::types::{OrderType, Side, TimeInForce};

#[test]
fn process_batch_two_resting_buys_equals_two_adds() {
    let (mut a, _sa) = engine_with_memory();
    let (mut b, _sb) = engine_with_memory();

    let c1 = OrderCommand::Add(add_cmd(
        1,
        100,
        Side::Buy,
        OrderType::Limit,
        Some(50),
        10,
        TimeInForce::Gtc,
    ));
    let c2 = OrderCommand::Add(add_cmd(
        2,
        100,
        Side::Buy,
        OrderType::Limit,
        Some(51),
        5,
        TimeInForce::Gtc,
    ));

    a.process_batch(vec![c1.clone(), c2.clone()]).unwrap();
    OrderMatchingService::process(&mut b, c1).unwrap();
    OrderMatchingService::process(&mut b, c2).unwrap();

    assert_eq!(a.best_bid(), b.best_bid());
    assert_eq!(a.best_ask(), b.best_ask());
}
