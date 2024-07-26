use crate::error::{PolicyError, ReserveError};
use crate::policy::Policy;
use crate::storage::{State, Storage};
use crate::LocalTime;
use crate::{ChronoTimestampMillis, Duration, RateLimit, Reservation};
use chrono::TimeZone;
use std::cmp::{max, min};
use std::ops::Add;

pub struct SlidingWindowPolicy<'a, Store: Storage<SlidingWindowState, SlidingWindowState>> {
    limit: usize,
    key: String,
    interval: chrono::Duration,
    storage: &'a mut Store,
}

impl<Store: Storage<SlidingWindowState, SlidingWindowState>> Policy
    for SlidingWindowPolicy<'_, Store>
{
    fn reserve(
        &mut self,
        tokens: usize,
        max_time: Option<i64>,
    ) -> Result<Reservation, ReserveError> {
        if tokens > self.limit {
            // Cannot reserve more tokens than the size of the rate limiter.
            return Err(ReserveError::TooManyTokensError {
                requested: tokens,
                max: self.limit,
            });
        }

        let mut state = self
            .storage
            .fetch(self.key.as_str())
            .unwrap_or_else(|| SlidingWindowState::new(self.key.clone(), &self.interval));

        if state.is_expired() {
            state = SlidingWindowState::create_from_previous_window(&state, &self.interval);
        }

        let now = LocalTime::now();
        let hit_count = state.get_hit_count();
        let available_tokens = self.get_available_tokens(hit_count);

        let reservation = if tokens == 0 {
            let available_tokens = available_tokens.unwrap_or(0);
            let reset_duration = state.calculate_time_for_tokens(self.limit, state.get_hit_count());
            let reset_time = if available_tokens > 0 {
                LocalTime::now()
            } else {
                LocalTime::timestamp_millis_opt(&LocalTime, now.timestamp_millis() + reset_duration)
                    .unwrap()
            };

            Reservation {
                time_to_act: now.clone(),
                rate_limit: RateLimit {
                    available_tokens,
                    retry_after: reset_time,
                    accepted: true,
                    limit: self.limit,
                },
            }
        } else if available_tokens.is_some() && available_tokens.unwrap() >= tokens {
            state.add(Some(tokens));
            Reservation {
                time_to_act: now.clone(),
                rate_limit: RateLimit {
                    available_tokens: self
                        .get_available_tokens(state.get_hit_count())
                        .unwrap_or(0),
                    retry_after: now.clone(),
                    accepted: true,
                    limit: self.limit,
                },
            }
        } else {
            let wait_duration = state.calculate_time_for_tokens(self.limit, tokens);

            if let Some(max_time) = max_time {
                if wait_duration > max_time {
                    return Err(ReserveError::MaxWaitDurationExceededError);
                }
            }

            state.add(Some(tokens));

            let retry_after =
                LocalTime::timestamp_millis_opt(&LocalTime, wait_duration + now.timestamp_millis())
                    .unwrap();

            Reservation {
                time_to_act: retry_after.clone(),
                rate_limit: RateLimit {
                    available_tokens: self
                        .get_available_tokens(state.get_hit_count())
                        .unwrap_or(0),
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

impl<'a, Store: Storage<SlidingWindowState, SlidingWindowState>> SlidingWindowPolicy<'a, Store> {
    pub fn new(
        limit: usize,
        key: String,
        interval: Duration,
        storage: &'a mut Store,
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
            storage,
        })
    }

    fn get_available_tokens(&self, hit_count: usize) -> Option<usize> {
        if hit_count > self.limit {
            return None; // Avoid to subtract with overflow
        }

        Some(self.limit - hit_count)
    }
}

#[derive(Debug, Clone)]
pub struct SlidingWindowState {
    pub key: String,
    hit_count: usize,
    hit_count_for_last_window: usize,
    pub interval: ChronoTimestampMillis,
    pub window_end_at: ChronoTimestampMillis,
}

impl State<SlidingWindowState> for SlidingWindowState {
    fn get_id(&self) -> String {
        self.key.clone()
    }

    fn get_expiration_time(&self) -> usize {
        self.interval as usize
    }
}

impl SlidingWindowState {
    pub fn new(key: String, interval: &chrono::Duration) -> Self {
        Self {
            key,
            hit_count: 0,
            hit_count_for_last_window: 0,
            interval: interval.num_milliseconds(),
            window_end_at: LocalTime::now().timestamp_millis() + interval.num_milliseconds(),
        }
    }

    pub fn create_from_previous_window(window: &Self, interval: &chrono::Duration) -> Self {
        let mut new = Self::new(window.key.clone(), interval);
        let window_end_at = window.window_end_at + interval.num_milliseconds();

        if LocalTime::now().timestamp_millis() < window_end_at {
            new.hit_count_for_last_window = window.hit_count;
            new.window_end_at = window_end_at;
        }

        new
    }

    pub fn get_expiration_time(&self) -> ChronoTimestampMillis {
        // TODO : Maybe subtract with overflow?
        self.window_end_at + self.interval - LocalTime::now().timestamp_millis()
    }

    pub fn is_expired(&self) -> bool {
        LocalTime::now().timestamp_millis() > self.window_end_at
    }

    pub fn add(&mut self, hits: Option<usize>) {
        let hits = hits.unwrap_or(1); // TODO : maybe error if hits == 0?
        self.hit_count += hits;
    }

    /// Calculates the sliding window number of request.
    pub fn get_hit_count(&self) -> usize {
        let start_of_window = self.window_end_at - self.interval;
        let percent_of_current_time_frame =
            min(LocalTime::now().timestamp_millis() - start_of_window, 1) as usize;

        // TODO : Maybe subtract with overflow?
        self.hit_count_for_last_window * (1 - percent_of_current_time_frame) + self.hit_count
    }

    pub fn calculate_time_for_tokens(&self, max_size: usize, tokens: usize) -> i64 {
        let remaining = max_size - self.get_hit_count();

        if remaining >= tokens {
            return 0;
        }

        let time = LocalTime::now().timestamp_millis();
        let start_of_window = self.window_end_at - self.interval;
        let time_passed = time - start_of_window;

        // https://github.com/symfony/rate-limiter/blob/f1fbc60e7fed63f1c77bbf8601170cc80fddd95a/Policy/SlidingWindow.php#L97
        let window_passed: f64 = {
            // I would do it via std::cmp::min, but Ord<f64> is not implemented,
            // so you can't do without that shit
            let value = time_passed as f64 / self.interval as f64;
            if value > 1. {
                1.
            } else {
                value
            }
        };

        // https://github.com/symfony/rate-limiter/blob/f1fbc60e7fed63f1c77bbf8601170cc80fddd95a/Policy/SlidingWindow.php#L98
        let releasable = max(
            1,
            max_size
                - ((self.hit_count_for_last_window as f64 * (1. - window_passed)).floor() as usize),
        );

        let remaining_window = (self.interval - time_passed) as usize;
        let needed = tokens - remaining;

        if releasable >= needed {
            return (needed as f64 * (remaining_window as f64 / max(1, releasable) as f64)) as i64;
        }

        // TODO : Refactor

        (self.window_end_at - time)
            + (needed as i64 - releasable as i64) * (self.interval / max_size as i64)
    }
}
