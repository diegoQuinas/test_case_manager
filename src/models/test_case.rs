use serde::{Deserialize, Serialize};
use crate::models::TestStatus;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TestCase {
    pub id: String,
    pub description: String,
    pub status: TestStatus,
    pub observations: String,
    pub evidence: String,
    pub version: String,
    pub ticket_numbers: String,
}
