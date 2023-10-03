//! Merge downloaded data to synthesize test data files
//!
//! This is not actually an example, but a tool to help recreate the required
//! data files from publicly available sources. See the README in `/data`.

use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;
use std::io::{BufRead, BufReader, BufWriter};
use std::str::FromStr;

use rayon::iter::{IntoParallelIterator, ParallelIterator};
use smartstring::alias::String as SmartString;

fn main() {
    let word_list = read_word_list();
    process_unigrams(&word_list);
    process_bigrams(&word_list);
}

/// Read bigrams from the input file parts, filter them, and write to file
fn process_bigrams(word_list: &HashSet<SmartString>) {
    let bigrams = (0..BIGRAM_PARTS)
        .into_par_iter()
        .map(|part| {
            let fname = format!("data/cache/eng-2-{:05}-{:05}.txt", part, BIGRAM_PARTS);
            let f = File::open(fname).unwrap();
            let mut reader = BufReader::with_capacity(4 * 1024 * 1024, f);

            let mut ln = String::new();
            let mut bigrams = HashMap::new();
            loop {
                // Example line: `using pozzolan	1925,1,1	1947,2,2	1948,2,2	(...)\n`
                // Tab-separated line. The first column contains two words, separated by a space.
                // Other columns contain a comma-separated triple of (year, match count, volume
                // count).

                ln.clear();
                match reader.read_line(&mut ln) {
                    Ok(0) => break,
                    Err(e) => {
                        eprintln!("error: {:?}", e);
                        break;
                    }
                    _ => {}
                }

                let mut iter = ln.trim().split('\t');
                let words = match iter.next() {
                    Some(word) => word,
                    None => continue,
                };

                let mut word_iter = words.split(' ');
                let word1 = match word_iter.next() {
                    Some(word) => word,
                    _ => continue,
                };

                let word1 = match normalize(word1, word_list) {
                    Some(word) => word,
                    _ => continue,
                };

                let word2 = match word_iter.next() {
                    Some(word) if word_list.contains(word) => word,
                    _ => continue,
                };

                let word2 = match normalize(word2, word_list) {
                    Some(word) => word,
                    _ => continue,
                };

                let mut matches = 0;
                for year_data in iter {
                    let mut parts = year_data.split(',');
                    if parts.next().unwrap() < START_YEAR {
                        continue;
                    }
                    matches += usize::from_str(parts.next().unwrap()).unwrap();
                }

                if bigrams.capacity() == 0 {
                    // While it's not uncommon for a part to result in 0 words, the average for
                    // parts that contain more than 0 is about 300k, median is about 350k. Allocate
                    // a decent chunk immediately to avoid too many intermediate reallocations.
                    bigrams.reserve(256 * 1024)
                }

                *bigrams.entry((word1, word2)).or_default() += matches;
            }

            eprintln!("extracted {} bigrams from part {}", bigrams.len(), part);
            bigrams
        })
        .reduce(
            HashMap::<(SmartString, SmartString), usize>::new,
            |mut left, right| {
                for (k, v) in right.into_iter() {
                    *left.entry(k).or_default() += v;
                }
                left
            },
        );

    let f = File::create("data/en-bigrams.txt").unwrap();
    let mut writer = BufWriter::with_capacity(4 * 1024 * 1024, f);
    let mut bigrams = bigrams.into_iter().collect::<Vec<_>>();
    bigrams.sort_by_key(|(_, freq)| Reverse(*freq));
    for (i, ((left, right), freq)) in bigrams.into_iter().enumerate() {
        if i == MAX_BIGRAMS {
            break;
        }

        writeln!(writer, "{} {}\t{}", left, right, freq).unwrap();
    }
}

