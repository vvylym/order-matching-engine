//! ITCH decode throughput: [`omer::itch::scan_decode_book_messages`] on a contiguous buffer.
//!
//! Measures **book-affecting** messages decoded per second (add-only workload). This is one
//! component toward stack-level throughput; see README “North star” for end-to-end scope.

use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use omer::itch::scan_decode_book_messages;

/// One AddOrder ITCH packet (36-byte payload), same layout as unit tests in `messages::stream`.
fn one_add_order_packet() -> Vec<u8> {
    let mut buf = vec![0u8; 3 + 36];
    buf[0..2].copy_from_slice(&36u16.to_be_bytes());
    buf[2] = b'A';
    buf[4..6].copy_from_slice(&0u16.to_be_bytes());
    buf[8..14].copy_from_slice(&[0u8; 6]);
    buf[14..22].copy_from_slice(&1u64.to_be_bytes());
    buf[22] = b'B';
    buf[23..27].copy_from_slice(&10u32.to_be_bytes());
    buf[35..39].copy_from_slice(&10000u32.to_be_bytes());
    buf
}

fn itch_parse_throughput(c: &mut Criterion) {
    const N_MSG: u64 = 50_000;
    let one = one_add_order_packet();
    let mut buf = Vec::with_capacity(one.len() * N_MSG as usize);
    for _ in 0..N_MSG {
        buf.extend_from_slice(&one);
    }
    let n_decode = scan_decode_book_messages(&buf).expect("fixture must decode");
    assert_eq!(n_decode, N_MSG);

    let mut group = c.benchmark_group("itch_parse");
    group.throughput(Throughput::Elements(N_MSG));
    group.bench_function("scan_decode_book_messages_add_only", |b| {
        b.iter(|| {
            black_box(scan_decode_book_messages(black_box(&buf)).unwrap());
        });
    });
    group.finish();
}

criterion_group!(benches, itch_parse_throughput);
criterion_main!(benches);
