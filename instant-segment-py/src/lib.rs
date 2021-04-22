use std::fs::File;
use std::io::{BufReader, BufWriter};

use pyo3::exceptions::PyValueError;
use pyo3::proc_macro::{pyclass, pymethods, pymodule, pyproto};
use pyo3::types::{PyIterator, PyModule};
use pyo3::{PyErr, PyIterProtocol, PyRef, PyRefMut, PyResult, Python};
use smartstring::alias::String as SmartString;

#[pymodule]
fn instant_segment(_: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Search>()?;
    m.add_class::<Segmenter>()?;
    Ok(())
}

/// Segmenter holding the word lists
#[pyclass]
struct Segmenter {
    inner: instant_segment::Segmenter,
}

#[pymethods]
impl Segmenter {
    /// Build a segmenter from `unigrams` and `bigrams` iterators
    ///
    /// The `unigrams` iterator should yield `(str, float)` items, while the `bigrams`
    /// iterator should yield `((str, str), float)` items.
    #[new]
    fn new(unigrams: &PyIterator, bigrams: &PyIterator) -> PyResult<Self> {
        let unigrams = unigrams
            .map(|item| {
                let item = item?;
                let key = item.get_item(0)?.extract::<&str>()?;
                let val = item.get_item(1)?.extract::<f64>()?;
                Ok((SmartString::from(key), val))
            })
            .collect::<Result<HashMap<_, _>, PyErr>>()?;

        let bigrams = bigrams
            .map(|item| {
                let item = item?;

                let key = item.get_item(0)?;
                let first = key.get_item(0)?.extract::<&str>()?;
                let second = key.get_item(1)?.extract::<&str>()?;

                let val = item.get_item(1)?.extract::<f64>()?;
                Ok(((SmartString::from(first), SmartString::from(second)), val))
            })
            .collect::<Result<HashMap<_, _>, PyErr>>()?;

        Ok(Self {
            inner: instant_segment::Segmenter::from_maps(unigrams, bigrams),
        })
    }

    /// Load a segmenter from the given file name
    #[staticmethod]
    fn load(fname: &str) -> PyResult<Self> {
        let hnsw = bincode::deserialize_from::<_, instant_segment::Segmenter>(
            BufReader::with_capacity(32 * 1024 * 1024, File::open(fname)?),
        )
        .map_err(|e| PyValueError::new_err(format!("deserialization error: {:?}", e)))?;
        Ok(Self { inner: hnsw })
    }

    /// Dump the segmenter to the given file name
    fn dump(&self, fname: &str) -> PyResult<()> {
        let f = BufWriter::with_capacity(32 * 1024 * 1024, File::create(fname)?);
        bincode::serialize_into(f, &self.inner)
            .map_err(|e| PyValueError::new_err(format!("serialization error: {:?}", e)))?;
        Ok(())
    }

    /// Segment the given str `s`
    ///
    /// The `search` object contains buffers used for searching. When the search completes,
    /// iterate over the `Search` to get the resulting words.
    ///
    /// For best performance, reusing `Search` objects is recommended.
    fn segment(&self, s: &str, search: &mut Search) -> PyResult<()> {
        match self.inner.segment(s, &mut search.inner) {
            Ok(_) => {
                search.cur = Some(0);
                Ok(())
            }
            Err(_) => Err(PyValueError::new_err(
                "only lowercase ASCII letters allowed",
            )),
        }
    }

    /// Returns the sentence's score
    ///
    /// Returns the relative probability for the given sentence in the the corpus represented by
    /// this `Segmenter`. Will return `None` iff given an empty iterator argument.
    fn sentence_score(&self, words: &PyIterator) -> PyResult<Option<f64>> {
        let words = words
            .map(|s| s?.extract::<&str>())
            .collect::<Result<Vec<_>, _>>()?;
        Ok(self.inner.sentence_score(words.into_iter()))
    }
}

/// Search buffer and result set
#[pyclass]
struct Search {
    inner: instant_segment::Search,
    cur: Option<usize>,
}

#[pymethods]
impl Search {
    /// Initialize an empty search buffer
    #[new]
    fn new() -> Self {
        Self {
            inner: instant_segment::Search::default(),
            cur: None,
        }
    }
}

#[pyproto]
impl PyIterProtocol for Search {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    /// Return the next closest point
    fn __next__(mut slf: PyRefMut<Self>) -> Option<String> {
        let idx = match &slf.cur {
            Some(idx) => *idx,
            None => return None,
        };

        let word = match slf.inner.get(idx) {
            Some(word) => String::from(word),
            None => {
                slf.cur = None;
                return None;
            }
        };

        slf.cur = Some(idx + 1);
        Some(word)
    }
}

type HashMap<K, V> = std::collections::HashMap<K, V, ahash::RandomState>;
