pub mod mutual_credit;
pub mod mutual_credit_v2;
pub mod account_manager;
pub mod transaction_processor;

// Re-export old types for backward compatibility
pub use mutual_credit::{
    AccountBalance,
    Transaction,
    TransactionType,
    TransactionStatus,
    MutualCreditSystem as LegacyMutualCreditSystem,
    MutualCreditLedger,
    MutualCreditConfig as LegacyMutualCreditConfig,
};

// Re-export new implementation
pub use mutual_credit_v2::{
    MutualCreditSystem,
    MutualCreditConfig,
};

pub use account_manager::{AccountManager, Account};
pub use transaction_processor::{TransactionProcessor, TransactionResult}; 