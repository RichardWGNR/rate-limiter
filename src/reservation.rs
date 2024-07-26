use crate::{LocalDateTime, RateLimit};

#[derive(Debug)]
pub struct Reservation {
    /// Unix timestamp in seconds when this reservation should act
    pub(crate) time_to_act: LocalDateTime,
    pub(crate) rate_limit: RateLimit,
}

impl Reservation {
    pub fn get_time_to_act(&self) -> &LocalDateTime {
        &self.time_to_act
    }

    pub fn get_rate_limit(&self) -> &RateLimit {
        &self.rate_limit
    }
}
