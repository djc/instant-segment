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

#[pyclass]
struct Segmenter {
    inner: instant_segment::Segmenter,
}

#[pymethods]
impl Segmenter {
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
