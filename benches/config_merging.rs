use criterion::{criterion_group, criterion_main, Criterion};
use crm_tool::utils::merge_json;
use serde_json::json;
use std::hint::black_box;

fn bench_config_merging(c: &mut Criterion) {
    let default = json!({
        "url": "https://example.com",
        "username": "admin",
        "concurrency": 6,
        "features": ["a", "b"],
        "options": {
            "retry": true,
            "timeout": 30
        },
        "reports": {
            "r1": {
                "type": "daily",
                "filters": {
                    "status": "closed"
                }
            }
        }
    });

    let current = json!({
        "url": "https://custom.com",
        "options": {
            "timeout": 60
        },
        "reports": {
            "r1": {
                "filters": {
                    "owner": "admin"
                }
            },
            "r2": {
                "type": "weekly"
            }
        }
    });

    c.bench_function("merge_json_complex", |b| {
        b.iter(|| {
            let mut curr = current.clone();
            merge_json(&mut curr, &default);
            black_box(curr);
        });
    });
}

criterion_group!(benches, bench_config_merging);
criterion_main!(benches);
