use instant_segment::{Search, Segmenter};
use std::collections::HashMap;

fn main() {
    let mut unigrams = HashMap::default();

    unigrams.insert("choose".into(), 50 as f64);
    unigrams.insert("chooses".into(), 10 as f64);

    unigrams.insert("spain".into(), 50 as f64);
    unigrams.insert("pain".into(), 10 as f64);

    let mut bigrams = HashMap::default();

    bigrams.insert(("choose".into(), "spain".into()), 10 as f64);
    bigrams.insert(("chooses".into(), "pain".into()), 10 as f64);

    let segmenter = Segmenter::from_maps(unigrams, bigrams);
    let mut search = Search::default();

    let words = segmenter.segment("choosespain", &mut search).unwrap();

    println!("{:?}", words.collect::<Vec<&str>>());
}
