#![cfg(feature = "__test_data")]

use bencher::{benchmark_group, benchmark_main, Bencher};

use instant_segment::test_data::{crate_data_dir, segmenter};
use instant_segment::Search;

benchmark_group!(benches, short);
benchmark_main!(benches);

fn short(bench: &mut Bencher) {
    let segmenter = segmenter(crate_data_dir());
    let mut search = Search::default();
    bench.iter(|| {
        let _ = segmenter.segment("thisisatest", &mut search);
    });
}
