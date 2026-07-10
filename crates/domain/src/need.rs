//! What the business user needs: task, purpose and data sensitivity.

use serde::{Deserialize, Serialize};

/// Task the business user wants the model for (v1 taxonomy).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Task {
    CodeGeneration,
    Agentic,
    SummaryExtraction,
    Classification,
    Writing,
    Translation,
    Reasoning,
    GeneralChat,
}

/// RGPD-aligned processing purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Purpose {
    PublicContent,
    PersonalData,
    AutomatedDecision,
    HealthData,
}

/// Data sensitivity level, C0 (public) to C3 (restricted). Ordered: a rule
/// scoped to a level also applies to every level above it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Sensitivity {
    C0,
    C1,
    C2,
    C3,
}

/// What the business user needs: task, purpose and data sensitivity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NeedProfile {
    pub(crate) task: Task,
    pub(crate) purpose: Purpose,
    pub(crate) sensitivity: Sensitivity,
}

impl NeedProfile {
    pub fn new(task: Task, purpose: Purpose, sensitivity: Sensitivity) -> Self {
        Self {
            task,
            purpose,
            sensitivity,
        }
    }
}
