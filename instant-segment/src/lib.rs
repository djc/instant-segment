use std::ops::{BitOrAssign, Index, Range};
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
    uni_total: f64,
    bi_total: f64,
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
        Self {
            uni_total: unigrams.values().sum(),
            bi_total: bigrams.values().sum(),
            unigrams,
            bigrams,
            limit: DEFAULT_LIMIT,
        }
    }

    /// Segment the text in `input`
    ///
    /// Requires that the input `text` consists of lowercase ASCII characters only. Otherwise,
    /// returns `Err(InvalidCharacter)`. The `search` parameter contains caches that are used
    /// segmentation; passing it in allows the callers to reuse the cache allocations.
    pub fn segment<'a>(
        &self,
        input: &str,
        search: &'a mut Search,
    ) -> Result<impl Iterator<Item = &'a str> + ExactSizeIterator, InvalidCharacter> {
        SegmentState::new(Ascii::new(input)?, &self, search).run();
        Ok(search.result.iter().map(|v| v.as_str()))
    }

    fn score(&self, word: &str, previous: Option<&str>) -> f64 {
        if let Some(prev) = previous {
            if let Some(bi) = self.bigrams.get(&(prev.into(), word.into())) {
                if let Some(uni) = self.unigrams.get(prev) {
                    // Conditional probability of the word given the previous
                    // word. The technical name is "stupid backoff" and it's
                    // not a probability distribution but it works well in practice.
                    return (bi / self.bi_total) / (uni / self.uni_total);
                }
            }
        }

        match self.unigrams.get(word) {
            // Probability of the given word
            Some(p) => p / self.uni_total,
            // Penalize words not found in the unigrams according
            // to their length, a crucial heuristic.
            None => 10.0 / (self.uni_total * 10.0f64.powi(word.len() as i32)),
        }
    }

    /// Customize the word length `limit`
    pub fn set_limit(&mut self, limit: usize) {
        self.limit = limit;
    }
}

struct SegmentState<'a> {
    data: &'a Segmenter,
    text: Ascii<'a>,
    search: &'a mut Search,
    offset: usize,
}

impl<'a> SegmentState<'a> {
    fn new(text: Ascii<'a>, data: &'a Segmenter, search: &'a mut Search) -> Self {
        search.clear();
        Self {
            data,
            text,
            search,
            offset: 0,
        }
    }

    /// Returns a list of words that is the best segmentation of `text`
    fn run(mut self) {
        let (mut start, mut end) = (0, 0);
        while end < self.text.len() {
            end = self.text.len().min(end + SEGMENT_SIZE);
            self.offset = start;
            self.search(0, start..end, None);

            let mut limit = usize::MAX;
            if end < self.text.len() {
                limit = 5;
            }

            for split in self.search.best[0].decode(self.offset).take(limit) {
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

            let key = (
                (start - self.offset) as u8,
                (split - self.offset) as u8,
                (end - self.offset) as u8,
            );

            let (suffix_score, suffix_splits) = match self.search.memo.get(&key) {
                Some((score, suffix_splits)) => (*score, *suffix_splits),
                None => {
                    let suffix_score = self.search(level + 1, split..end, Some(start..split));
                    let suffix_splits = self.search.best[level + 1];
                    self.search.memo.insert(key, (suffix_score, suffix_splits));
                    (suffix_score, suffix_splits)
                }
            };

            let score = prefix_score + suffix_score;
            if score > best {
                best = score;
                let new_splits = &mut self.search.best[level];
                new_splits.clear();
                new_splits.set(split - self.offset);
                *new_splits |= suffix_splits;
            }
        }

        best
    }
}

#[derive(Clone)]
pub struct Search {
    memo: HashMap<(u8, u8, u8), (f64, BitVec)>,
    best: [BitVec; SEGMENT_SIZE],
    result: Vec<String>,
}

impl Search {
    fn clear(&mut self) {
        self.memo.clear();
        for inner in self.best.iter_mut() {
            inner.clear();
        }
        self.result.clear();
    }
}

impl Default for Search {
    fn default() -> Self {
        Self {
            memo: HashMap::default(),
            best: [BitVec::default(); SEGMENT_SIZE],
            result: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Default)]
struct BitVec([u64; 4]);

impl BitVec {
    fn set(&mut self, mut bit: usize) {
        debug_assert!(bit < 256);
        let mut idx = 3;
        while bit > 63 {
            idx -= 1;
            bit -= 64;
        }
        self.0[idx] |= 1 << bit;
    }

    fn clear(&mut self) {
        self.0.iter_mut().for_each(|dst| {
            *dst = 0;
        });
    }

    fn decode(&self, offset: usize) -> Splits {
        Splits {
            vec: self.0,
            offset,
            idx: 3,
        }
    }
}

impl BitOrAssign for BitVec {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0
            .iter_mut()
            .zip(rhs.0.iter())
            .for_each(|(dst, src)| *dst |= *src);
    }
}

struct Splits {
    vec: [u64; 4],
    offset: usize,
    idx: usize,
}

impl Iterator for Splits {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        while self.idx > 0 && self.vec[self.idx] == 0 {
            self.idx -= 1;
        }

        let cur = self.vec[self.idx];
        if cur == 0 {
            return None;
        }

        let trailing = cur.trailing_zeros();
        let next = Some(self.offset + (3 - self.idx) * 64 + trailing as usize);
        self.vec[self.idx] -= 1 << trailing;
        next
    }
}

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
    use super::*;

    #[test]
    fn test_clean() {
        Ascii::new("Can't buy me love!").unwrap_err();
        let text = Ascii::new("cantbuymelove").unwrap();
        assert_eq!(&text[0..text.len()], "cantbuymelove");
    }

    #[test]
    fn bitvec() {
        let mut splits = BitVec::default();
        assert_eq!(splits.decode(0).collect::<Vec<_>>(), vec![]);

        splits.set(1);
        assert_eq!(splits.decode(0).collect::<Vec<_>>(), vec![1]);

        splits.set(5);
        assert_eq!(splits.decode(10).collect::<Vec<_>>(), vec![11, 15]);

        splits.set(64);
        assert_eq!(splits.decode(0).collect::<Vec<_>>(), vec![1, 5, 64]);

        splits.set(255);
        assert_eq!(splits.decode(0).collect::<Vec<_>>(), vec![1, 5, 64, 255]);

        let mut new = BitVec::default();
        new.set(3);
        new.set(16);
        new.set(128);

        splits |= new;
        assert_eq!(
            splits.decode(0).collect::<Vec<_>>(),
            vec![1, 3, 5, 16, 64, 128, 255]
        );
    }
}
