use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use crm_tool::utils::build_csv_reader_from_reader;

fn bench_csv_parsing(c: &mut Criterion) {
    let mut data = String::new();
    data.push_str("col1,col2,col3\n");
    for i in 0..1000 {
        data.push_str(&format!("{},{},{}\n", i, i * 2, i * 3));
    }
    let data_bytes = data.into_bytes();

    c.bench_function("csv_parsing 1000 rows", |b| {
        // Isolate measurement strictly to the parser, avoiding reallocation of test data
        b.iter_batched(
            || data_bytes.as_slice(),
            |bytes| {
                let mut rdr = build_csv_reader_from_reader(bytes);
                let mut valid_count = 0;
                for result in rdr.records() {
                    let _ = result.unwrap();
                    valid_count += 1;
                }
                valid_count
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, bench_csv_parsing);
criterion_main!(benches);
