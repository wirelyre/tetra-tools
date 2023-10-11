use ::fumen as fumen_;
use pyo3::{
    exceptions::{PyNotImplementedError, PyValueError},
    prelude::*,
    types::PyString,
};

use crate::types::{Field, Fumen, Solution};

#[pymethods]
impl Fumen {
    #[new]
    fn new(ob: &PyAny) -> PyResult<Fumen> {
        if let Ok(field) = ob.downcast::<PyCell<Field>>() {
            let field: &Field = &field.borrow();
            field.try_into()
        } else if let Ok(solution) = ob.downcast::<PyCell<Solution>>() {
            let solution: &Solution = &solution.borrow();

            let mut fumen: Fumen = (&solution.initial_field).try_into()?;
            let page = &mut fumen.0.pages[0];

            for &piece in &solution.pieces {
                for (x, y) in piece.minoes() {
                    if x >= 10 || y >= 23 {
                        return Err(PyValueError::new_err("piece out of bounds"));
                    }
                    page.field[y as usize][x as usize] = piece.shape.into();
                }
            }

            Ok(fumen)
        } else if let Ok(s) = ob.downcast::<PyString>() {
            match fumen_::Fumen::decode(s.to_str()?) {
                Ok(f) => Ok(Fumen(f)),
                Err(_) => Err(PyValueError::new_err("invalid fumen")),
            }
        } else {
            Err(PyValueError::new_err("cannot create input"))
        }
    }

    fn __str__(&self) -> String {
        self.0.encode()
    }

    fn __repr__(&self) -> String {
        format!("Fumen(\"{}\")", self.0.encode())
    }

    fn ascii(&self, height: Option<usize>) -> PyResult<String> {
        if self.0.pages.len() != 1 {
            return Err(PyNotImplementedError::new_err(
                "ASCII art currently works for 1-page fumens only",
            ));
        }
        let page = &self.0.pages[0];

        let mut actual_height = 0;
        for (i, row) in page.field.iter().enumerate() {
            if row.iter().any(|cell| *cell != fumen_::CellColor::Empty) {
                actual_height = i + 1;
            }
        }

        let height = match height {
            None => actual_height,
            Some(h) if actual_height <= h => h,
            Some(_) => return Err(PyValueError::new_err("height shorter than fumen")),
        };

        let mut s = String::new();
        for row in (0..height).rev() {
            for col in 0..10 {
                use fumen_::CellColor as C;
                let char = match page.field[row][col] {
                    C::Empty => '_',
                    C::I => 'I',
                    C::L => 'L',
                    C::O => 'O',
                    C::Z => 'Z',
                    C::T => 'T',
                    C::J => 'J',
                    C::S => 'S',
                    C::Grey => 'G',
                };
                s.push(char);
            }
            s.push('\n');
        }

        Ok(s)
    }
}

impl TryFrom<&Field> for Fumen {
    type Error = PyErr;

    fn try_from(value: &Field) -> Result<Self, Self::Error> {
        if value.get_height() > 23 {
            return Err(PyValueError::new_err("field too tall"));
        }

        let mut page = fumen_::Page::default();
        for idx in value.0.iter_ones() {
            page.field[idx / 10][idx % 10] = fumen_::CellColor::Grey;
        }

        Ok(Fumen(fumen_::Fumen {
            pages: vec![page],
            ..Default::default()
        }))
    }
}
