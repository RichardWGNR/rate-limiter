mod fixed_window;
mod sliding_window;

use crate::error::ReserveError;
use crate::storage::Storage;
use crate::Reservation;

pub use fixed_window::{FixedWindowPolicy, FixedWindowState};
pub use sliding_window::{SlidingWindowPolicy, SlidingWindowState};

pub trait Policy {
    // reset
    // consume(tokens = 1)
    // reserve(tokens = 1, float maxTime = null)

    fn reserve(
        &mut self,
        tokens: usize,
        max_time: Option<i64>,
    ) -> Result<Reservation, ReserveError>;

    fn consume(&mut self, tokens: usize) -> Result<Reservation, ReserveError>;
}