/// Read unigrams from the input file parts, filter them, and write to file
fn process_unigrams(word_list: &HashSet<SmartString>) {
    let unigrams = (0..UNIGRAM_PARTS)
        .into_par_iter()
        .map(|part| {
            let fname = format!("data/cache/eng-1-{:05}-{:05}.txt", part, UNIGRAM_PARTS);
            let f = File::open(fname).unwrap();
            let mut reader = BufReader::with_capacity(4 * 1024 * 1024, f);

            let mut ln = String::new();
            let mut unigrams = HashMap::with_capacity(8 * 1024);
            loop {
                // Example line: `ephedrins	1924,1,1	1928,1,1	1931,2,1	(...)\n`
                // Tab-separated line. The first column contains the word. All later columns
                // contain a comma-separated triple of (year, match count, volume count).

                ln.clear();
                match reader.read_line(&mut ln) {
                    Ok(0) => break,
                    Err(e) => {
                        eprintln!("error: {:?}", e);
                        break;
                    }
                    _ => {}
                }

                let mut iter = ln.trim().split('\t');
                let word = match iter.next() {
                    Some(word) => word,
                    _ => continue,
                };

                let word = match normalize(word, word_list) {
                    Some(word) => word,
                    _ => continue,
                };

                let mut matches = 0;
                for year_data in iter {
                    let mut parts = year_data.split(',');
                    if parts.next().unwrap() < START_YEAR {
                        continue;
                    }
                    matches += usize::from_str(parts.next().unwrap()).unwrap();
                }

                *unigrams.entry(word).or_default() += matches;
            }

            eprintln!("extracted {} unigrams from part {}", unigrams.len(), part);
            unigrams
        })
        .reduce(HashMap::<SmartString, usize>::new, |mut left, right| {
            for (k, v) in right.into_iter() {
                *left.entry(k).or_default() += v;
            }
            left
        });

    let mut unigrams = unigrams.into_iter().collect::<Vec<_>>();
    unigrams.sort_by_key(|(_, freq)| Reverse(*freq));
    let f = File::create("data/en-unigrams.txt").unwrap();
    let mut writer = BufWriter::with_capacity(4 * 1024 * 1024, f);
    for (i, (word, freq)) in unigrams.into_iter().enumerate() {
        if i == MAX_UNIGRAMS {
            break;
        }

        writeln!(writer, "{}\t{}", word, freq).unwrap();
    }
}

/// Read the word list and gather it up into a hash set for easy lookups
///
/// We use this to filter crappy words out of the (pretty noisy) ngram data.
/// Considering the way we want to [`normalize()`], we'll filter for
/// only-letter contents but keep any uppercase characters intact.
fn read_word_list() -> HashSet<SmartString> {
    const AVERAGE_WORD_LIST_LINE_LEN: usize = 9;

    let f = File::open("data/cache/eng-wordlist.txt").unwrap();
    let size = f.metadata().unwrap().len() as usize;
    let mut reader = BufReader::with_capacity(4 * 1024 * 1024, f);

    eprintln!("read word list...");
    let mut word_list = HashSet::with_capacity(size / AVERAGE_WORD_LIST_LINE_LEN);
    let mut ln = String::new();
    loop {
        // Example line: `A\n` (`BufRead::read_line()` includes the trailing newline character)

        ln.clear();
        match reader.read_line(&mut ln) {
            Ok(0) => break,
            Err(e) => {
                eprintln!("error: {:?}", e);
                break;
            }
            _ => {}
        }

        let word = ln.trim_end(); // Need to remove the trailing newlines here
        if word.as_bytes().iter().all(|b| b.is_ascii_alphabetic()) {
            word_list.insert(word.into());
        }
    }

    eprintln!("read {} words from word list", word_list.len());
    word_list
}

/// Normalize the input word and filter it
///
/// The order in which we do things here matters quite a bit. First we trim
/// the word to get rid of surrounding whitespace (which can make the word list
/// lookup fail). Then we check if the word consists of only letters -- we
/// disregard any words with digits or punctuation for our purposes. Only then
/// we lowercase the word.
///
/// This has to happen last so that we get the correct match counts from the
/// ngram data. For example, the word 'Spain' is usually capitalized, and only
/// the capitalized version is in the word list. For our purposes though, we
/// want to operate on lowercased words, so we'll do that after filtering.
fn normalize(word: &str, list: &HashSet<SmartString>) -> Option<SmartString> {
    let word = word.trim();
    if !word.as_bytes().iter().all(|b| b.is_ascii_alphabetic()) || !list.contains(word) {
        return None;
    }

    let mut word = SmartString::from(word);
    word.make_ascii_lowercase();
    Some(word)
}

const MAX_UNIGRAMS: usize = 256 * 1024;
const MAX_BIGRAMS: usize = 256 * 1024;

const UNIGRAM_PARTS: usize = 24;
const BIGRAM_PARTS: usize = 589;

const START_YEAR: &str = "2000";
