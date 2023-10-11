use pyo3::prelude::*;

mod interop;
pub mod solver;
pub mod types;

#[pymodule]
#[pyo3(name = "native")]
fn tetra(_py: Python, m: &PyModule) -> PyResult<()> {
    // m.add_function(wrap_pyfunction!(rust_native_function, m)?)?;

    m.add_class::<types::Field>()?;
    m.add_class::<types::Fumen>()?;
    m.add_class::<types::Piece>()?;
    m.add_class::<types::Solution>()?;
    m.add_class::<solver::Srs4lSolver>()?;

    Ok(())
}
