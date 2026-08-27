//! Prompt-library IPC DTOs — plain data only, per `model/`'s import
//! allow-list: `serde`, `ts_rs`. Persistence, ID generation, and clock reads
//! belong to `vertice-app`; this module only defines the typed contract.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub struct Prompt {
    pub id: String,
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub best_for_context: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub struct PromptDraft {
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub best_for_context: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub struct PromptUpdate {
    pub id: String,
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub best_for_context: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub enum PromptError {
    InvalidInput { field: String },
    NotFound { id: String },
    StoreUnavailable { reason: String },
}

#[cfg(test)]
mod tests {
    use super::{Prompt, PromptDraft, PromptError, PromptUpdate};

    #[test]
    fn prompt_dtos_round_trip_using_camel_case_json_shape() {
        let prompt = Prompt {
            id: "9fbd4b8b-1336-4c9e-84b5-09a4f8cf1f94".to_string(),
            title: "Refactor guide".to_string(),
            body: "Explain the refactor in steps.".to_string(),
            tags: vec!["architecture".to_string(), "tdd".to_string()],
            best_for_context: Some("Pull request review".to_string()),
            updated_at: "2026-08-26T14:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&prompt).expect("prompt serializes");
        assert!(json.contains("bestForContext"));
        assert!(json.contains("updatedAt"));
        assert!(!json.contains("best_for_context"));
        assert!(!json.contains("updated_at"));

        let decoded: Prompt = serde_json::from_str(&json).expect("prompt deserializes");
        assert_eq!(decoded, prompt);
    }

    #[test]
    fn draft_update_and_errors_keep_typed_ipc_shape() {
        let draft = PromptDraft {
            title: "Daily review".to_string(),
            body: "Summarize blockers.".to_string(),
            tags: vec!["standup".to_string()],
            best_for_context: None,
        };
        let update = PromptUpdate {
            id: "9fbd4b8b-1336-4c9e-84b5-09a4f8cf1f94".to_string(),
            title: draft.title.clone(),
            body: draft.body.clone(),
            tags: draft.tags.clone(),
            best_for_context: draft.best_for_context.clone(),
        };
        let error = PromptError::InvalidInput {
            field: "title".to_string(),
        };

        assert_eq!(
            serde_json::to_value(&draft).unwrap()["bestForContext"],
            serde_json::Value::Null
        );
        assert_eq!(
            serde_json::to_value(&update).unwrap()["id"],
            "9fbd4b8b-1336-4c9e-84b5-09a4f8cf1f94"
        );
        assert_eq!(
            serde_json::to_value(error).unwrap()["invalidInput"]["field"],
            "title"
        );
    }
}
