#![cfg(feature = "__test_data")]

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::str::FromStr;

use super::{HashMap, Segmenter};

#[test]
fn test_data() {
    crate::test_cases::run(&segmenter(crate_data_dir()));
}

pub fn segmenter(dir: PathBuf) -> Segmenter {
    let mut ln = String::new();

    let uni_file = dir.join("en-unigrams.txt");
    let mut reader = BufReader::new(File::open(&uni_file).unwrap());
    let mut i = 0;
    let mut unigrams = HashMap::default();
    while reader.read_line(&mut ln).unwrap() > 0 {
        i += 1;
        let split = ln
            .find('\t')
            .unwrap_or_else(|| panic!("no tab found in {:?}:{}", uni_file, i));

        let word = ln[..split].into();
        let p = usize::from_str(ln[split + 1..].trim())
            .unwrap_or_else(|e| panic!("error at {:?}:{}: {}", uni_file, i, e));
        unigrams.insert(word, p as f64);
        ln.clear();
    }

    let bi_file = dir.join("en-bigrams.txt");
    let mut reader = BufReader::new(File::open(&bi_file).unwrap());
    let mut i = 0;
    let mut bigrams = HashMap::default();
    while reader.read_line(&mut ln).unwrap() > 0 {
        i += 1;
        let word_split = ln
            .find(' ')
            .unwrap_or_else(|| panic!("no space found in {:?}:{}", bi_file, i));
        let score_split = ln[word_split + 1..]
            .find('\t')
            .unwrap_or_else(|| panic!("no tab found in {:?}:{}", bi_file, i))
            + word_split
            + 1;

        let word1 = ln[..word_split].into();
        let word2 = ln[word_split + 1..score_split].into();
        let p = usize::from_str(ln[score_split + 1..].trim())
            .unwrap_or_else(|e| panic!("error at {:?}:{}: {}", bi_file, i, e));

        bigrams.insert((word1, word2), p as f64);
        ln.clear();
    }

    Segmenter::from_maps(unigrams, bigrams)
}

pub fn crate_data_dir() -> PathBuf {
    PathBuf::from(format!("{}/../data", env!("CARGO_MANIFEST_DIR")))
}
