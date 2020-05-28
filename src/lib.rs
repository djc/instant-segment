use std::{
    fs::File,
    io::{self, BufRead, BufReader},
    num::ParseIntError,
    ops::Range,
    path::Path,
    str::FromStr,
};

use ahash::AHashMap as HashMap;
use err_derive::Error;

pub struct Segmenter {
    unigrams: HashMap<String, f64>,
    bigrams: HashMap<(String, String), f64>,
    total: f64,
    limit: usize,
}

impl Segmenter {
    /// Create `Segmenter` from files in the given directory
    ///
    /// Reads from `unigrams.txt` and `bigrams.txt` in `dir`.
    pub fn from_dir(dir: &Path) -> Result<Self, ParseError> {
        let uni_file = dir.join("unigrams.txt");
        let bi_file = dir.join("bigrams.txt");
        Ok(Self {
            unigrams: parse_unigrams(BufReader::new(File::open(&uni_file)?), uni_file.to_str())?,
            bigrams: parse_bigrams(BufReader::new(File::open(&bi_file)?), bi_file.to_str())?,
            limit: DEFAULT_LIMIT,
            total: DEFAULT_TOTAL,
        })
    }

    /// Returns a list of words that is the best segmentation of `text`
    pub fn segment(&self, text: &str) -> Vec<String> {
        let clean = clean(text);
        let mut words = vec![];
        let mut memo = HashMap::new();

        let (mut start, mut end) = (0, 0);
        loop {
            end = clean.len().min(end + SEGMENT_SIZE);
            let prefix = &clean[start..end];
            let window_words = self.search(&prefix, "<s>", &mut memo).1;

            for word in &window_words[..window_words.len().saturating_sub(5)] {
                start += word.len();
                words.push((*word).into());
            }

            if end == clean.len() {
                break;
            }
        }

        let window_words = self.search(&clean[start..], "<s>", &mut memo).1;
        words.extend(window_words.into_iter().map(|s| s.to_owned()));
        words
    }

    /// Score `word` in the context of `previous` word
    fn search<'a, 'b: 'a>(
        &self,
        text: &'b str,
        previous: &str,
        memo: &'a mut MemoMap<'b>,
    ) -> (f64, Vec<&'b str>) {
        if text.is_empty() {
            return (0.0, vec![]);
        }

        let mut best = (f64::MIN, vec![]);
        for (prefix, suffix) in TextDivider::new(text, self.limit) {
            let prefix_score = self.score(prefix, Some(previous)).log10();
            let pair = (suffix, prefix);

            let (suffix_score, suffix_words) = match memo.get(&pair) {
                Some((score, words)) => (*score, words.as_slice()),
                None => {
                    let (suffix_score, suffix_words) = self.search(&suffix, prefix, memo);
                    let value = memo.entry(pair).or_insert((suffix_score, suffix_words));
                    (suffix_score, value.1.as_slice())
                }
            };

            let score = prefix_score + suffix_score;
            if score > best.0 {
                best.0 = score;
                best.1.clear();
                best.1.push(prefix);
                best.1.extend(suffix_words);
            }
        }

        best
    }

    fn score(&self, word: &str, previous: Option<&str>) -> f64 {
        match previous {
            None => match self.unigrams.get(word) {
                // Probabibility of the given word
                Some(p) => p / self.total,
                // Penalize words not found in the unigrams according
                // to their length, a crucial heuristic.
                None => 10.0 / (self.total * 10.0f64.powf(word.len() as f64)),
            },
            Some(prev) => match (
                self.bigrams.get(&(prev.into(), word.into())),
                self.unigrams.get(prev),
            ) {
                // Conditional probability of the word given the previous
                // word. The technical name is "stupid backoff" and it's
                // not a probability distribution but it works well in practice.
                (Some(pb), Some(_)) => pb / self.total / self.score(prev, None),
                // Fall back to using the unigram probability
                _ => self.score(word, None),
            },
        }
    }

    /// Customize the word length `limit
    pub fn set_limit(&mut self, limit: usize) {
        self.limit = limit;
    }

    /// Customize the relative score by setting the `total`
    pub fn set_total(&mut self, total: f64) {
        self.total = total;
    }
}

