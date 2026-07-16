use criterion::{criterion_group, criterion_main, Criterion};
use regex::Regex;

fn bench_regex_matching(c: &mut Criterion) {
    let re = Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
    let sample = "2024-01-01";

    c.bench_function("regex_matching", |b| {
        b.iter(|| {
            let _ = re.is_match(sample);
        });
    });
}

criterion_group!(benches, bench_regex_matching);
criterion_main!(benches);
