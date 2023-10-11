use std::sync::OnceLock;

use bitvec::prelude::*;
use pyo3::{exceptions::PyValueError, prelude::*};
use regex::bytes::Regex;

use crate::types::Field;

#[pymethods]
impl Field {
    pub const WIDTH: usize = 10;

    /// Create a field with optional initial cells and height.
    ///
    /// Initial cells are specified as rows of `[_G]{WIDTH}`, separated by `\n`.
    ///
    /// If `height` is given, the new field will have that exact height.  It
    /// must be greater than or equal to the height of the initial field, if
    /// both are given.
    ///
    /// If neither is given, the field is empty.
    #[new]
    pub fn py_new(initial: Option<&str>, height: Option<usize>) -> PyResult<Self> {
        let Some(field) = initial else {
            let height = height.unwrap_or(0);
            return Ok(Field(BitVec::repeat(false, height * Field::WIDTH)));
        };

        // `WIDTH` copies of [_G], separated by newlines
        static FORMAT: OnceLock<Regex> = OnceLock::new();
        let format = FORMAT.get_or_init(|| {
            let row = format!("[_G]{{{}}}", Field::WIDTH);
            let re = format!(r"^(?:{0}\n)*{0}\n?$", row);
            Regex::new(&re).unwrap()
        });

        if !format.is_match(field.as_bytes()) {
            return Err(PyValueError::new_err("invalid field"));
        }

        let height = {
            let init_height = (field.len() + Field::WIDTH) / (Field::WIDTH + 1);

            match height {
                None => init_height,
                Some(h) if h >= init_height => h,
                Some(_) => return Err(PyValueError::new_err("height shorter than field")),
            }
        };
        let mut result = BitVec::repeat(false, height * Field::WIDTH);

        let mut bytes = field.bytes();
        'l: for row in (0..height).rev() {
            for col in 0..Field::WIDTH {
                match bytes.next() {
                    Some(b'_') => (),
                    Some(b'G') => result.set(Field::WIDTH * row + col, true),
                    Some(_) => unreachable!(),
                    None => break 'l,
                }
            }
            match bytes.next() {
                Some(b'\n') => continue,
                Some(_) => unreachable!(),
                None => assert_eq!(row, 0),
            }
        }

        Ok(Field(result))
    }

    /// `field[column, row] == True` if the cell is filled.  Out of bounds reads
    /// return `False` (empty) and do not grow the field.
    pub fn __getitem__<'a>(&self, coords: (usize, usize)) -> PyResult<bool> {
        if coords.0 >= Field::WIDTH {
            return Err(PyValueError::new_err("coordinate too large"));
        }

        let idx = Field::WIDTH * coords.1 + coords.0;
        let filled = self.0.get(idx).as_deref().cloned().unwrap_or(false);
        Ok(filled)
    }

    /// `field[column, row] = True` fills the cell.  The field automatically
    /// grows if necessary.
    pub fn __setitem__(&mut self, coords: (usize, usize), value: bool) -> PyResult<()> {
        if coords.0 >= Field::WIDTH {
            return Err(PyValueError::new_err("coordinate too large"));
        }

        let idx = Field::WIDTH * coords.1 + coords.0;
        if idx >= self.0.len() {
            self.set_height((idx + Field::WIDTH - 1) / Field::WIDTH + 1);
        }

        self.0.set(idx, value);
        Ok(())
    }

    #[getter]
    pub fn get_height(&self) -> usize {
        self.0.len() / Field::WIDTH
    }

    /// Resize the field to be exactly `height` tall.  Either truncates its top
    /// or grows it with empty cells.
    #[setter]
    pub fn set_height(&mut self, height: usize) {
        self.0.resize(height * Field::WIDTH, false);
    }

    /// Number of cells in the field.  Always a multiple of `WIDTH`.
    pub fn __len__(&self) -> usize {
        self.0.len()
    }

    /// Human-readable ASCII art of the field.
    pub fn __str__(&self) -> String {
        let mut s = String::new();

        for row in (0..self.get_height()).rev() {
            s.push_str(&self.format_row(row));
            s.push('\n');
        }

        s.pop();
        s
    }

    pub fn __repr__(&self) -> String {
        if self.0.len() == 0 {
            "Field()".to_string()
        } else {
            let mut s = "Field(\"".to_string();
            for row in (0..self.get_height()).rev() {
                s.push_str(&self.format_row(row));
                s.push_str("\\n");
            }
            s.pop();
            s.pop();
            s.push_str("\")");
            s
        }
    }

    pub fn __copy__(&self) -> Self {
        self.clone()
    }
}

impl Field {
    fn format_row(&self, row: usize) -> String {
        let mut s = String::with_capacity(Field::WIDTH);
        for col in 0..Field::WIDTH {
            let c = match self.0[Field::WIDTH * row + col] {
                false => '_',
                true => 'G',
            };
            s.push(c);
        }
        s
    }
}
