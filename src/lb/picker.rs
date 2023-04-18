use std::{
    error, fmt,
    sync::{Arc, Mutex},
};

use rand::{rngs::StdRng, Rng, SeedableRng};

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
    fn pick_backend(&self, backends: &[T]) -> Result<usize, PickerError>;
}

pub struct RRPicker {
    last_index: Arc<Mutex<Option<usize>>>,
}

impl RRPicker {
    pub fn new() -> Self {
        Self {
            last_index: Arc::new(Mutex::new(None)),
        }
    }
}

impl Default for RRPicker {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Backend> Picker<T> for RRPicker {
    fn pick_backend(&self, backends: &[T]) -> Result<usize, PickerError> {
        let mut last_index_guard = self.last_index.lock().unwrap();
        if let Some(last_index) = *last_index_guard {
            if last_index >= backends.len() - 1 {
                *last_index_guard = Some(0);
                return Ok(0);
            } else {
                *last_index_guard = Some(last_index + 1);
                return Ok(last_index + 1);
            }
        }

        *last_index_guard = Some(0);
        Ok(0)
    }
}

pub struct RandomPicker {
    rng: Arc<Mutex<StdRng>>,
}

impl RandomPicker {
    pub fn new() -> Self {
        Self {
            rng: Arc::new(Mutex::new(StdRng::from_entropy())),
        }
    }
}

impl Default for RandomPicker {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Backend> Picker<T> for RandomPicker {
    fn pick_backend(&self, backends: &[T]) -> Result<usize, PickerError> {
        Ok(self.rng.lock().unwrap().gen_range(0..backends.len()))
    }
}

pub struct WeightedRRPicker {
    weights: Vec<usize>,
    last_index: Arc<Mutex<Option<usize>>>,
    last_inner_index: Arc<Mutex<Option<usize>>>,
}

impl WeightedRRPicker {
    pub fn new(weights: Vec<usize>) -> Self {
        Self {
            weights,
            last_index: Arc::new(Mutex::new(None)),
            last_inner_index: Arc::new(Mutex::new(None)),
        }
    }
}

impl<T: Backend> Picker<T> for WeightedRRPicker {
    fn pick_backend(&self, backends: &[T]) -> Result<usize, PickerError> {
        let mut li = self.last_index.lock().unwrap();
        let mut li_i = self.last_inner_index.lock().unwrap();

        if backends.len() != self.weights.len() {
            return Err(PickerError::InconsistentLength(
                backends.len(),
                self.weights.len(),
            ));
        }

        if let Some(last_index) = *li {
            let last_inner_index = (*li_i).expect("inner index should not be None");

            let last_inner_max = self.weights[last_index];

            if last_inner_index < last_inner_max - 1 {
                *li_i = Some(last_inner_index + 1);
                return Ok(last_index);
            } else {
                *li_i = Some(0);
            }

            if last_index >= backends.len() - 1 {
                *li = Some(0);
                return Ok(0);
            } else {
                *li = Some(last_index + 1);
                return Ok(last_index + 1);
            }
        }

        *li = Some(0);
        *li_i = Some(0);
        Ok(0)
    }
}
