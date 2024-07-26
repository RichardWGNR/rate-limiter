use chrono::{TimeZone};
use crate::error::{PolicyError, ReserveError};
use crate::policy::Policy;
use crate::storage::{State, Storage};
use crate::{LocalDateTime, LocalTime, Duration, Reservation, RateLimit};

pub struct FixedWindowPolicy<'a, Store: Storage<FixedWindowState, FixedWindowState>> {
    limit: usize,
    key: String,
    interval: chrono::Duration,
    storage: &'a mut Store
}

impl<Store: Storage<FixedWindowState, FixedWindowState>> Policy for FixedWindowPolicy<'_, Store> {
    fn reserve(&mut self, tokens: usize, max_time: Option<i64>) -> Result<Reservation, ReserveError> {
        if tokens > self.limit {
            // Cannot reserve more tokens than the size of the rate limiter.
            return Err(ReserveError::TooManyTokensError {
                requested: tokens,
                max: self.limit
            });
        }

        let mut state = self
            .storage
            .fetch(self.key.as_str())
            .unwrap_or_else(|| FixedWindowState::new(
                self.key.clone(),
                &self.interval,
                self.limit
            ));

        let now = LocalTime::now();
        let available_tokens = state.get_available_tokens(&now);

        let reservation: Reservation = if tokens == 0 {
            let wait_duration = state.calculate_time_for_tokens(tokens, &now);
            let retry_after = LocalTime::timestamp_millis_opt(
                &LocalTime,
                now.timestamp_millis() + wait_duration
            ).unwrap();

            Reservation {
                time_to_act: retry_after.clone(),
                rate_limit: RateLimit {
                    available_tokens: available_tokens.unwrap_or(0),
                    retry_after,
                    accepted: true,
                    limit: self.limit,
                },
            }
        } else if available_tokens.is_some() && available_tokens.unwrap() >= tokens {
            state.add(Some(tokens), Some(&now));
            Reservation {
                time_to_act: now.clone(),
                rate_limit: RateLimit {
                    available_tokens: state.get_available_tokens(&now).unwrap_or(0),
                    retry_after: now.clone(),
                    accepted: true,
                    limit: self.limit,
                },
            }
        } else {
            let wait_duration = state.calculate_time_for_tokens(tokens, &now);

            if let Some(max_time) = max_time {
                if wait_duration > max_time {
                    return Err(ReserveError::MaxWaitDurationExceededError);
                }
            }

            state.add(Some(tokens), Some(&now));

            let retry_after = LocalTime::timestamp_millis_opt(
                &LocalTime,
                now.timestamp_millis() + wait_duration
            ).unwrap();

            Reservation {
                time_to_act: retry_after.clone(),
                rate_limit: RateLimit {
                    available_tokens: state.get_available_tokens(&now).unwrap_or(0),
                    retry_after,
                    accepted: false,
                    limit: self.limit,
                },
            }
        };

        if tokens > 0 {
            self.storage.save(&self.key, state);
        }

        Ok(reservation)
    }

    fn consume(&mut self, tokens: usize) -> Result<Reservation, ReserveError> {
        self.reserve(tokens, None)
    }
}

impl<'a, Store: Storage<FixedWindowState, FixedWindowState>> FixedWindowPolicy<'a, Store> {
    pub fn new(
        limit: usize,
        key: String,
        interval: Duration,
        storage: &'a mut Store
    ) -> Result<Self, PolicyError> {
        if limit == 0 {
            return Err(PolicyError::ZeroLimitError);
        }

        if key.is_empty() {
            return Err(PolicyError::EmptyKeyError);
        }

        Ok(Self {
            limit,
            key,
            interval,
            storage
        })
    }
}

#[derive(Debug, Clone)]
pub struct FixedWindowState {
    pub key: String,
    pub hit_count: usize,
    pub interval: i64, // chrono timestamp millis
    pub max_size: usize,
    pub timer: i64
}

impl State<FixedWindowState> for FixedWindowState {
    fn get_id(&self) -> String {
        self.key.clone()
    }

    fn get_expiration_time(&self) -> usize {
        self.interval as usize
    }
}

impl FixedWindowState {
    pub fn new(key: String, interval: &chrono::Duration, max_size: usize) -> Self {
        Self {
            key,
            hit_count: 0,
            interval: interval.num_milliseconds(),
            max_size,
            timer: 0
        }
    }

    pub fn add(&mut self, hits: Option<usize>, now: Option<&LocalDateTime>) {
        let hits = hits.unwrap_or(1); // TODO : maybe error if hits == 0 ?
        let now = now
            .map(|date| date.clone())
            .unwrap_or_else(|| LocalTime::now())
            .timestamp_millis();

        if (now - self.timer) > self.interval {
            // reset window
            self.timer = now;
            self.hit_count = 0;
        }

        self.hit_count += hits;
    }

    pub fn get_available_tokens(&self, now: &LocalDateTime) -> Option<usize> {
        let now = now.timestamp_millis();

        if (now - self.timer) > self.interval {
            return Some(self.max_size)
        }

        if self.hit_count > self.max_size {
            return None; // Avoid to subtract with overflow
        }

        Some(self.max_size - self.hit_count)
    }

    pub fn calculate_time_for_tokens(&self, tokens: usize, now: &LocalDateTime) -> i64 {
        if (self.max_size - self.hit_count) >= tokens {
            return 0;
        }

        self.timer + self.interval - now.timestamp_millis()
    }
}