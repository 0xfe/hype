use std::{error, fmt};

use rand::{
    rngs::{StdRng, ThreadRng},
    Rng, SeedableRng,
};

use super::backend::Backend;

#[derive(Debug)]
pub enum PickerError {
    InconsistentLength(usize, usize),
}

impl fmt::Display for PickerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::InconsistentLength(want, got) => write!(
                f,
                "Picker: inconsistent number of backends {} vs {}",
                want, got
            ),
        }
    }
}

impl error::Error for PickerError {}

pub trait Picker<T: Backend>: Send + Sync {
    fn pick_backend(&mut self, backends: &Vec<T>) -> Result<usize, PickerError>;
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
    fn pick_backend(&mut self, backends: &Vec<T>) -> Result<usize, PickerError> {
        if let Some(last_index) = self.last_index {
            if last_index >= backends.len() - 1 {
                self.last_index = Some(0);
                return Ok(0);
            } else {
                self.last_index = Some(last_index + 1);
                return Ok(last_index + 1);
            }
        }

        self.last_index = Some(0);
        Ok(0)
    }
}

pub struct RandomPicker {
    rng: StdRng,
}

impl RandomPicker {
    pub fn new() -> Self {
        Self {
            rng: StdRng::from_entropy(),
        }
    }
}

impl<T: Backend> Picker<T> for RandomPicker {
    fn pick_backend(&mut self, backends: &Vec<T>) -> Result<usize, PickerError> {
        Ok(self.rng.gen_range(0..backends.len()))
    }
}

pub struct WeightedRRPicker {
    cumulative_weights: Vec<usize>,
    last_index: Option<usize>,
    last_inner_index: Option<usize>,
}

impl WeightedRRPicker {
    pub fn new(weights: Vec<usize>) -> Self {
        /*
        let mut acc = 0 as usize;
        let _cumulative_weights = weights
            .iter()
            .map(|w| {
                acc += w;
                acc
            })
            .collect();

        println!("cumulative weights: {:?}", _cumulative_weights);
        */

        Self {
            cumulative_weights: weights,
            last_index: None,
            last_inner_index: None,
        }
    }
}

impl<T: Backend> Picker<T> for WeightedRRPicker {
    fn pick_backend(&mut self, backends: &Vec<T>) -> Result<usize, PickerError> {
        if backends.len() != self.cumulative_weights.len() {
            return Err(PickerError::InconsistentLength(
                backends.len(),
                self.cumulative_weights.len(),
            ));
        }

        if let Some(last_index) = self.last_index {
            let last_inner_index = self
                .last_inner_index
                .expect("inner index should not be None");

            let last_inner_max = self.cumulative_weights[last_index];

            if last_inner_index < last_inner_max - 1 {
                self.last_inner_index = Some(last_inner_index + 1);
                return Ok(last_index);
            } else {
                self.last_inner_index = Some(0);
            }

            if last_index >= backends.len() - 1 {
                self.last_index = Some(0);
                return Ok(0);
            } else {
                self.last_index = Some(last_index + 1);
                return Ok(last_index + 1);
            }
        }

        self.last_index = Some(0);
        self.last_inner_index = Some(0);
        Ok(0)
    }
}
