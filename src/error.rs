use thiserror::Error;

pub type Result<T> = std::result::Result<T, EngineError>;

#[derive(Debug, Error, PartialEq)]
pub enum EngineError {
    #[error("CSV error: {0}")]
    Csv(String),
    #[error("I/O error: {0}")]
    Io(String),
    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),
    #[error("Numeric overflow")]
    NumericOverflow,
    #[error("Numeric underflow")]
    NumericUnderflow,
    #[error("Transaction not found: tx {0}")]
    TransactionNotFound(u32),
    #[error("Account not found: client {0}")]
    AccountNotFound(u16),
    #[error("Account locked: client {0}")]
    AccountLocked(u16),
    #[error("Insufficient funds: client {0} has {1}, needs {2}")]
    InsufficientFunds(u16, String, String),
    #[error("Duplicate transaction: tx {0}")]
    DuplicateTransaction(u32),
    #[error("Dispute already active: tx {0}")]
    DisputeAlreadyActive(u32),
    #[error("Dispute not found: tx {0}")]
    DisputeNotFound(u32),
    #[error("Fraud attempt: client {0} tried to access tx {1} belonging to client {2}")]
    FraudAttempt(u16, u32, u16),
}

impl From<csv::Error> for EngineError {
    fn from(err: csv::Error) -> Self {
        EngineError::Csv(err.to_string())
    }
}

impl From<std::io::Error> for EngineError {
    fn from(err: std::io::Error) -> Self {
        EngineError::Io(err.to_string())
    }
}
