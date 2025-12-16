use rust_decimal::Decimal;
use serde::{Serialize, Serializer};

#[derive(Debug, Clone, Serialize)]
pub struct Account {
    #[serde(rename = "client")]
    pub client_id: u16,
    #[serde(serialize_with = "serialize_decimal")]
    pub available: Decimal,
    #[serde(serialize_with = "serialize_decimal")]
    pub held: Decimal,
    #[serde(serialize_with = "serialize_decimal")]
    pub total: Decimal,
    pub locked: bool,
}

impl Account {
    pub fn new(client_id: u16) -> Self {
        Self {
            client_id,
            available: Decimal::ZERO,
            held: Decimal::ZERO,
            total: Decimal::ZERO,
            locked: false,
        }
    }

    pub fn update_total(&mut self) {
        self.total = self.available + self.held;
    }

    pub fn has_sufficient_funds(&self, amount: Decimal) -> bool {
        self.available >= amount
    }
}

fn serialize_decimal<S>(value: &Decimal, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    // Round to max 4 decimal places,
    // then strip trailing zeros for cleaner output
    let normalized = value.round_dp(4).normalize();
    serializer.serialize_str(&normalized.to_string())
}