/// Parse unigrams from the `reader` (format: `<word>\t<int>\n`)
///
/// The optional `name` argument may be used to provide a source name for error messages.
pub fn parse_unigrams<R: BufRead>(
    reader: R,
    name: Option<&str>,
) -> Result<HashMap<String, f64>, ParseError> {
    let name = name.unwrap_or("(unnamed)");
    reader
        .lines()
        .enumerate()
        .map(|(i, ln)| {
            let ln = ln?;
            let split = ln
                .find('\t')
                .ok_or_else(|| ParseError::String(format!("no tab found in {:?}:{}", name, i)))?;

            let word = ln[..split].to_owned();
            let p = usize::from_str(&ln[split + 1..])
                .map_err(|e| ParseError::String(format!("error at {:?}:{}: {}", name, i, e)))?;
            Ok((word, p as f64))
        })
        .collect()
}

/// Parse bigrams from the `reader` (format: `<word-1> <word-2>\t<int>\n`)
///
/// The optional `name` argument may be used to provide a source name for error messages.
pub fn parse_bigrams<R: BufRead>(
    reader: R,
    name: Option<&str>,
) -> Result<HashMap<(String, String), f64>, ParseError> {
    let name = name.unwrap_or("(unnamed)");
    reader
        .lines()
        .enumerate()
        .map(|(i, ln)| {
            let ln = ln?;
            let word_split = ln
                .find(' ')
                .ok_or_else(|| ParseError::String(format!("no space found in {:?}:{}", name, i)))?;
            let score_split = ln[word_split + 1..]
                .find('\t')
                .ok_or_else(|| ParseError::String(format!("no tab found in {:?}:{}", name, i)))?
                + word_split
                + 1;

            let word1 = ln[..word_split].to_owned();
            let word2 = ln[word_split + 1..score_split].to_owned();
            let p = usize::from_str(&ln[score_split + 1..])
                .map_err(|e| ParseError::String(format!("error at {:?}:{}: {}", name, i, e)))?;

            Ok(((word1, word2), p as f64))
        })
        .collect()
}

/// Iterator that yields `(prefix, suffix)` pairs from `text`
struct TextDivider<'a> {
    text: &'a str,
    split: Range<usize>,
}

impl<'a> TextDivider<'a> {
    fn new(text: &'a str, limit: usize) -> Self {
        TextDivider {
            text,
            split: 1..(text.len().min(limit) + 1),
        }
    }
}

impl<'a> Iterator for TextDivider<'a> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        self.split
            .next()
            .map(|split| (&self.text[..split], &self.text[split..]))
    }
}

/// Return `text` lower-cased with non-alphanumeric characters removed
fn clean(s: &str) -> String {
    s.chars()
        .filter_map(|c| {
            if c.is_ascii_alphanumeric() {
                Some(c.to_ascii_lowercase())
            } else {
                None
            }
        })
        .collect()
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error(display = "I/O error: {}", _0)]
    Io(#[source] io::Error),
    #[error(display = "integer parsing error: {}", _0)]
    ParseInt(#[source] ParseIntError),
    #[error(display = "{}", _0)]
    String(String),
}

type MemoMap<'a> = HashMap<(&'a str, &'a str), (f64, Vec<&'a str>)>;

const DEFAULT_LIMIT: usize = 24;
const DEFAULT_TOTAL: f64 = 1_024_908_267_229.0;
const SEGMENT_SIZE: usize = 250;

#[cfg(test)]
mod tests {
    #[test]
    fn test_clean() {
        assert_eq!(super::clean("Can't buy me love!"), "cantbuymelove");
    }
}
