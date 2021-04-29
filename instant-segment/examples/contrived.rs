use instant_segment::{Search, Segmenter};
use std::collections::HashMap;

fn main() {
    let mut unigrams = HashMap::default();

    unigrams.insert("choose".into(), 80_000.0);
    unigrams.insert("chooses".into(), 7_000.0);

    unigrams.insert("spain".into(), 20_000.0);
    unigrams.insert("pain".into(), 90_000.0);

    let mut bigrams = HashMap::default();

    bigrams.insert(("choose".into(), "spain".into()), 7.0);
    bigrams.insert(("chooses".into(), "pain".into()), 0.0);

    let segmenter = Segmenter::from_maps(unigrams, bigrams);
    let mut search = Search::default();

    let words = segmenter.segment("choosespain", &mut search).unwrap();

    println!("{:?}", words.collect::<Vec<&str>>());
}
