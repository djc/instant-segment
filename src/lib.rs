use std::ops::{Index, Range};
use std::str;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use smartstring::alias::String;

#[cfg(feature = "test-cases")]
pub mod test_cases;
#[cfg(feature = "__test_data")]
pub mod test_data;

/// Central data structure used to calculate word probabilities
#[cfg_attr(feature = "with-serde", derive(Deserialize, Serialize))]
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
    pub fn from_iters<U, B>(unigrams: U, bigrams: B) -> Self
    where
        U: Iterator<Item = (String, f64)>,
        B: Iterator<Item = ((String, String), f64)>,
    {
        Self::from_maps(unigrams.collect(), bigrams.collect())
    }

    /// Create `Segmenter` from the given hashmaps (using ahash)
    ///
    /// Note: the `String` types used in this API are defined in the `smartstring` crate. Any
    /// `&str` or `String` can be converted into the `String` used here by calling `into()` on it.
    /// The `HashMap` type here refers to `std::collections::HashMap` parametrized with the
    /// `ahash::RandomState`.
    pub fn from_maps(
        unigrams: HashMap<String, f64>,
        bigrams: HashMap<(String, String), f64>,
    ) -> Self {
        let total = unigrams.values().sum();
        Self {
            unigrams,
            bigrams,
            limit: DEFAULT_LIMIT,
            total,
        }
    }

    /// Segment the text in `input`
    ///
    /// Requires that the input `text` consists of lowercase ASCII characters only. Otherwise,
    /// returns `Err(InvalidCharacter)`. The `search` parameter contains caches that are used
    /// segmentation; passing it in allows the callers to reuse the cache allocations.
    ///
    /// The segmentation result can be retrieved through the `Search::split()` method.
    pub fn segment(
        &self,
        input: &str,
        search: &mut Search,
    ) -> Result<(), InvalidCharacter> {
        SegmentState::new(Ascii::new(input)?, &self, search).run();
        Ok(())
    }

    fn score(&self, word: &str, previous: Option<&str>) -> f64 {
        if let Some(prev) = previous {
            if let Some(bi) = self.bigrams.get(&(prev.into(), word.into())) {
                if let Some(uni) = self.unigrams.get(prev) {
                    // Conditional probability of the word given the previous
                    // word. The technical name is "stupid backoff" and it's
                    // not a probability distribution but it works well in practice.
                    return (bi / self.total) / (uni / self.total);
                }
            }
        }

        match self.unigrams.get(word) {
            // Probability of the given word
            Some(p) => p / self.total,
            // Penalize words not found in the unigrams according
            // to their length, a crucial heuristic.
            None => 10.0 / (self.total * 10.0f64.powi(word.len() as i32)),
        }
    }

    /// Customize the word length `limit`
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
    text: Ascii<'a>,
    search: &'a mut Search,
}

impl<'a> SegmentState<'a> {
    fn new(
        text: Ascii<'a>,
        data: &'a Segmenter,
        search: &'a mut Search,
    ) -> Self {
        search.clear();
        Self {
            data,
            text,
            search,
        }
    }

    /// Returns a list of words that is the best segmentation of `text`
    fn run(mut self) {
        let (mut start, mut end) = (0, 0);
        while end < self.text.len() {
            end = self.text.len().min(end + SEGMENT_SIZE);
            self.search(0, start..end, None);

            let mut splits = &self.search.best[0][..];
            if end < self.text.len() {
                splits = &splits[..splits.len().saturating_sub(5)];
            }

            for &split in splits {
                self.search.result.push(self.text[start..split].into());
                start = split;
            }
        }
    }

    /// Score `word` in the context of `previous` word
    fn search(&mut self, level: usize, range: Range<usize>, previous: Option<Range<usize>>) -> f64 {
        if range.is_empty() {
            self.search.best[level].clear();
            return 0.0;
        }

        let mut best = f64::MIN;
        for split in 1..(range.len().min(self.data.limit) + 1) {
            let (start, split, end) = (range.start, range.start + split, range.end);
            let previous = previous.clone().map(|range| &self.text[range]);
            let prefix_score = self.data.score(&self.text[start..split], previous).log10();

            let pair = (split..end, start..split);
            let (suffix_score, suffix_splits) = match self.search.memo.get(&pair) {
                Some((score, splits)) => {
                    (*score, &self.search.split_cache[splits.start..splits.end])
                }
                None => {
                    let suffix_score = self.search(level + 1, split..end, Some(start..split));

                    let start = self.search.split_cache.len();
                    self.search
                        .split_cache
                        .extend(&self.search.best[level + 1][..]);
                    let end = self.search.split_cache.len();
                    self.search.memo.insert(pair, (suffix_score, start..end));

                    (suffix_score, &self.search.split_cache[start..end])
                }
            };

            let score = prefix_score + suffix_score;
            if score > best {
                best = score;
                let splits = &mut self.search.best[level];
                splits.clear();
                splits.push(split);
                splits.extend(suffix_splits);
            }
        }

        best
    }
}

#[derive(Clone)]
pub struct Search {
    memo: HashMap<MemoKey, (f64, Range<usize>)>,
    split_cache: Vec<usize>,
    best: Vec<Vec<usize>>,
    result: Vec<String>,
}

impl Default for Search {
    fn default() -> Self {
        Self {
            memo: HashMap::default(),
            split_cache: Vec::with_capacity(32),
            best: vec![vec![]; SEGMENT_SIZE],
            result: Vec::new(),
        }
    }
}

impl Search {
    fn clear(&mut self) {
        self.memo.clear();
        self.split_cache.clear();
        for inner in self.best.iter_mut() {
            inner.clear();
        }
        self.result.clear();
    }

    /// Get the segmentation result
    pub fn split(&self) -> impl Iterator<Item = &str> {
        self.result.iter().map(|v| v.as_str())
    }
}

type MemoKey = (Range<usize>, Range<usize>);

#[derive(Debug)]
struct Ascii<'a>(&'a [u8]);

impl<'a> Ascii<'a> {
    fn new(s: &'a str) -> Result<Self, InvalidCharacter> {
        let bytes = s.as_bytes();
        match bytes.iter().all(|b| b.is_ascii_lowercase()) {
            true => Ok(Self(bytes)),
            false => Err(InvalidCharacter),
        }
    }

    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<'a> Index<Range<usize>> for Ascii<'a> {
    type Output = str;

    fn index(&self, index: Range<usize>) -> &Self::Output {
        let bytes = self.0.index(index);
        // Since `Ascii` can only be instantiated with ASCII characters, this should be safe
        unsafe { str::from_utf8_unchecked(bytes) }
    }
}

#[derive(Debug)]
pub struct InvalidCharacter;

impl std::error::Error for InvalidCharacter {}

impl std::fmt::Display for InvalidCharacter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid character")
    }
}

type HashMap<K, V> = std::collections::HashMap<K, V, ahash::RandomState>;

const DEFAULT_LIMIT: usize = 24;
const SEGMENT_SIZE: usize = 250;

#[cfg(test)]
pub mod tests {
    #[test]
    fn test_clean() {
        super::Ascii::new("Can't buy me love!").unwrap_err();
        let text = super::Ascii::new("cantbuymelove").unwrap();
        assert_eq!(&text[0..text.len()], "cantbuymelove");
    }
}
