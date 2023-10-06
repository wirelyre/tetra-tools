use std::{io::Write, time::Duration};

use rayon::{
    iter::{IntoParallelRefMutIterator, ParallelIterator},
    prelude::*,
};
use smallvec::SmallVec;

use compute::{Counter, ShardedHashMap};
use srs_4l::{
    gameplay::{Board, Physics, Shape},
    vector::Placements,
};

type NoHashBuilder = nohash::BuildNoHashHasher<u64>;
type Map = ShardedHashMap<Board, SmallVec<[Board; 6]>, 20, NoHashBuilder>;
type Set = ShardedHashMap<Board, (), 20, NoHashBuilder>;

pub fn compute() -> Vec<Board> {
    let mut stages: Vec<Map> = Vec::new();
    stages.resize_with(11, Map::new);

    stages[0].insert(Board::empty(), SmallVec::new());

    for iter in 1..=10 {
        let (prev_stage, this_stage) = match &mut stages[iter - 1..] {
            [prev, this, ..] => (prev, this),
            _ => unreachable!(),
        };

        let counter = Counter::zero();
        let total = prev_stage.len();

        crossbeam::scope(|s| {
            s.spawn(|_| loop {
                let counted = counter.get();
                eprint!(
                    "\r{:>12} / {:>12} ({:>6.2}%)",
                    counted,
                    total,
                    (counted as f64) / (total as f64) * 100.
                );
                std::io::stdout().flush().unwrap();
                if counted == total as u64 {
                    return;
                }
                std::thread::sleep(Duration::from_millis(100));
            });

            prev_stage.par_iter_mut().for_each(|(&board, _preds)| {
                for shape in Shape::ALL {
                    // No need to use Physics::SRS, since Jstris placements are
                    // a superset of SRS placements.
                    for (_piece, new_board) in (Placements::place(board, shape, Physics::Jstris)
                        | Placements::place(board, shape, Physics::Tetrio))
                    .canonical()
                    {
                        if new_board.has_isolated_cell() || new_board.has_imbalanced_split() {
                            continue;
                        }

                        let mut guard = this_stage.get_shard_guard(&new_board);
                        let preds = guard.entry(new_board).or_default();
                        if !preds.contains(&board) {
                            preds.push(board);
                        }
                    }
                }
                counter.increment();
            });
        })
        .unwrap();

        eprintln!();
    }

    let stages: Vec<_> = stages.drain(..).map(ShardedHashMap::freeze).collect();

    const FULL: Board = Board(0xFFFFF_FFFFF);
    let mut work = {
        let work = Set::new();
        work.insert(FULL, ());
        work.freeze()
    };
    let mut all_boards = vec![FULL];

    for (i, stage) in stages.iter().enumerate().rev() {
        println!("{:>4}-piece boards: {:>9}", i, work.len());

        work = work
            .par_iter()
            .flat_map_iter(|(&board, ())| stage.get(&board).unwrap())
            .map(|&board| (board, ()))
            .collect();

        all_boards.extend(work.iter().map(|(&board, ())| board));
    }

    // Dropping the stages takes a long time.  We're almost done anyway.
    std::mem::forget(stages);

    println!("sorting...");
    all_boards.par_sort_unstable();
    println!("sorted.");
    all_boards
}
