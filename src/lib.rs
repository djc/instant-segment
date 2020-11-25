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
    pub fn from_iters<'a, U, B>(unigrams: U, bigrams: B) -> Result<Self, Box<dyn Error>>
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
    memo: HashMap<(&'a str, &'a str), (f64, Vec<&'a str>)>,
    result: &'a mut Vec<String>,
}

impl<'a> SegmentState<'a> {
    fn new(text: &'a str, data: &'a Segmenter, result: &'a mut Vec<String>) -> Self {
        Self {
            data,
            text,
            memo: HashMap::new(),
            result,
        }
    }

    /// Returns a list of words that is the best segmentation of `text`
    fn run(mut self) {
        let (mut start, mut end) = (0, 0);
        loop {
            end = self.text.len().min(end + SEGMENT_SIZE);
            let prefix = &self.text[start..end];
            let window_words = self.search(&prefix, "<s>").1;

            for word in &window_words[..window_words.len().saturating_sub(5)] {
                start += word.len();
                self.result.push((*word).into());
            }

            if end == self.text.len() {
                break;
            }
        }

        let window_words = self.search(&self.text[start..], "<s>").1;
        self.result
            .extend(window_words.into_iter().map(|s| s.into()));
    }

    /// Score `word` in the context of `previous` word
    fn search(&mut self, text: &'a str, previous: &str) -> (f64, Vec<&'a str>) {
        if text.is_empty() {
            return (0.0, vec![]);
        }

        let mut best = (f64::MIN, vec![]);
        for (prefix, suffix) in TextDivider::new(text, self.data.limit) {
            let prefix_score = self.data.score(prefix, Some(previous)).log10();
            let pair = (suffix, prefix);

            let (suffix_score, suffix_words) = match self.memo.get(&pair) {
                Some((score, words)) => (*score, words.as_slice()),
                None => {
                    let (suffix_score, suffix_words) = self.search(&suffix, prefix);
                    let value = self
                        .memo
                        .entry(pair)
                        .or_insert((suffix_score, suffix_words));
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
