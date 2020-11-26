use std::error::Error;
use std::io;
use std::num::ParseIntError;
use std::ops::{Index, Range};
use std::str;

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
        SegmentState::new(&Ascii::new(text), &self, out).run()
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
    text: &'a Ascii,
    memo: HashMap<(Range<usize>, Range<usize>), (f64, Range<usize>)>,
    split_cache: Vec<usize>,
    result: &'a mut Vec<String>,
    best: Vec<Vec<usize>>,
}

impl<'a> SegmentState<'a> {
    fn new(text: &'a Ascii, data: &'a Segmenter, result: &'a mut Vec<String>) -> Self {
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
        while end < self.text.len() {
            end = self.text.len().min(end + SEGMENT_SIZE);
            if !self.search(0, start..end, None).1 {
                continue;
            }

            let mut splits = &self.best[0][..];
            if end < self.text.len() {
                splits = &splits[..splits.len().saturating_sub(5)];
            }

            for &split in splits {
                self.result.push(self.text[start..split].into());
                start = split;
            }
        }
    }

    /// Score `word` in the context of `previous` word
    fn search(&mut self, level: usize, range: Range<usize>, previous: Option<&str>) -> (f64, bool) {
        if range.is_empty() {
            return (0.0, false);
        }

        let mut best = f64::MIN;
        for split in 1..(range.len().min(self.data.limit) + 1) {
            let (start, split, end) = (range.start, range.start + split, range.end);
            let prefix = &self.text[start..split];
            let prefix_score = self.data.score(prefix, previous).log10();

            let pair = (split..end, start..split);
            let (suffix_score, suffix_splits) = match self.memo.get(&pair) {
                Some((score, splits)) => (*score, &self.split_cache[splits.start..splits.end]),
                None => {
                    let (suffix_score, has_splits) =
                        self.search(level + 1, split..end, Some(prefix));
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

struct Ascii(Vec<u8>);

impl Ascii {
    fn new(s: &str) -> Self {
        Self(
            s.chars()
                .filter_map(|c| match c.is_ascii_alphanumeric() {
                    true => Some(c.to_ascii_lowercase()),
                    false => None,
                })
                .collect::<std::string::String>()
                .into_bytes(),
        )
    }

    fn len(&self) -> usize {
        self.0.len()
    }
}

impl Index<Range<usize>> for Ascii {
    type Output = str;

    fn index(&self, index: Range<usize>) -> &Self::Output {
        let bytes = self.0.index(index);
        // Since `Ascii` can only be instantiated with ASCII characters, this should be safe
        unsafe { str::from_utf8_unchecked(bytes) }
    }
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
        let text = super::Ascii::new("Can't buy me love!");
        assert_eq!(&text[0..text.len()], "cantbuymelove");
    }
}
