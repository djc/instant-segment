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
    // Trie containing unigrams and, for eah unigram, bigram scores.
    trie: Trie,
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
        let mut builder = TrieBuilder::new();
        let mut uni_total = 0.0;
        for (word, uni) in unigrams {
            builder.insert(
                &word,
                WordData {
                    uni,
                    bi_scores: HashMap::default(),
                },
            );
            uni_total += uni;
        }

        let mut bi_total = 0.0;
        for ((word1, word2), bi) in bigrams {
            let Some(wd) = builder.lookup_mut(&word2) else {
                // We throw away bigrams for which we do not have a unigram for
                // the second word. This case shouldn't ever happen on
                // real-world data, and in fact, it never happens on the word
                // count lists shipped with this crate.
                continue;
            };
            wd.bi_scores.insert(word1, bi);
            bi_total += bi;
        }

        // Now convert the counts to the values we actually want,
        // namely logarithms of relative frequencies
        for wd in &mut builder.words {
            wd.uni = (wd.uni / uni_total).log10();
            for bi in wd.bi_scores.values_mut() {
                *bi = (*bi / bi_total).log10();
            }
        }

        Self {
            uni_total_log10: uni_total.log10(),
            trie: builder.build(),
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
    ) -> Result<Segments<'a>, InvalidCharacter> {
        let state = SegmentState::new(Ascii::new(input)?, self, search);
        let score = match input {
            "" => 0.0,
            _ => state.run(),
        };

        Ok(Segments {
            iter: search.result.iter(),
            score,
        })
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
        match self.trie.lookup(word) {
            Some(wd) => self.score_found(wd.uni, &wd.bi_scores, previous),
            None => self.score_not_found(word.len()),
        }
    }

    /// Score for words found in the dictionary
    #[inline]
    fn score_found(
        &self,
        uni: f64,
        bi_scores: &HashMap<String, f64>,
        previous: Option<&str>,
    ) -> f64 {
        if let Some(prev) = previous {
            if let Some(bi) = bi_scores.get(prev) {
                if let Some(prev_wd) = self.trie.lookup(prev) {
                    // Conditional probability of the word given the previous
                    // word. The technical name is "stupid backoff" and it's
                    // not a probability distribution but it works well in practice.
                    return bi - prev_wd.uni;
                }
            }
        }
        uni
    }

    /// Score for non-words
    #[inline]
    fn score_not_found(&self, word_len: usize) -> f64 {
        // Penalize words not found in the unigrams according
        // to their length, a crucial heuristic.
        1.0 - self.uni_total_log10 - word_len as f64
    }

    /// Customize the word length `limit`
    pub fn set_limit(&mut self, limit: usize) {
        self.limit = limit;
    }
}

pub struct Segments<'a> {
    iter: std::slice::Iter<'a, String>,
    score: f64,
}

impl Segments<'_> {
    /// Returns the score of the segmented text
    pub fn score(&self) -> f64 {
        self.score
    }
}

impl<'a> Iterator for Segments<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|v| v.as_str())
    }
}

