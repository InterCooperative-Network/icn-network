//! Common types used throughout the mutual credit system.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, AddAssign, Div, Mul, Sub, SubAssign, Neg};

/// A Decentralized Identifier (DID) used to identify accounts in the system
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DID {
    /// The DID string, e.g., "did:icn:alpha:xyz123"
    value: String,
}

impl DID {
    /// Create a new DID with the given value
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
        }
    }

    /// Get the DID value as a string
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for DID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

/// A timestamp used for timing events in the system
pub type Timestamp = DateTime<Utc>;

/// An amount of mutual credit
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Amount {
    /// The numeric value of the amount
    value: Decimal,
}

impl Amount {
    /// Create a new amount with the given integer value
    pub fn new(value: i64) -> Self {
        Self {
            value: Decimal::new(value, 0),
        }
    }

    /// Create a zero amount
    pub fn zero() -> Self {
        Self {
            value: Decimal::new(0, 0),
        }
    }

    /// Get the absolute value of the amount
    pub fn abs(&self) -> Self {
        Self {
            value: self.value.abs(),
        }
    }

    /// Scale the amount by a multiplier
    pub fn scale(&self, multiplier: Decimal) -> Self {
        Self {
            value: self.value * multiplier,
        }
    }

    /// Check if the amount is zero
    pub fn is_zero(&self) -> bool {
        self.value.is_zero()
    }

    /// Check if the amount is positive
    pub fn is_positive(&self) -> bool {
        self.value.is_sign_positive() && !self.value.is_zero()
    }

    /// Check if the amount is negative
    pub fn is_negative(&self) -> bool {
        self.value.is_sign_negative()
    }

    /// Get the underlying decimal value
    pub fn decimal_value(&self) -> Decimal {
        self.value
    }
}

impl Add for Amount {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            value: self.value + other.value,
        }
    }
}

impl AddAssign for Amount {
    fn add_assign(&mut self, other: Self) {
        self.value += other.value;
    }
}

impl Sub for Amount {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            value: self.value - other.value,
        }
    }
}

impl SubAssign for Amount {
    fn sub_assign(&mut self, other: Self) {
        self.value -= other.value;
    }
}

impl Mul<Decimal> for Amount {
    type Output = Self;

    fn mul(self, rhs: Decimal) -> Self {
        Self {
            value: self.value * rhs,
        }
    }
}

impl Div<Decimal> for Amount {
    type Output = Self;

    fn div(self, rhs: Decimal) -> Self {
        Self {
            value: self.value / rhs,
        }
    }
}

impl Neg for Amount {
    type Output = Self;

    fn neg(self) -> Self {
        Self {
            value: -self.value,
        }
    }
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl PartialOrd for Amount {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

impl Ord for Amount {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value.cmp(&other.value)
    }
}

/// A reputation score for an account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationScore {
    /// The account this reputation is for
    pub did: DID,
    /// The reputation score, from 0.0 to 1.0
    pub score: f64,
    /// When this reputation was last updated
    pub updated_at: Timestamp,
    /// Optional detailed reputation metrics
    pub details: Option<ReputationDetails>,
}

/// Detailed reputation metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationDetails {
    /// Reputation based on transaction history
    pub transaction_history: f64,
    /// Reputation based on community endorsements
    pub endorsements: f64,
    /// Reputation based on length of participation
    pub longevity: f64,
    /// Reputation based on governance participation
    pub governance_participation: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amount_operations() {
        let a = Amount::new(100);
        let b = Amount::new(50);
        
        assert_eq!(a.clone() + b.clone(), Amount::new(150));
        assert_eq!(a.clone() - b.clone(), Amount::new(50));
        
        let mut c = Amount::new(75);
        c += b.clone();
        assert_eq!(c, Amount::new(125));
        
        c -= b;
        assert_eq!(c, Amount::new(75));
        
        assert_eq!(a.scale(Decimal::new(15, 1)), Amount::new(150)); // 1.5 * 100
        
        assert!(Amount::new(100) > Amount::new(50));
        assert!(Amount::new(-50) < Amount::zero());
        
        assert!(Amount::new(100).is_positive());
        assert!(Amount::new(-100).is_negative());
        assert!(Amount::zero().is_zero());
    }

    #[test]
    fn test_did() {
        let did = DID::new("did:icn:alpha:test123");
        assert_eq!(did.as_str(), "did:icn:alpha:test123");
        assert_eq!(did.to_string(), "did:icn:alpha:test123");
        
        let did2 = DID::new("did:icn:alpha:test123");
        assert_eq!(did, did2);
        
        let did3 = DID::new("did:icn:beta:test456");
        assert_ne!(did, did3);
    }
} 