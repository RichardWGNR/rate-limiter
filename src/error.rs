#[derive(Debug, thiserror::Error)]
pub enum BuilderError {
    #[error("")]
    KeyNotConfiguredError,

    #[error("")]
    PolicyNotConfiguredError,
}

#[derive(Debug)]
pub enum PolicyError {
    ZeroLimitError,
    EmptyKeyError
}

#[derive(Debug, thiserror::Error)]
pub enum ReserveError {
    #[error("Cannot reserve more tokens ({requested}) than the size of the rate limiter ({max})")]
    TooManyTokensError {
        requested: usize,
        max: usize,
    },

    #[error("")]
    MaxWaitDurationExceededError
}

#[derive(Debug)]
pub struct RateLimitExceededError;