impl ExactSizeIterator for Segments<'_> {
    fn len(&self) -> usize {
        self.iter.len()
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
        search.candidates.resize(
            text.len(),
            Candidate {
                len: 1,
                score: f64::NEG_INFINITY,
            },
        );
        Self { data, text, search }
    }

    fn run(self) -> f64 {
        let n = self.text.len();

        for split in 0..n {
            let (prev, prev_score) = match split {
                0 => (None, 0.0),
                _ => {
                    let prefix = self.search.candidates[split - 1];
                    let word = &self.text[split - prefix.len..split];
                    (Some(word), prefix.score)
                }
            };

            let end_limit = (split + self.data.limit).min(n);
            let mut trie_node = 0u32;

            for end in (split + 1)..=end_limit {
                let c = self.text.0[end - 1];

                let advanced = match c {
                    b'a'..=b'z' => self.data.trie.child(trie_node, c - b'a'),
                    _ => None,
                };

                let word_len = end - split;
                let score = match advanced {
                    Some(next) => {
                        trie_node = next;
                        match self.data.trie.word_data(next) {
                            Some(wd) => self.data.score_found(wd.uni, &wd.bi_scores, prev),
                            None => self.data.score_not_found(word_len),
                        }
                    }
                    None => self.data.score_not_found(word_len),
                } + prev_score;

                let cur = &mut self.search.candidates[end - 1];
                if score > cur.score {
                    cur.len = word_len;
                    cur.score = score;
                }

                if advanced.is_none() {
                    break;
                }
            }
        }

        let mut end = self.text.len();
        let mut best = self.search.candidates[end - 1];
        let score = best.score;
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
        score
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

impl Index<Range<usize>> for Ascii<'_> {
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

const TRIE_NULL: u32 = u32::MAX;

#[cfg_attr(feature = "with-serde", derive(Deserialize, Serialize))]
struct Trie {
    nodes: Vec<TrieNode>,
    words: Vec<WordData>,
}

#[cfg_attr(feature = "with-serde", derive(Deserialize, Serialize))]
struct TrieNode {
    children: [u32; 26],

    // index into Trie::words
    word: u32,
}

#[cfg_attr(feature = "with-serde", derive(Deserialize, Serialize))]
struct WordData {
    uni: f64,
    bi_scores: HashMap<String, f64>,
}

impl Trie {
    // Returns an index into `self.nodes`, if the child exists.
    #[inline]
    fn child(&self, node: u32, c: u8) -> Option<u32> {
        let idx = self.nodes[node as usize].children[c as usize];
        if idx == TRIE_NULL {
            None
        } else {
            Some(idx)
        }
    }

    #[inline]
    fn word_data(&self, node: u32) -> Option<&WordData> {
        let idx = self.nodes[node as usize].word;
        if idx == TRIE_NULL {
            None
        } else {
            Some(&self.words[idx as usize])
        }
    }

    fn lookup(&self, word: &str) -> Option<&WordData> {
        let mut node = 0u32;
        for &b in word.as_bytes() {
            let c = b.wrapping_sub(b'a');
            if c >= 26 {
                return None;
            }
            node = self.child(node, c)?;
        }
        self.word_data(node)
    }
}

struct TrieBuilder {
    nodes: Vec<BuilderNode>,
    words: Vec<WordData>,
}

struct BuilderNode {
    children: [u32; 26],
    word: u32,
}

impl TrieBuilder {
    fn new() -> Self {
        Self {
            nodes: vec![BuilderNode {
                children: [TRIE_NULL; 26],
                word: TRIE_NULL,
            }],
            words: Vec::new(),
        }
    }

    fn insert(&mut self, word: &str, data: WordData) {
        let mut node = 0usize;
        for &b in word.as_bytes() {
            let c = b.wrapping_sub(b'a');
            if c >= 26 {
                return;
            }
            let c = c as usize;
            if self.nodes[node].children[c] == TRIE_NULL {
                self.nodes[node].children[c] = self.nodes.len() as u32;
                self.nodes.push(BuilderNode {
                    children: [TRIE_NULL; 26],
                    word: TRIE_NULL,
                });
            }
            node = self.nodes[node].children[c] as usize;
        }
        let idx = self.words.len() as u32;
        self.words.push(data);
        self.nodes[node].word = idx;
    }

    fn lookup_mut(&mut self, word: &str) -> Option<&mut WordData> {
        let mut node = 0usize;
        for &b in word.as_bytes() {
            let c = b.wrapping_sub(b'a');
            if c >= 26 {
                return None;
            }
            let c = c as usize;
            let next = self.nodes[node].children[c];
            if next == TRIE_NULL {
                return None;
            }
            node = next as usize;
        }
        let idx = self.nodes[node].word;
        if idx == TRIE_NULL {
            None
        } else {
            Some(&mut self.words[idx as usize])
        }
    }

    fn build(self) -> Trie {
        let nodes = self
            .nodes
            .into_iter()
            .map(|n| TrieNode {
                children: n.children,
                word: n.word,
            })
            .collect();
        Trie {
            nodes,
            words: self.words,
        }
    }
}

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
