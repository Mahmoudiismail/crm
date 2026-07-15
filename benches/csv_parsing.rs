use criterion::{criterion_group, criterion_main, Criterion};
use crm_tool::utils::build_csv_reader_from_reader;

fn bench_csv_parsing(c: &mut Criterion) {
    let mut data = String::new();
    for i in 0..1000 {
        data.push_str(&format!("{},{},{}\n", i, i * 2, i * 3));
    }

    c.bench_function("csv_parsing 1000 rows", |b| {
        b.iter(|| {
            let mut rdr = build_csv_reader_from_reader(data.as_bytes());
            for result in rdr.records() {
                let _ = result.unwrap();
            }
        });
    });
}

criterion_group!(benches, bench_csv_parsing);
criterion_main!(benches);
