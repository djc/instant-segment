use std::path::PathBuf;

use bencher::{benchmark_group, benchmark_main, Bencher};

use word_segmenters::Segmenter;

benchmark_group!(benches, short);
benchmark_main!(benches);

fn short(bench: &mut Bencher) {
    let segmenter = Segmenter::from_dir(&PathBuf::from(format!(
        "{}/data",
        env!("CARGO_MANIFEST_DIR")
    )))
    .unwrap();

    bench.iter(|| segmenter.segment("thisisatest"));
}
