use std::{collections::HashSet, sync::OnceLock};

use pyo3::{exceptions::PyValueError, prelude::*};
use regex::bytes::Regex;
use srs_4l::gameplay::Physics;

use crate::types::{Field, Piece, Shape, Solution};

#[pyclass]
pub struct Srs4lSolver {
    pub physics: Physics,
}

#[pymethods]
impl Srs4lSolver {
    #[pyo3(signature = (*, physics))]
    #[new]
    pub fn new(physics: &str) -> PyResult<Srs4lSolver> {
        let physics = match physics {
            "SRS" => Physics::SRS,
            "Jstris" => Physics::Jstris,
            "TETRIO" => Physics::Tetrio,
            _ => return Err(PyValueError::new_err("unsupported physics")),
        };

        Ok(Srs4lSolver { physics })
    }

    pub fn solve(&self, field: &Field, queue: &str) -> PyResult<Vec<Solution>> {
        let board: srs_4l::gameplay::Board = field.try_into()?;
        let queue: Vec<Shape> = parse_queue(queue)?;

        if queue.len() > 10 {
            return Err(PyValueError::new_err("queue too long"));
        }

        let mut this = HashSet::new();
        let mut next = HashSet::new();

        let first = srs_4l::brokenboard::BrokenBoard::from_garbage(board.0);
        this.insert(first);

        for shape in queue {
            for old_board in this.drain() {
                for (piece, _new_board) in
                    srs_4l::vector::Placements::place(old_board.board, shape.into(), self.physics)
                {
                    next.insert(old_board.place(piece));
                }
            }

            std::mem::swap(&mut this, &mut next);
        }

        let mut solutions: Vec<Solution> = this
            .drain()
            .map(|bb| Solution {
                initial_field: field.clone(),
                pieces: bb.pieces.iter().copied().map(Piece::from).collect(),
                held: None,
            })
            .collect();
        solutions.sort_unstable();
        Ok(solutions)
    }

    pub fn placements(&self, field: &Field, shape: Shape) -> PyResult<Vec<Field>> {
        let board = field.try_into()?;

        let placements = srs_4l::vector::Placements::place(board, shape.into(), self.physics);
        let mut boards: Vec<_> = placements.map(|(_piece, board)| board).collect();

        boards.sort_unstable();
        boards.dedup();
        Ok(boards.drain(..).map(Field::from).collect())
    }
}

fn parse_queue(queue: &str) -> PyResult<Vec<Shape>> {
    static QUEUE_FORMAT: OnceLock<Regex> = OnceLock::new();
    let queue_format = QUEUE_FORMAT.get_or_init(|| Regex::new("^[IJLOSTZ]*$").unwrap());

    if !queue_format.is_match(queue.as_bytes()) {
        return Err(PyValueError::new_err("invalid queue"));
    }

    Ok(queue.chars().map(|c| c.try_into().unwrap()).collect())
}
