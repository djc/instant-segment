use std::path::PathBuf;

use once_cell::sync::Lazy;

use word_segmenters::Segmenter;

macro_rules! assert_segments {
    ($list:expr) => {
        assert_eq!(SEGMENTER.segment(&$list.join("")), $list);
    };
}

#[test]
fn test_segment_0() {
    assert_segments!(&["choose", "spain"]);
}

#[test]
fn test_segment_1() {
    assert_segments!(&["this", "is", "a", "test"]);
}

#[test]
fn test_segment_2() {
    assert_segments!(&[
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
    ]);
}

#[test]
fn test_segment_3() {
    assert_segments!(&["who", "represents"]);
}

#[test]
fn test_segment_4() {
    assert_segments!(&["experts", "exchange"]);
}

#[test]
fn test_segment_5() {
    assert_segments!(&["speed", "of", "art"]);
}

#[test]
fn test_segment_6() {
    assert_segments!(&["now", "is", "the", "time", "for", "all", "good"]);
}

#[test]
fn test_segment_7() {
    assert_segments!(&["it", "is", "a", "truth", "universally", "acknowledged"]);
}

#[test]
fn test_segment_8() {
    assert_segments!(&[
        "it", "was", "a", "bright", "cold", "day", "in", "april", "and", "the", "clocks", "were",
        "striking", "thirteen",
    ]);
}

#[test]
fn test_segment_9() {
    assert_segments!(&[
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
    ]);
}

#[test]
fn test_segment_10() {
    assert_segments!(&[
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
    ]);
}

#[test]
fn test_segment_11() {
    assert_segments!(vec![
        "in", "a", "hole", "in", "the", "ground", "there", "lived", "a", "hobbit", "not", "a",
        "nasty", "dirty", "wet", "hole", "filled", "with", "the", "ends", "of", "worms", "and",
        "an", "oozy", "smell", "nor", "yet", "a", "dry", "bare", "sandy", "hole", "with",
        "nothing", "in", "it", "to", "sit", "down", "on", "or", "to", "eat", "it", "was", "a",
        "hobbit", "hole", "and", "that", "means", "comfort"
    ]);
}

#[test]
fn test_segment_12() {
    assert_segments!(&[
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
    ]);
}

static SEGMENTER: Lazy<Segmenter> = Lazy::new(|| {
    Segmenter::from_dir(&PathBuf::from(format!(
        "{}/data",
        env!("CARGO_MANIFEST_DIR")
    )))
    .unwrap()
});
