use pyo3::prelude::*;

#[pyfunction]
fn add(x: usize, y: usize) -> usize {
    x + y
}

#[pymodule]
fn denote(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(add, m)?)?;
    Ok(())
}
