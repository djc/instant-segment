![Cover logo](./cover.svg)

# Instant Segment: fast English word segmentation in Rust

[![Documentation](https://docs.rs/instant-segment/badge.svg)](https://docs.rs/instant-segment/)
[![Crates.io](https://img.shields.io/crates/v/instant-segment.svg)](https://crates.io/crates/instant-segment)
[![PyPI](https://img.shields.io/pypi/v/instant-segment)](https://pypi.org/project/instant-segment/)
[![Build status](https://github.com/InstantDomainSearch/instant-segment/workflows/CI/badge.svg)](https://github.com/InstantDomainSearch/instant-segment/actions?query=workflow%3ACI)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE-APACHE)

Instant Segment is a fast Apache-2.0 library for English word segmentation. It
is based on the Python [wordsegment][python] project written by Grant Jenks,
which is in turn based on code from Peter Norvig's chapter [Natural Language
Corpus Data][chapter] from the book [Beautiful Data][book] (Segaran and
Hammerbacher, 2009).

The data files in this repository are derived from the [Google Web Trillion Word
Corpus][corpus], as described by Thorsten Brants and Alex Franz, and
[distributed][distributed] by the Linguistic Data Consortium. Note that this
data **"may only be used for linguistic education and research"**, so for any
other usage you should acquire a different data set.

For the microbenchmark included in this repository, Instant Segment is ~100x
faster than the Python implementation. Further optimizations are planned -- see
the [issues][issues]. The API has been carefully constructed so that multiple
segmentations can share the underlying state to allow parallel usage.

## How it works

Instant Segment works by segmenting a string into words by selecting the splits
with the highest probability given a corpus of words and their occurrences.

For instance, provided that `choose` and `spain` occur more frequently than
`chooses` and `pain`, and that the pair `choose spain` occurs more frequently
than `chooses pain`, Instant Segment can help identify the domain
`choosespain.com` as `ChooseSpain.com` which more likely matches user intent.

We use this technique at
[Instant Domain Search](https://instantdomainsearch.com/search/sale?q=choosespain)
to help our users find relevant domains.

## Using the library

### Python **(>= 3.9)**

```sh
pip install instant-segment
```

### Rust

```toml
[dependencies]
instant-segment = "0.8.1"
```

### Examples

The following examples expect `unigrams` and `bigrams` to exist. See the
examples ([Rust](./instant-segment/examples/contrived.rs),
[Python](./instant-segment-py/examples/contrived.py)) to see how to construct
these objects.

```python
import instant_segment

segmenter = instant_segment.Segmenter(unigrams, bigrams)
search = instant_segment.Search()
segmenter.segment("instantdomainsearch", search)
print([word for word in search])

--> ['instant', 'domain', 'search']
```

```rust
use instant_segment::{Search, Segmenter};
use std::collections::HashMap;

let segmenter = Segmenter::from_maps(unigrams, bigrams);
let mut search = Search::default();
let words = segmenter
    .segment("instantdomainsearch", &mut search)
    .unwrap();
println!("{:?}", words.collect::<Vec<&str>>())

--> ["instant", "domain", "search"]
```

Check out the tests for more thorough examples:
[Rust](./instant-segment/src/test_cases.rs),
[Python](./instant-segment-py/test/test.py)

## Testing

To run the tests run the following:

```
cargo t -p instant-segment --all-features
```

You can also test the Python bindings with:

```
make test-python
```

[python]: https://github.com/grantjenks/python-wordsegment
[chapter]: http://norvig.com/ngrams/
[book]: http://oreilly.com/catalog/9780596157111/
[corpus]:
  http://googleresearch.blogspot.com/2006/08/all-our-n-gram-are-belong-to-you.html
[distributed]: https://catalog.ldc.upenn.edu/LDC2006T13
[issues]: https://github.com/InstantDomainSearch/instant-segment/issues
