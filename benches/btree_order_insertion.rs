use arbitrary::{Arbitrary, Unstructured};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ironlobe::{
    book::{
        btree_book::{BTreeBook, Metadata},
        Book,
    },
    order::PlainOrder,
};
use rand::rngs::StdRng;
use rand::{Rng, RngCore, SeedableRng};

const SAMPLE_SECS: u64 = 5;
const BUFLEN: usize = 256;

fn make_orders(n: usize) -> Vec<PlainOrder> {
    let mut rng = StdRng::seed_from_u64(42); // Deterministic RNG for reproducibility
    (0..n)
        .map(|_| {
            let mut bytes = vec![0u8; BUFLEN];
            rng.fill_bytes(&mut bytes);
            let mut unstructured = Unstructured::new(&bytes);
            let mut order = PlainOrder::arbitrary(&mut unstructured)
                .expect("Failed to generate instance");
            order.price = rng.gen_range(10.0..100.0); // Set realistic price ranges
            order
        })
        .collect()
}

fn insert_into_book(
    orders: &Vec<PlainOrder>,
    book: &mut BTreeBook<PlainOrder>,
) {
    orders.iter().for_each(|x| book.add(x.clone())); // Ensure add can handle references to avoid cloning
}

fn benchmark_1000(c: &mut Criterion) {
    let orders = make_orders(black_box(1000));

    c.bench_function("insert 1000", |b| {
        b.iter(|| {
            let mut book = BTreeBook::meta(Metadata {
                id: 1,
                name: "Benchmark Book".to_string(),
                ticker: "BENCH".to_string(),
            });
            insert_into_book(&orders, &mut book)
        })
    });
}

fn benchmark_10000(c: &mut Criterion) {
    let orders = make_orders(black_box(10000));

    c.bench_function("insert 10000", |b| {
        b.iter(|| {
            let mut book = BTreeBook::meta(Metadata {
                id: 1,
                name: "Benchmark Book".to_string(),
                ticker: "BENCH".to_string(),
            });
            insert_into_book(&orders, &mut book)
        })
    });
}

fn configure_criterion() -> Criterion {
    Criterion::default()
        .measurement_time(std::time::Duration::from_secs(SAMPLE_SECS))
}

criterion_group! {
    name = benches;
    config = configure_criterion();
    targets = benchmark_1000, benchmark_10000,
}
criterion_main!(benches);
