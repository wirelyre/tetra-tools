use std::collections::HashSet;

use js_sys::Uint8Array;
use miniserde::Deserialize;
use wasm_bindgen::prelude::*;

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub struct Game {
    field: Vec<u8>,
    physics: Vec<Physics>,
    width: u8,
    height: u8,
    spawn_height: u8,
}

#[wasm_bindgen]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Piece {
    pub physics_idx: usize,
    pub col: u8,
    pub row: u8,
    pub orientation: Orientation,
}

#[wasm_bindgen]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Orientation {
    North,
    East,
    South,
    West,
}

#[wasm_bindgen]
impl Game {
    pub fn init_srs(width: u8, height: u8, spawn_height: u8) -> Game {
        let size = width as usize * height as usize * 2;

        Game {
            field: vec![0; size.next_power_of_two()],
            physics: parse(include_str!("../srs.json")).unwrap(),
            width,
            height,
            spawn_height,
        }
    }

    pub fn buffer(&self) -> Uint8Array {
        unsafe { Uint8Array::view(&self.field) }
    }

    pub fn piece_minoes(&self, piece: &Piece) -> Uint8Array {
        let physics = &self.physics[piece.physics_idx];
        let unshifted = &physics.minoes[piece.orientation as usize];

        let minoes = Uint8Array::new_with_length(unshifted.len() as u32 * 2);
        for (i, (col, row)) in unshifted.iter().enumerate() {
            minoes.set_index(i as u32 * 2, col + piece.col);
            minoes.set_index(i as u32 * 2 + 1, row + piece.row);
        }

        minoes
    }

    pub fn spawn(&self, shape: &str) -> Option<Piece> {
        let (idx, physics) = self
            .physics
            .iter()
            .enumerate()
            .find(|p| p.1.name == shape)?;

        Some(Piece {
            physics_idx: idx,
            col: (self.width - physics.width) / 2,
            row: self.spawn_height,
            orientation: Orientation::North,
        })
        .filter(|piece| !self.collides(*piece))
    }

    pub fn get(&self, x: u8, y: u8) -> bool {
        if x >= self.width {
            true
        } else if y >= self.height {
            false
        } else {
            self.field[(x as usize + y as usize * self.width as usize) * 2] != 0
        }
    }

    pub fn collides(&self, piece: Piece) -> bool {
        use Orientation::*;

        let physics = &self.physics[piece.physics_idx];

        let oriented_width = match piece.orientation {
            North | South => physics.width,
            East | West => physics.height,
        };
        // TODO: overflow
        if piece.col + oriented_width >= self.width {
            // collides with right wall
            return true;
        }

        for (x, y) in &physics.minoes[piece.orientation as usize] {
            if self.get(x + piece.col, y + piece.row) {
                return true;
            }
        }

        false
    }

    pub fn place(&mut self, piece: Piece) {
        assert!(!self.collides(piece));

        let physics = &self.physics[piece.physics_idx];

        for (x, y) in &physics.minoes[piece.orientation as usize] {
            let idx = (x + piece.col) as usize + (y + piece.row) as usize * self.width as usize;

            if idx < self.width as usize * self.height as usize {
                self.field[idx * 2] = physics.color;
            }
        }
    }

    pub fn hard_drop(&mut self, mut piece: Piece) {
        while self.move_down(&mut piece) {}
        self.place(piece)
    }

    pub fn move_left(&self, piece: &mut Piece) -> bool {
        match piece.col.checked_sub(1) {
            Some(col) => {
                piece.col = col;
                true
            }
            None => false,
        }
    }

    pub fn move_right(&self, piece: &mut Piece) -> bool {
        match piece.col.checked_add(1) {
            Some(col) => {
                piece.col = col;
                true
            }
            None => false,
        }
    }

    pub fn move_down(&self, piece: &mut Piece) -> bool {
        match piece.row.checked_sub(1) {
            Some(row) => {
                piece.row = row;
                true
            }
            None => false,
        }
    }

    pub fn rotate_cw(&self, piece: &mut Piece) -> bool {
        use Orientation::*;
        let physics = &self.physics[piece.physics_idx];
        let (attempts, new_orientation) = match piece.orientation {
            North => (&physics.rotations.ne, East),
            East => (&physics.rotations.es, South),
            South => (&physics.rotations.sw, West),
            West => (&physics.rotations.wn, North),
        };
        self.attempt_rotation(piece, attempts, new_orientation)
    }

    pub fn rotate_180(&self, piece: &mut Piece) -> bool {
        use Orientation::*;
        let physics = &self.physics[piece.physics_idx];
        let (attempts, new_orientation) = match piece.orientation {
            North => (&physics.rotations.ns, South),
            East => (&physics.rotations.ew, West),
            South => (&physics.rotations.sn, North),
            West => (&physics.rotations.we, East),
        };
        self.attempt_rotation(piece, attempts, new_orientation)
    }

    pub fn rotate_ccw(&self, piece: &mut Piece) -> bool {
        use Orientation::*;
        let physics = &self.physics[piece.physics_idx];
        let (attempts, new_orientation) = match piece.orientation {
            North => (&physics.rotations.nw, West),
            East => (&physics.rotations.en, North),
            South => (&physics.rotations.se, East),
            West => (&physics.rotations.ws, South),
        };
        self.attempt_rotation(piece, attempts, new_orientation)
    }

