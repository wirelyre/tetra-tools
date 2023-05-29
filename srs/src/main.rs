use std::io::Read;

use rhai::Engine;

use srs::physics::Chunk;

#[derive(Clone)]
pub struct Rotations<T> {
    pub ne: T,
    pub ns: T,
    pub nw: T,

    pub es: T,
    pub ew: T,
    pub en: T,

    pub sw: T,
    pub sn: T,
    pub se: T,

    pub wn: T,
    pub we: T,
    pub ws: T,
}

fn main() {
    let mut src = Vec::new();
    std::io::stdin().lock().read_to_end(&mut src).unwrap();
    let src = std::str::from_utf8(&src).unwrap();

    let mut e = Engine::new();
    e.set_max_expr_depths(0, 0);

    e.on_print(|s| print!("{}", s))
        .register_fn("println", |s: &str| println!("{}", s));

    e.register_fn("upper_shift", |i: i64, x: i64, y: i64| {
        (i as u32).upper_shift(x as i8, y as u8) as i64
    })
    .register_fn("lower_shift", |i: i64, x: i64, y: i64| {
        (i as u32).lower_shift(x as i8, y as u8) as i64
    });

    e.run(src).unwrap();
}
