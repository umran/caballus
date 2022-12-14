use oso::PolarClass;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, PolarClass)]
pub struct Member {
    #[polar(attribute)]
    pub id: Uuid,
    pub status: Status,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "name", rename_all = "snake_case")]
pub enum Status {
    PeindingVerification,
    Verified,
}

impl Status {
    pub fn name(&self) -> String {
        match self {
            Self::PeindingVerification => "pending_verification".into(),
            Self::Verified => "verified".into(),
        }
    }
}
