pub mod identity;
pub mod storage;
pub mod reputation;
pub mod economic;

// Re-export all interfaces for easier access
pub use identity::IdentityProvider;
pub use storage::StorageProvider;
pub use reputation::ReputationProvider;
pub use economic::EconomicProvider; 