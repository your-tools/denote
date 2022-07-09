use std::path::PathBuf;
use std::str::FromStr;

use pyo3::basic::CompareOp;
use pyo3::exceptions::{PyOSError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDateTime, PyModule, PyTuple, PyType};

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

fn path_buf_to_pathlib(path_buf: PathBuf) -> PyResult<PyObject> {
    Python::with_gil(|py| {
        let pathlib = PyModule::import(py, "pathlib")?;
        let path_class = pathlib.getattr("Path")?;
        let args = PyTuple::new(py, &[path_buf.to_string_lossy().to_string()]);
        let res = path_class.call1(args)?;
        Ok(res.into())
    })
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

    #[getter]
    fn relative_path(&self) -> String {
        self._inner.relative_path().to_string_lossy().to_string()
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

#[pyclass]
struct Note {
    _inner: crate::Note,
}

#[pymethods]
impl Note {
    #[new]
    fn new(metadata: &Metadata, text: &str) -> Self {
        let inner_note = crate::Note {
            metadata: metadata._inner.clone(),
            text: text.to_string(),
        };
        Note { _inner: inner_note }
    }

    #[getter]
    fn relative_path(&self) -> String {
        self._inner.relative_path().to_string_lossy().to_string()
    }

    #[getter]
    fn front_matter(&self) -> FrontMatter {
        FrontMatter {
            _inner: self._inner.front_matter().clone(),
        }
    }

    #[getter]
    fn metadata(&self) -> Metadata {
        Metadata {
            _inner: self._inner.metadata().clone(),
        }
    }

    pub fn dump(&self) -> String {
        self._inner.dump()
    }

    fn __str__(slf: PyRef<'_, Self>) -> String {
        let inner = &slf._inner;
        format!("{inner:?}")
    }
}

#[pyclass]
struct NotesRepository {
    _inner: crate::NotesRepository,
}

#[pymethods]
impl NotesRepository {
    #[classmethod]
    fn open(_cls: &PyType, base_path: &PyAny) -> PyResult<Self> {
        // If base_path is a string or a pathlib.Path instance,
        // as_str will have the correct value.
        // Otherwise, all bets are off, but let the caller deal with that :)
        let as_str = base_path.to_string();
        let path = match PathBuf::from_str(&as_str) {
            Ok(p) => p,
            Err(e) => {
                return Err(PyValueError::new_err(e.to_string()));
            }
        };
        let inner = unwrap(crate::NotesRepository::open(&path))?;
        Ok(NotesRepository { _inner: inner })
    }

    #[getter]
    fn base_path(&self) -> String {
        self._inner.base_path().to_string_lossy().to_string()
    }

    fn import_from_markdown(&self, markdown_path: &PyAny) -> PyResult<PyObject> {
        let as_path = PathBuf::from_str(&markdown_path.to_string())?;
        let saved_path = unwrap(self._inner.import_from_markdown(&as_path))?;
        path_buf_to_pathlib(saved_path)
    }

    fn on_update(&self, relative_path: &PyAny) -> PyResult<PyObject> {
        let as_path = PathBuf::from_str(&relative_path.to_string())?;
        let new_path = unwrap(self._inner.update(&as_path))?;
        path_buf_to_pathlib(new_path)
    }

    fn load(&self, relative_path: &PyAny) -> PyResult<Note> {
        let as_path = PathBuf::from_str(&relative_path.to_string())?;
        let note = unwrap(self._inner.load(&as_path))?;
        Ok(Note { _inner: note })
    }

    fn save(&self, note: &Note) -> PyResult<PyObject> {
        let path = unwrap(self._inner.save(&note._inner))?;
        path_buf_to_pathlib(path)
    }

    fn __str__(slf: PyRef<'_, Self>) -> String {
        let inner = &slf._inner;
        format!("{inner:?}")
    }
}

#[pymodule]
fn denote(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(slugify, m)?)?;
    m.add_class::<Id>()?;
    m.add_class::<Metadata>()?;
    m.add_class::<FrontMatter>()?;
    m.add_class::<Note>()?;
    m.add_class::<NotesRepository>()?;
    Ok(())
}
