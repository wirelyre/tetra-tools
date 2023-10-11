use bitvec::prelude::BitArray;
use pyo3::{exceptions::PyValueError, prelude::*};

use crate::types::{Field, Orientation, Piece};

#[pymethods]
impl Piece {
    #[new]
    fn new(shape: &str, orientation: &str, column: u8, rows: Vec<u8>) -> PyResult<Piece> {
        let Ok(shape) = shape.try_into() else {
            return Err(PyValueError::new_err("invalid shape"));
        };
        let Ok(orientation) = Orientation::try_from(orientation) else {
            return Err(PyValueError::new_err("invalid orientation"));
        };

        let (width, height) = SIZES[shape as usize][orientation as usize % 2];

        if column as usize + width as usize >= Field::WIDTH {
            return Err(PyValueError::new_err("column too large"));
        }
        if rows.len() != height as usize {
            return Err(PyValueError::new_err("wrong number of rows"));
        }

        let mut rows_bits = BitArray::new([0]);
        for &row in &rows {
            if row >= 32 {
                return Err(PyValueError::new_err("row too large"));
            }
            if rows_bits.replace(row as usize, true) {
                return Err(PyValueError::new_err("duplicate row"));
            }
        }

        Ok(Piece {
            shape,
            orientation,
            column,
            rows: rows_bits,
        })
    }

    #[getter]
    fn get_rows(&self) -> Vec<u8> {
        self.rows.iter_ones().map(|row| row as u8).collect()
    }

    #[getter]
    fn width(&self) -> u8 {
        SIZES[self.shape as usize][self.orientation as usize % 2].0
    }

    #[getter]
    fn height(&self) -> u8 {
        SIZES[self.shape as usize][self.orientation as usize % 2].1
    }

    pub fn minoes(&self) -> Vec<(u8, u8)> {
        let rows: Vec<u8> = self.rows.iter_ones().map(|r| r as u8).collect();
        debug_assert_eq!(rows.len(), self.height() as usize);

        MINOES[self.shape as usize][self.orientation as usize]
            .iter()
            .map(|&(x, y)| (x + self.column, rows[y as usize]))
            .collect()
    }
}

static SIZES: [[(u8, u8); 2]; 7] = [
    [(4, 1), (1, 4)], // I
    [(3, 2), (2, 3)], // J
    [(3, 2), (2, 3)], // L
    [(2, 2), (2, 2)], // O
    [(3, 2), (2, 3)], // S
    [(3, 2), (2, 3)], // T
    [(3, 2), (2, 3)], // Z
];

// TODO: DEFINITELY check these.
static MINOES: [[&[(u8, u8)]; 4]; 7] = [
    [
        &[(0, 0), (1, 0), (2, 0), (3, 0)],
        &[(0, 0), (0, 1), (0, 2), (0, 3)],
        &[(0, 0), (1, 0), (2, 0), (3, 0)],
        &[(0, 0), (0, 1), (0, 2), (0, 3)],
    ], // I
    [
        &[(0, 0), (1, 0), (2, 0), (0, 1)],
        &[(0, 0), (1, 0), (1, 1), (1, 2)],
        &[(2, 0), (0, 1), (1, 1), (2, 1)],
        &[(0, 0), (0, 1), (0, 2), (1, 2)],
    ], // J
    [
        &[(0, 0), (1, 0), (2, 0), (2, 1)],
        &[(0, 0), (1, 0), (0, 1), (0, 2)],
        &[(0, 0), (0, 1), (1, 1), (2, 1)],
        &[(1, 0), (1, 1), (0, 2), (1, 2)],
    ], // L
    [
        &[(0, 0), (0, 1), (1, 0), (1, 1)],
        &[(0, 0), (0, 1), (1, 0), (1, 1)],
        &[(0, 0), (0, 1), (1, 0), (1, 1)],
        &[(0, 0), (0, 1), (1, 0), (1, 1)],
    ], // O
    [
        &[(0, 0), (1, 0), (1, 1), (2, 1)],
        &[(1, 0), (0, 1), (1, 1), (0, 2)],
        &[(0, 0), (1, 0), (1, 1), (2, 1)],
        &[(1, 0), (0, 1), (1, 1), (0, 2)],
    ], // S
    [
        &[(0, 0), (1, 0), (2, 0), (1, 1)],
        &[(0, 0), (0, 1), (1, 1), (0, 2)],
        &[(1, 0), (0, 1), (1, 1), (2, 1)],
        &[(1, 0), (0, 1), (1, 1), (1, 2)],
    ], // T
    [
        &[(1, 0), (2, 0), (0, 1), (1, 1)],
        &[(0, 0), (0, 1), (1, 1), (1, 2)],
        &[(1, 0), (2, 0), (0, 1), (1, 1)],
        &[(0, 0), (0, 1), (1, 1), (1, 2)],
    ], // Z
];
