use crate::{Search, Segmenter};

/// Run a segmenter against the built-in test cases
pub fn run(segmenter: &Segmenter) {
    let mut search = Search::default();
    for test in TEST_CASES.iter().copied() {
        assert_segments(test, &mut search, segmenter);
    }
    assert_segments(FAIL, &mut search, segmenter);
}

pub fn assert_segments(s: &[&str], search: &mut Search, segmenter: &Segmenter) {
    segmenter.segment(&s.join(""), search).unwrap();
    let cmp = search.split().collect::<Vec<_>>();
    assert_eq!(cmp, s);
}

pub fn check_segments(s: &[&str], search: &mut Search, segmenter: &Segmenter) -> bool {
    match segmenter.segment(&s.join(""), search) {
        Ok(()) => s == search.split().collect::<Vec<_>>(),
        Err(_) => false,
    }
}

/// Built-in test cases
///
/// These are exposed so that you can test with different data sources.
pub const TEST_CASES: &[&[&str]] = &[
    &["choose", "spain"],
    &["this", "is", "a", "test"],
    &["who", "represents"],
    &["experts", "exchange"],
    &["speed", "of", "art"],
    &["now", "is", "the", "time", "for", "all", "good"],
    &["it", "is", "a", "truth", "universally", "acknowledged"],
    &[
        "it", "was", "a", "bright", "cold", "day", "in", "april", "and", "the", "clocks", "were",
        "striking", "thirteen",
    ],
    &[
        "when",
        "in",
        "the",
        "course",
        "of",
        "human",
        "events",
        "it",
        "becomes",
        "necessary",
    ],
    &[
        "it",
        "was",
        "the",
        "best",
        "of",
        "times",
        "it",
        "was",
        "the",
        "worst",
        "of",
        "times",
        "it",
        "was",
        "the",
        "age",
        "of",
        "wisdom",
        "it",
        "was",
        "the",
        "age",
        "of",
        "foolishness",
    ],
    &[
        "as",
        "gregor",
        "samsa",
        "awoke",
        "one",
        "morning",
        "from",
        "uneasy",
        "dreams",
        "he",
        "found",
        "himself",
        "transformed",
        "in",
        "his",
        "bed",
        "into",
        "a",
        "gigantic",
        "insect",
    ],
    &[
        "in", "a", "hole", "in", "the", "ground", "there", "lived", "a", "hobbit", "not", "a",
        "nasty", "dirty", "wet", "hole", "filled", "with", "the", "ends", "of", "worms", "and",
        "an", "oozy", "smell", "nor", "yet", "a", "dry", "bare", "sandy", "hole", "with",
        "nothing", "in", "it", "to", "sit", "down", "on", "or", "to", "eat", "it", "was", "a",
        "hobbit", "hole", "and", "that", "means", "comfort",
    ],
];

/// Incorrectly segmented, since the test data doesn't contain "unregarded"
const FAIL: &[&str] = &[
    "far",
    "out",
    "in",
    "the",
    "uncharted",
    "backwaters",
    "of",
    "the",
    "unfashionable",
    "end",
    "of",
    "the",
    "western",
    "spiral",
    "arm",
    "of",
    "the",
    "galaxy",
    "lies",
    "a",
    "small",
    "un",
    "regarded",
    "yellow",
    "sun",
];
