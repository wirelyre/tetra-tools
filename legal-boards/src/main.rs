pub mod boardgraph;

fn main() {
    for num in BitChoiceWizard::new(2, 5) {
        println!("{}", num);
    }
    println!("{}", BitChoiceWizard::new(12, 40).count());
}

pub struct BitChoiceWizard {
    max: u8,
    bits: Vec<u8>,
}

impl BitChoiceWizard {
    pub fn new(count: usize, max: u8) -> BitChoiceWizard {
        assert!(count > 0);
        assert!(count <= max as usize);

        let mut bits = vec![0; count];
        bits[0] = 255;

        BitChoiceWizard { max, bits }
    }
}

impl Iterator for BitChoiceWizard {
    type Item = u64;

    fn next(&mut self) -> Option<u64> {
        if self.bits[0] == 255 {
            self.bits[0] = 0;
            return Some((1 << self.bits.len()) - 1);
        }

        if self.bits[0] as usize + self.bits.len() == self.max as usize {
            return None;
        }

        for place in 0..self.bits.len() {
            self.bits[place] += 1;
            let this = self.bits[place];

            match self.bits.get(place + 1) {
                Some(&that) if this > that => {
                    self.bits[place] = 0;
                    continue;
                }
                _ => break,
            }
        }

        let mut num = 0;

        for (i, bit_num) in self.bits.iter().enumerate() {
            let bit = 1 << (*bit_num as usize + i);
            num |= bit;
        }

        Some(num)
    }
}

// 0, 0 => 0011
// 1, 0 => 0101
// 2, 0 => 1001
// 3! 0
// 0, 1 => 0011
