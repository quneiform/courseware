//! Natural keys and validation for the privately generated contract.
pub(crate) use crate::schema::qune::core::ThingRef as Ref;
use crate::schema::qune::core::Uuid;
pub(crate) use crate::schema::quneic::learning::v1::{
    CellRef, LearningMap, LearningRoot, Placement, SourcePin,
};

pub(crate) fn natural_id(parts: &[&str]) -> Ref {
    let mut bytes = Vec::new();
    for part in parts {
        bytes.extend_from_slice(&(part.len() as u64).to_be_bytes());
        bytes.extend_from_slice(part.as_bytes());
    }
    let key = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_URL, &bytes);
    Ref {
        uuid: Uuid {
            value: key.as_bytes().to_vec(),
        }
        .into(),
        ..Default::default()
    }
}
impl Ref {
    pub(super) fn valid(&self) -> bool {
        (self.uuid.is_set() || self.digest.is_set())
            && self.uuid.as_option().is_none_or(|u| u.value.len() == 16)
            && self
                .digest
                .as_option()
                .is_none_or(|d| (1..=3).contains(&d.algorithm.to_i32()) && d.value.len() == 32)
    }
}
impl SourcePin {
    pub(super) fn repo(&self) -> &str {
        self.repository.as_deref().unwrap_or_default()
    }
    pub(super) fn manifest(&self) -> &str {
        self.manifest_path.as_deref().unwrap_or_default()
    }
    pub(super) fn commit_id(&self) -> &str {
        self.commit.as_deref().unwrap_or_default()
    }
}
