use pyo3::{exceptions::PyValueError, prelude::*, types::PyString};

use crate::types::{Orientation, Shape};

impl IntoPy<Py<PyAny>> for Shape {
    fn into_py(self, py: Python<'_>) -> Py<PyAny> {
        PyString::new(py, self.into()).into_py(py)
    }
}

impl IntoPy<Py<PyAny>> for Orientation {
    fn into_py(self, py: Python<'_>) -> Py<PyAny> {
        PyString::new(py, self.into()).into_py(py)
    }
}

impl ToPyObject for Shape {
    fn to_object(&self, py: Python<'_>) -> PyObject {
        PyString::new(py, self.into()).to_object(py)
    }
}

impl ToPyObject for Orientation {
    fn to_object(&self, py: Python<'_>) -> PyObject {
        PyString::new(py, self.into()).to_object(py)
    }
}

impl FromPyObject<'_> for Shape {
    fn extract(ob: &'_ PyAny) -> PyResult<Self> {
        let s: &PyString = ob.downcast_exact()?;
        Shape::try_from(s.to_str()?).map_err(|_| PyValueError::new_err("invalid shape"))
    }
}

impl FromPyObject<'_> for Orientation {
    fn extract(ob: &'_ PyAny) -> PyResult<Self> {
        let s: &PyString = ob.downcast_exact()?;
        Orientation::try_from(s.to_str()?).map_err(|_| PyValueError::new_err("invalid orientation"))
    }
}
