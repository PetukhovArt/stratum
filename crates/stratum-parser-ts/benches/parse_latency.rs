//! Criterion bench for OXC TSX parse latency on a small file.
//!
//! Baseline 2026-05-18 on Intel Core i5-12600K: ~3.19 µs/iter
//! (range 3.15–3.23 µs, 100 samples, release profile).

#![allow(clippy::unwrap_used)]

use camino::Utf8Path;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use stratum_parser_ts::{LanguageExtractor, OxcTsExtractor};

const SAMPLE: &str = r#"
import React from "react";
import { useState } from "react";
import { Cart } from "@/features/cart";
import type { User } from "@/entities/user";

export function App() {
  const [count, setCount] = useState(0);
  return <Cart count={count} />;
}
"#;

fn bench_extract(c: &mut Criterion) {
    let e = OxcTsExtractor::new();
    c.bench_function("oxc_extract_small_tsx", |b| {
        b.iter(|| {
            let data = e
                .extract(Utf8Path::new("App.tsx"), black_box(SAMPLE))
                .unwrap();
            black_box(data);
        });
    });
}

criterion_group!(benches, bench_extract);
criterion_main!(benches);
