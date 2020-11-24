#![cfg(feature = "__test_data")]

use bencher::{benchmark_group, benchmark_main, Bencher};

benchmark_group!(benches, short);
benchmark_main!(benches);

fn short(bench: &mut Bencher) {
    let segmenter = word_segmenters::test_data::segmenter();
    let mut out = Vec::new();
    bench.iter(|| segmenter.segment("thisisatest", &mut out));
}
