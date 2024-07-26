use crate::error::RateLimitExceededError;
use crate::LocalDateTime;

/// A structure containing information about
/// the current speed limit for a particular key.
#[derive(Debug)]
pub struct RateLimit {
    pub(crate) available_tokens: usize,
    pub(crate) retry_after: LocalDateTime,
    pub(crate) accepted: bool,
    pub(crate) limit: usize,
}

impl RateLimit {
    /// Returns the number of tokens available.
    pub fn get_remaining_tokens(&self) -> usize {
        self.available_tokens
    }

    /// If the tokens have run out, this method will return the time after which
    /// at least one token will be available.
    pub fn get_retry_after(&self) -> LocalDateTime {
        self.retry_after.clone()
    }

    /// Returns a result reflecting whether this request was executed within the current limit.
    pub fn is_accepted(&self) -> bool {
        self.accepted
    }

    /// TODO doc
    pub fn get_limit(&self) -> usize {
        self.limit
    }

    /// Same as [`Self::is_accepted()`], but will return Err(RateLimitExceededError) if
    /// the request failed within the current limit.
    pub fn ensure_accepted(&self) -> Result<(), RateLimitExceededError> {
        if !self.accepted {
            return Err(RateLimitExceededError); // TODO : with extra info
        }

        Ok(())
    }
}