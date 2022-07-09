use std::str::FromStr;

use pyo3::basic::CompareOp;
use pyo3::exceptions::{PyOSError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDateTime, PyType};

fn unwrap<T>(result: crate::Result<T>) -> PyResult<T> {
    match result {
        Ok(v) => Ok(v),
        Err(e) => to_python_result(e),
    }
}

fn to_python_result<T>(error: crate::Error) -> PyResult<T> {
    match error {
        crate::Error::ParseError(e) => Err(PyValueError::new_err(e.to_string())),
        crate::Error::OSError(e) => Err(PyOSError::new_err(e.to_string())),
    }
}

#[pyfunction]
fn add(x: usize, y: usize) -> usize {
    x + y
}

#[pyfunction]
fn slugify(title: &str) -> PyResult<String> {
    Ok(crate::slugify(title).to_string())
}

#[pyclass]
struct Id {
    _inner: crate::Id,
}

#[pymethods]
impl Id {
    #[new]
    fn new(s: &str) -> PyResult<Self> {
        let res = crate::Id::from_str(s);
        match res {
            Ok(i) => Ok(Id { _inner: i }),
            Err(e) => to_python_result(e),
        }
    }

    fn human_date(&self) -> String {
        self._inner.human_date()
    }

    #[classmethod]
    fn from_date(_cls: &PyType, date: &PyDateTime) -> PyResult<Self> {
        let pystring = date
            .str()
            .expect("datetime.__str__ method should not throw");

        let date_str = pystring
            .to_str()
            .expect("datetime.__str__ should not throw UnicodeDecode error");

        // date_str looks like this:
        //  2022-07-09 16:34:10.892856
        let year = &date_str[0..4];
        let month = &date_str[5..7];
        let day = &date_str[8..10];
        let hours = &date_str[11..13];
        let minutes = &date_str[14..16];
        let seconds = &date_str[17..19];
        let id = format!("{year}{month}{day}T{hours}{minutes}{seconds}");
        let id = unwrap(crate::Id::from_str(&id))?;
        Ok(Self { _inner: id })
    }

    fn __str__(slf: PyRef<'_, Self>) -> String {
        let res = slf._inner.as_str();
        res.to_string()
    }
}

#[pyclass]
struct Metadata {
    _inner: crate::Metadata,
}

#[pymethods]
impl Metadata {
    #[new]
    fn new(id: &Id, title: String, keywords: Vec<String>, extension: String) -> PyResult<Self> {
        let id = &id._inner;
        let metadata = crate::Metadata::new(id.clone(), title, keywords, extension);
        Ok(Self { _inner: metadata })
    }

    #[getter]
    fn id(&self) -> &str {
        self._inner.id()
    }

    #[getter]
    fn slug(&self) -> &str {
        self._inner.slug()
    }

    #[getter]
    fn title(&self) -> Option<&String> {
        self._inner.title()
    }

    #[getter]
    fn extension(&self) -> &str {
        self._inner.extension()
    }

    #[getter]
    fn keywords(&self) -> Vec<String> {
        self._inner.keywords().to_vec()
    }

    fn __str__(slf: PyRef<'_, Self>) -> String {
        let metadata = &slf._inner;
        format!("{metadata:?}")
    }
}

#[pyclass]
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct FrontMatter {
    _inner: crate::FrontMatter,
}

#[pymethods]
impl FrontMatter {
    #[getter]
    fn title(&self) -> Option<&String> {
        self._inner.title()
    }

    #[getter]
    fn keywords(&self) -> Vec<String> {
        self._inner.keywords()
    }

    fn dump(&self) -> String {
        self._inner.dump()
    }

    #[classmethod]
    fn parse(_cls: &PyType, front_matter: &str) -> PyResult<Self> {
        let inner = unwrap(crate::FrontMatter::parse(front_matter))?;
        Ok(Self { _inner: inner })
    }

    fn __richcmp__(&self, other: &FrontMatter, op: CompareOp) -> PyResult<bool> {
        match op {
            CompareOp::Eq => Ok(other == self),
            CompareOp::Lt => Ok(other < self),
            CompareOp::Le => Ok(other <= self),
            CompareOp::Ne => Ok(other != self),
            CompareOp::Gt => Ok(other > self),
            CompareOp::Ge => Ok(other >= self),
        }
    }

    fn __str__(slf: PyRef<'_, Self>) -> String {
        let frontmatter = &slf._inner;
        format!("{frontmatter:?}")
    }
}

#[pymodule]
fn denote(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(add, m)?)?;
    m.add_function(wrap_pyfunction!(slugify, m)?)?;
    m.add_class::<Id>()?;
    m.add_class::<Metadata>()?;
    m.add_class::<FrontMatter>()?;
    Ok(())
}
