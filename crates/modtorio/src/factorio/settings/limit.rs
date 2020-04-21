use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone, Copy)]
pub enum Limit {
    Unlimited,
    Limited(u64),
}

impl From<u64> for Limit {
    fn from(val: u64) -> Self {
        if val == 0 {
            Self::Unlimited
        } else {
            Self::Limited(val)
        }
    }
}

impl From<Limit> for u64 {
    fn from(val: Limit) -> Self {
        match val {
            Limit::Unlimited => 0,
            Limit::Limited(v) => v,
        }
    }
}
