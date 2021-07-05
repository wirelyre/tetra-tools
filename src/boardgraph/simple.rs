use super::Stage;

pub struct SimpleStage(pub Stage<()>);

impl SimpleStage {
    pub fn new() -> SimpleStage {
        SimpleStage(Stage::initial(()))
    }

    pub fn step(&self) -> SimpleStage {
        todo!()
    }

    pub fn count_boards(&self) -> usize {
        self.0 .0.iter().map(|subset| subset.lock().len()).sum()
    }
}
