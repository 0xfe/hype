use rand::{rngs::ThreadRng, Rng};

use super::backend::Backend;

pub trait Picker<T: Backend> {
    fn pick_backend(&mut self, backends: &Vec<T>) -> usize;
}

pub struct RRPicker {
    last_index: Option<usize>,
}

impl RRPicker {
    pub fn new() -> Self {
        Self { last_index: None }
    }
}

impl<T: Backend> Picker<T> for RRPicker {
    fn pick_backend(&mut self, backends: &Vec<T>) -> usize {
        if let Some(last_index) = self.last_index {
            if last_index >= backends.len() - 1 {
                self.last_index = Some(0);
                return 0;
            } else {
                self.last_index = Some(last_index + 1);
                return last_index + 1;
            }
        }

        self.last_index = Some(0);
        0
    }
}

pub struct RandomPicker {
    rng: ThreadRng,
}

impl RandomPicker {
    pub fn new() -> Self {
        Self {
            rng: rand::thread_rng(),
        }
    }
}

impl<T: Backend> Picker<T> for RandomPicker {
    fn pick_backend(&mut self, backends: &Vec<T>) -> usize {
        self.rng.gen_range(0..backends.len())
    }
}

pub struct WeightedRRPicker {
    rng: ThreadRng,
    weights: Vec<u16>,
}

impl WeightedRRPicker {
    pub fn new(weights: Vec<u16>) -> Self {
        Self {
            weights,
            rng: rand::thread_rng(),
        }
    }
}

impl<T: Backend> Picker<T> for WeightedRRPicker {
    fn pick_backend(&mut self, backends: &Vec<T>) -> usize {
        self.rng.gen_range(0..backends.len())
    }
}
