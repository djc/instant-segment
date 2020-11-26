use std::error::Error;
use std::io;
use std::num::ParseIntError;
use std::ops::Range;

use ahash::AHashMap as HashMap;
use smartstring::alias::String;
use thiserror::Error;

#[cfg(feature = "__test_data")]
pub mod test_data;

pub struct Segmenter {
    unigrams: HashMap<String, f64>,
    bigrams: HashMap<(String, String), f64>,
    total: f64,
    limit: usize,
}

impl Segmenter {
    /// Create `Segmenter` from the given iterators
    ///
    /// Note: the `String` types used in this API are defined in the `smartstring` crate. Any
    /// `&str` or `String` can be converted into the `String` used here by calling `into()` on it.
    pub fn from_iters<U, B>(unigrams: U, bigrams: B) -> Result<Self, Box<dyn Error>>
    where
        U: Iterator<Item = Result<(String, f64), Box<dyn Error>>>,
        B: Iterator<Item = Result<((String, String), f64), Box<dyn Error>>>,
    {
        Ok(Self {
            unigrams: unigrams.collect::<Result<HashMap<_, _>, _>>()?,
            bigrams: bigrams.collect::<Result<HashMap<_, _>, _>>()?,
            limit: DEFAULT_LIMIT,
            total: DEFAULT_TOTAL,
        })
    }

    /// Appends list of words that is the best segmentation of `text` to `out`
    pub fn segment(&self, text: &str, out: &mut Vec<String>) {
        let clean = clean(text);
        SegmentState::new(&clean, &self, out).run()
    }

    fn score(&self, word: &str, previous: Option<&str>) -> f64 {
        if let Some(prev) = previous {
            if let Some(pb) = self.bigrams.get(&(prev.into(), word.into())) {
                if self.unigrams.get(prev).is_some() {
                    // Conditional probability of the word given the previous
                    // word. The technical name is "stupid backoff" and it's
                    // not a probability distribution but it works well in practice.
                    return pb / self.total / self.score(prev, None);
                }
            }
        }

        match self.unigrams.get(word) {
            // Probability of the given word
            Some(p) => p / self.total,
            // Penalize words not found in the unigrams according
            // to their length, a crucial heuristic.
            None => 10.0 / (self.total * 10.0f64.powf(word.len() as f64)),
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

struct SegmentState<'a> {
    data: &'a Segmenter,
    text: &'a str,
    memo: HashMap<(&'a str, &'a str), (f64, Range<usize>)>,
    split_cache: Vec<usize>,
    result: &'a mut Vec<String>,
    best: Vec<Vec<usize>>,
}

impl<'a> SegmentState<'a> {
    fn new(text: &'a str, data: &'a Segmenter, result: &'a mut Vec<String>) -> Self {
        Self {
            data,
            text,
            memo: HashMap::new(),
            split_cache: Vec::new(),
            result,
            best: vec![vec![]; SEGMENT_SIZE],
        }
    }

    /// Returns a list of words that is the best segmentation of `text`
    fn run(mut self) {
        let (mut start, mut end) = (0, 0);
        loop {
            end = self.text.len().min(end + SEGMENT_SIZE);
            let prefix = &self.text[start..end];
            if self.search(0, &prefix, None).1 {
                let splits = &self.best[0];
                for split in &splits[..splits.len().saturating_sub(5)] {
                    self.result.push(self.text[start..start + split].into());
                    start += split;
                }
            }

            if end == self.text.len() {
                break;
            }
        }

        if self.search(0, &self.text[start..], None).1 {
            for split in &self.best[0] {
                self.result.push(self.text[start..start + split].into());
                start += split;
            }
        }
    }

    /// Score `word` in the context of `previous` word
    fn search(&mut self, level: usize, text: &'a str, previous: Option<&str>) -> (f64, bool) {
        if text.is_empty() {
            return (0.0, false);
        }

        let mut best = f64::MIN;
        for split in 1..(text.len().min(self.data.limit) + 1) {
            let (prefix, suffix) = text.split_at(split);
            let prefix_score = self.data.score(prefix, previous).log10();
            let pair = (suffix, prefix);

            let (suffix_score, suffix_splits) = match self.memo.get(&pair) {
                Some((score, splits)) => (*score, &self.split_cache[splits.start..splits.end]),
                None => {
                    let (suffix_score, has_splits) = self.search(level + 1, &suffix, Some(prefix));
                    let start = self.split_cache.len();
                    self.split_cache.extend(if has_splits {
                        &self.best[level + 1][..]
                    } else {
                        &[]
                    });
                    let end = self.split_cache.len();
                    self.memo.insert(pair, (suffix_score, start..end));
                    (suffix_score, &self.split_cache[start..end])
                }
            };

            let score = prefix_score + suffix_score;
            if score > best {
                best = score;
                let splits = &mut self.best[level];
                splits.clear();
                splits.push(split);
                splits.extend(suffix_splits);
            }
        }

        (best, true)
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
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("integer parsing error: {0}")]
    ParseInt(#[from] ParseIntError),
    #[error("{0}")]
    String(String),
}

impl From<std::string::String> for ParseError {
    fn from(s: std::string::String) -> Self {
        ParseError::String(s.into())
    }
}

const DEFAULT_LIMIT: usize = 24;
const DEFAULT_TOTAL: f64 = 1_024_908_267_229.0;
const SEGMENT_SIZE: usize = 250;

#[cfg(test)]
pub mod tests {
    #[test]
    fn test_clean() {
        assert_eq!(&super::clean("Can't buy me love!"), "cantbuymelove");
    }
}
