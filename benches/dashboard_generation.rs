use criterion::{criterion_group, criterion_main, Criterion};

fn bench_dashboard_generation(c: &mut Criterion) {
    c.bench_function("dashboard_generation placeholder", |b| {
        b.iter(|| {
            let _ = 1 + 1;
        });
    });
}

criterion_group!(benches, bench_dashboard_generation);
criterion_main!(benches);
