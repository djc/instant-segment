![Cover logo](./cover.svg)

# Instant Segment: fast English word segmentation in Rust

[![Documentation](https://docs.rs/instant-segment/badge.svg)](https://docs.rs/instant-segment/)
[![Crates.io](https://img.shields.io/crates/v/instant-segment.svg)](https://crates.io/crates/instant-segment)
![PyPI](https://img.shields.io/pypi/v/instant-segment)
[![Build status](https://github.com/InstantDomainSearch/instant-segment/workflows/CI/badge.svg)](https://github.com/InstantDomainSearch/instant-segment/actions?query=workflow%3ACI)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE-APACHE)

```python
segmenter = instant_segment.Segmenter(unigrams(), bigrams())
search = instant_segment.Search()
segmenter.segment("instantdomainsearch", search)
print([word for word in search])

--> ['instant', 'domain', 'search']
```

```rust
let segmenter = Segmenter::from_maps(unigrams, bigrams);
let mut search = Search::default();
let words = segmenter
    .segment("instantdomainsearch", &mut search)
    .unwrap();
println!("{:?}", words.collect::<Vec<&str>>())

--> ["instant", "domain", "search"]
```

Instant Segment is a fast Apache-2.0 library for English word segmentation.
It is based on the Python [wordsegment][python] project written by Grant Jenkins,
which is in turn based on code from Peter Norvig's chapter [Natural Language
Corpus Data][chapter] from the book [Beautiful Data][book] (Segaran and Hammerbacher, 2009).

The data files in this repository are derived from the [Google Web Trillion Word
Corpus][corpus], as described by Thorsten Brants and Alex Franz, and [distributed][distributed] by the
Linguistic Data Consortium. Note that this data **"may only be used for linguistic
education and research"**, so for any other usage you should acquire a different data set.

For the microbenchmark included in this repository, Instant Segment is ~17x faster than
the Python implementation. Further optimizations are planned -- see the [issues][issues].
The API has been carefully constructed so that multiple segmentations can share
the underlying state to allow parallel usage.

## Installing

### Python **(>= 3.9)**

```sh
pip install instant-segment
```

### Rust

```toml
[dependencies]
instant-segment = "*"
```

## Using

Instant Segment works by segmenting a string into words by selecting the splits with the highest probability given a vocabulary of words and their occurances.

For instance, provided that `choose` and `spain` occur more frequently than `chooses` and `pain`, Instant Segment can help you split the string `choosespain.com` into [`ChooseSpain.com`](https://instantdomainsearch.com/search/sale?q=choosespain) which more likely matches user intent.

```python
import instant_segment


def main():
    unigrams = []
    unigrams.append(("choose", 50))
    unigrams.append(("chooses", 10))
    unigrams.append(("spain", 50))
    unigrams.append(("pain", 10))

    bigrams = []
    bigrams.append((("choose", "spain"), 10))
    bigrams.append((("chooses", "pain"), 10))

    segmenter = instant_segment.Segmenter(iter(unigrams), iter(bigrams))
    search = instant_segment.Search()
    segmenter.segment("choosespain", search)
    print([word for word in search])


if __name__ == "__main__":
    main()

```

```rust
use instant_segment::{Search, Segmenter};
use std::collections::HashMap;

fn main() {
    let mut unigrams = HashMap::default();

    unigrams.insert("choose".into(), 50 as f64);
    unigrams.insert("chooses".into(), 10 as f64);

    unigrams.insert("spain".into(), 50 as f64);
    unigrams.insert("pain".into(), 10 as f64);

    let mut bigrams = HashMap::default();

    bigrams.insert(("choose".into(), "spain".into()), 10 as f64);
    bigrams.insert(("chooses".into(), "pain".into()), 10 as f64);

    let segmenter = Segmenter::from_maps(unigrams, bigrams);
    let mut search = Search::default();

    let words = segmenter.segment("choosespain", &mut search).unwrap();

    println!("{:?}", words.collect::<Vec<&str>>())
}
```

```
['choose', 'spain']
```

Play with the examples above to see that different numbers of occurances will influence the results

The example above is succinct but, in practice, you will want to load these words and occurances from a corpus of data like the ones we provide [here](./data). Check out [the](./instant-segment/instant-segment-py/test/test.py) [tests](./instant-segment/instant-segment/src/test_data.rs) to see examples of how you might do that.

[python]: https://github.com/grantjenks/python-wordsegment
[chapter]: http://norvig.com/ngrams/
[book]: http://oreilly.com/catalog/9780596157111/
[corpus]: http://googleresearch.blogspot.com/2006/08/all-our-n-gram-are-belong-to-you.html
[distributed]: https://catalog.ldc.upenn.edu/LDC2006T13
[issues]: https://github.com/InstantDomainSearch/instant-segment/issues
