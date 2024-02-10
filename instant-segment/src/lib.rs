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
    // Maps a word to both its unigram score, as well has a nested HashMap in
    // which the bigram score can be looked up using the previous word. Scores
    // are base-10 logarithms of relative word frequencies
    scores: HashMap<String, (f64, HashMap<String, f64>)>,
    // Base-10 logarithm of the total count of unigrams
    uni_total_log10: f64,
    limit: usize,
}

impl Segmenter {
    /// Create `Segmenter` from the given unigram and bigram counts.
    ///
    /// Note: the `String` types used in this API are defined in the `smartstring` crate. Any
    /// `&str` or `String` can be converted into the `String` used here by calling `into()` on it.
    pub fn new<U, B>(unigrams: U, bigrams: B) -> Self
    where
        U: IntoIterator<Item = (String, f64)>,
        B: IntoIterator<Item = ((String, String), f64)>,
    {
        // Initially, `scores` contains the original unigram and bigram counts
        let mut scores = HashMap::default();
        let mut uni_total = 0.0;
        for (word, uni) in unigrams {
            scores.insert(word, (uni, HashMap::default()));
            uni_total += uni;
        }
        let mut bi_total = 0.0;
        for ((word1, word2), bi) in bigrams {
            let Some((_, bi_scores)) = scores.get_mut(&word2) else {
                // We throw away bigrams for which we do not have a unigram for
                // the second word. This case shouldn't ever happen on
                // real-world data, and in fact, it never happens on the word
                // count lists shipped with this crate.
                continue;
            };
            bi_scores.insert(word1, bi);
            bi_total += bi;
        }

        // Now convert the counts in `scores` to the values we actually want,
        // namely logarithms of relative frequencies
        for (uni, bi_scores) in scores.values_mut() {
            *uni = (*uni / uni_total).log10();
            for bi in bi_scores.values_mut() {
                *bi = (*bi / bi_total).log10();
            }
        }

        Self {
            uni_total_log10: uni_total.log10(),
            scores,
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
        let state = SegmentState::new(Ascii::new(input)?, self, search);
        if !input.is_empty() {
            state.run();
        }

        Ok(search.result.iter().map(|v| v.as_str()))
    }

    /// Returns the sentence's score
    ///
    /// Returns the relative probability for the given sentence in the the corpus represented by
    /// this `Segmenter`. Will return `None` iff given an empty iterator argument.
    pub fn score_sentence<'a>(&self, mut words: impl Iterator<Item = &'a str>) -> Option<f64> {
        let mut prev = words.next()?;
        let mut score = self.score(prev, None);
        for word in words {
            score += self.score(word, Some(prev));
            prev = word;
        }
        Some(score)
    }

    fn score(&self, word: &str, previous: Option<&str>) -> f64 {
        let (uni, bi_scores) = match self.scores.get(word) {
            Some((uni, bi_scores)) => (uni, bi_scores),
            // Penalize words not found in the unigrams according
            // to their length, a crucial heuristic.
            //
            // In the original presentation non-words are scored as
            //
            //    (1.0 - self.uni_total_log10 - word_len)
            //
            // However in practice this seems to under-penalize long non-words.  The intuition
            // behind the variation used here is that it applies this penalty once for each word
            // there "should" have been in the non-word's place.
            //
            // See <https://github.com/instant-labs/instant-segment/issues/53>.
            None => {
                let word_len = word.len() as f64;
                let word_count = word_len / 5.0;
                return (1.0 - self.uni_total_log10 - word_len) * word_count;
            }
        };

        if let Some(prev) = previous {
            if let Some(bi) = bi_scores.get(prev) {
                if let Some((uni_prev, _)) = self.scores.get(prev) {
                    // Conditional probability of the word given the previous
                    // word. The technical name is "stupid backoff" and it's
                    // not a probability distribution but it works well in practice.
                    return bi - uni_prev;
                }
            }
        }

        *uni
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
}

impl<'a> SegmentState<'a> {
    fn new(text: Ascii<'a>, data: &'a Segmenter, search: &'a mut Search) -> Self {
        search.clear();
        Self { data, text, search }
    }

    fn run(self) {
        for end in 1..=self.text.len() {
            let start = end.saturating_sub(self.data.limit);
            for split in start..end {
                let (prev, prev_score) = match split {
                    0 => (None, 0.0),
                    _ => {
                        let prefix = self.search.candidates[split - 1];
                        let word = &self.text[split - prefix.len..split];
                        (Some(word), prefix.score)
                    }
                };

                let word = &self.text[split..end];
                let score = self.data.score(word, prev) + prev_score;
                match self.search.candidates.get_mut(end - 1) {
                    Some(cur) if cur.score < score => {
                        cur.len = end - split;
                        cur.score = score;
                    }
                    None => self.search.candidates.push(Candidate {
                        len: end - split,
                        score,
                    }),
                    _ => {}
                }
            }
        }

        let mut end = self.text.len();
        let mut best = self.search.candidates[end - 1];
        loop {
            let word = &self.text[end - best.len..end];
            self.search.result.push(word.into());

            end -= best.len;
            if end == 0 {
                break;
            }

            best = self.search.candidates[end - 1];
        }

        self.search.result.reverse();
    }
}

/// Search state for a [`Segmenter`]
#[derive(Clone, Default)]
pub struct Search {
    candidates: Vec<Candidate>,
    result: Vec<String>,
}

impl Search {
    fn clear(&mut self) {
        self.candidates.clear();
        self.result.clear();
    }

    #[doc(hidden)]
    pub fn get(&self, idx: usize) -> Option<&str> {
        self.result.get(idx).map(|v| v.as_str())
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Candidate {
    len: usize,
    score: f64,
}

#[derive(Debug)]
struct Ascii<'a>(&'a [u8]);

impl<'a> Ascii<'a> {
    fn new(s: &'a str) -> Result<Self, InvalidCharacter> {
        let bytes = s.as_bytes();

        let valid = bytes
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit());

        match valid {
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

/// Error returned by [`Segmenter::segment`] when given an invalid character
#[derive(Debug)]
pub struct InvalidCharacter;

impl std::error::Error for InvalidCharacter {}

impl std::fmt::Display for InvalidCharacter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid character")
    }
}

type HashMap<K, V> = rustc_hash::FxHashMap<K, V>;

const DEFAULT_LIMIT: usize = 24;

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_clean() {
        Ascii::new("Can't buy me love!").unwrap_err();
        let text = Ascii::new("cantbuymelove").unwrap();
        assert_eq!(&text[0..text.len()], "cantbuymelove");
        let text_with_numbers = Ascii::new("c4ntbuym3l0v3").unwrap();
        assert_eq!(
            &text_with_numbers[0..text_with_numbers.len()],
            "c4ntbuym3l0v3"
        );
    }
}
