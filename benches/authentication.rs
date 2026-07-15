use criterion::{criterion_group, criterion_main, Criterion};

fn bench_authentication(c: &mut Criterion) {
    c.bench_function("authentication placeholder", |b| {
        b.iter(|| {
            let _ = 1 + 1;
        });
    });
}

criterion_group!(benches, bench_authentication);
criterion_main!(benches);
