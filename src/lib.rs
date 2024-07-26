pub mod policy;
pub mod error;
pub mod storage;

mod rate_limit;
mod reservation;

use chrono::DateTime;
use policy::Policy;
use error::BuilderError;

pub use rate_limit::RateLimit;
pub use reservation::Reservation;

pub(crate) use chrono::Local as LocalTime;
pub(crate) type LocalDateTime = DateTime<LocalTime>;
pub(crate) type ChronoTimestampMillis = i64;
pub type Duration = chrono::Duration;

#[derive(Debug)]
pub struct RateLimiterBuilder<P: Policy> {
    key: String,
    policy: Option<P>,
}

impl<P: Policy> RateLimiterBuilder<P> {
    pub fn new() -> Self {
        Self {
            key: Default::default(),
            policy: None,
        }
    }

    pub fn with_key<S: Into<String>>(mut self, key: S) -> Self {
        self.key = key.into();
        self
    }

    pub fn with_policy(mut self, policy: P) -> Self {
        self.policy = Some(policy);
        self
    }

    pub fn build(self) -> Result<(), BuilderError> {
        if self.key.is_empty() {
            return Err(BuilderError::KeyNotConfiguredError);
        }

        let Some(policy) = self.policy else {
            return Err(BuilderError::PolicyNotConfiguredError);
        };

        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use crate::policy::FixedWindowPolicy;
    use crate::storage::InMemoryStorage;
    use super::*;

    #[test]
    fn abs() {


    }
}