    fn attempt_rotation(
        &self,
        piece: &mut Piece,
        attempts: &[(i8, i8)],
        new_orientation: Orientation,
    ) -> bool {
        for attempt in attempts {
            let new_col: Result<u8, _> = ((piece.col as i16) + (attempt.0 as i16)).try_into();
            let new_row: Result<u8, _> = ((piece.row as i16) + (attempt.1 as i16)).try_into();

            match (new_col, new_row) {
                (Ok(x), Ok(y)) => {
                    let new_piece = Piece {
                        col: x,
                        row: y,
                        orientation: new_orientation,
                        ..*piece
                    };
                    if !self.collides(new_piece) {
                        *piece = new_piece;
                        return true;
                    }
                }

                _ => continue,
            }
        }

        false
    }
}

pub struct Physics {
    name: String,
    color: u8,
    minoes: [Vec<(u8, u8)>; 4],
    width: u8,
    height: u8,
    rotations: Rotations,
}

pub struct Rotations {
    pub ne: Vec<(i8, i8)>,
    pub ns: Vec<(i8, i8)>,
    pub nw: Vec<(i8, i8)>,

    pub es: Vec<(i8, i8)>,
    pub ew: Vec<(i8, i8)>,
    pub en: Vec<(i8, i8)>,

    pub sw: Vec<(i8, i8)>,
    pub sn: Vec<(i8, i8)>,
    pub se: Vec<(i8, i8)>,

    pub wn: Vec<(i8, i8)>,
    pub we: Vec<(i8, i8)>,
    pub ws: Vec<(i8, i8)>,
}

pub fn parse(s: &str) -> Option<Vec<Physics>> {
    #[derive(Deserialize)]
    struct PieceInfo {
        name: String,
        color: u8,
        minoes: Vec<(u8, u8)>,
        rotations: RotationsInfo,
    }

    #[derive(Deserialize, Debug)]
    struct RotationsInfo {
        ne: Option<Vec<(i8, i8)>>,
        ns: Option<Vec<(i8, i8)>>,
        nw: Option<Vec<(i8, i8)>>,

        es: Option<Vec<(i8, i8)>>,
        ew: Option<Vec<(i8, i8)>>,
        en: Option<Vec<(i8, i8)>>,

        sw: Option<Vec<(i8, i8)>>,
        sn: Option<Vec<(i8, i8)>>,
        se: Option<Vec<(i8, i8)>>,

        wn: Option<Vec<(i8, i8)>>,
        we: Option<Vec<(i8, i8)>>,
        ws: Option<Vec<(i8, i8)>>,
    }

    let infos: Vec<PieceInfo> = miniserde::json::from_str(s).ok()?;
    let mut physics: Vec<Physics> = Vec::new();

    for info in infos {
        if info.color == 0 {
            return None;
        }

        let min_x: u8 = info.minoes.iter().map(|(x, _)| *x).min()?;
        let min_y: u8 = info.minoes.iter().map(|(_, y)| *y).min()?;

        if min_x != 0 || min_y != 0 {
            return None;
        }

        let w: u8 = info.minoes.iter().map(|(x, _)| *x).max()?;
        let h: u8 = info.minoes.iter().map(|(_, y)| *y).max()?;

        let mut minoes_n = info.minoes;
        let mut minoes_e: Vec<_> = minoes_n.iter().map(|(x, y)| (*y, w - x)).collect();
        let mut minoes_s: Vec<_> = minoes_n.iter().map(|(x, y)| (w - x, h - y)).collect();
        let mut minoes_w: Vec<_> = minoes_n.iter().map(|(x, y)| (h - y, *x)).collect();
        minoes_n.sort();
        minoes_e.sort();
        minoes_s.sort();
        minoes_w.sort();

        let mut rotations = info.rotations;
        opposing(&mut rotations.ne, &mut rotations.en);
        opposing(&mut rotations.ns, &mut rotations.sn);
        opposing(&mut rotations.nw, &mut rotations.wn);
        opposing(&mut rotations.es, &mut rotations.se);
        opposing(&mut rotations.ew, &mut rotations.we);
        opposing(&mut rotations.sw, &mut rotations.ws);

        fn opposing(a: &mut Option<Vec<(i8, i8)>>, b: &mut Option<Vec<(i8, i8)>>) {
            let invert = |v: &[(i8, i8)]| v.iter().map(|(x, y)| (-x, -y)).collect();

            if a.is_some() && b.is_none() {
                *b = Some(invert(a.as_ref().unwrap()));
            } else if a.is_none() && b.is_some() {
                *a = Some(invert(b.as_ref().unwrap()));
            }
        }

        let rotations = Rotations {
            ne: rotations.ne.unwrap_or_default(),
            ns: rotations.ns.unwrap_or_default(),
            nw: rotations.nw.unwrap_or_default(),
            es: rotations.es.unwrap_or_default(),
            ew: rotations.ew.unwrap_or_default(),
            en: rotations.en.unwrap_or_default(),
            sw: rotations.sw.unwrap_or_default(),
            sn: rotations.sn.unwrap_or_default(),
            se: rotations.se.unwrap_or_default(),
            wn: rotations.wn.unwrap_or_default(),
            we: rotations.we.unwrap_or_default(),
            ws: rotations.ws.unwrap_or_default(),
        };

        physics.push(Physics {
            name: info.name,
            color: info.color,
            minoes: [minoes_n, minoes_e, minoes_s, minoes_w],
            width: w + 1,
            height: h + 1,
            rotations,
        });
    }

    let names: HashSet<&str> = physics.iter().map(|p| p.name.as_ref()).collect();
    if names.len() != physics.len() {
        return None;
    }

    Some(physics)
}
