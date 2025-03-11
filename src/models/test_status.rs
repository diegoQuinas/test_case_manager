use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum TestStatus {
    Pending,
    Validated,
    Rejected,
    Skipped,
    Blocked,
}

impl fmt::Display for TestStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TestStatus::Pending => write!(f, "⏳ Pendiente"),
            TestStatus::Validated => write!(f, "✅ Validado"),
            TestStatus::Rejected => write!(f, "❌ Rechazado"),
            TestStatus::Skipped => write!(f, "⏭️ Omitido"),
            TestStatus::Blocked => write!(f, "🚫 Bloqueado"),
        }
    }
}
