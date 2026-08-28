//! Subscription-library IPC DTOs — plain data only.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "UPPERCASE")]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub enum Currency {
    Eur,
    Usd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub enum BillingCycle {
    Monthly,
    Yearly,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub struct Subscription {
    pub id: String,
    pub provider: String,
    pub plan: String,
    pub amount: f64,
    pub currency: Currency,
    pub cycle: BillingCycle,
    pub renewal_day: u8,
    pub renewal_month: Option<u8>,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub struct SubscriptionDraft {
    pub provider: String,
    pub plan: String,
    pub amount: f64,
    pub currency: Currency,
    pub cycle: BillingCycle,
    pub renewal_day: u8,
    pub renewal_month: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub struct SubscriptionUpdate {
    pub id: String,
    pub provider: String,
    pub plan: String,
    pub amount: f64,
    pub currency: Currency,
    pub cycle: BillingCycle,
    pub renewal_day: u8,
    pub renewal_month: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub enum SubscriptionError {
    InvalidInput { field: String },
    NotFound { id: String },
    StoreCorrupt { reason: String },
    StoreUnavailable { reason: String },
    CommittedWithDurabilityWarning { reason: String },
}
#[cfg(test)]
mod tests {
    use super::{
        BillingCycle, Currency, Subscription, SubscriptionDraft, SubscriptionError,
        SubscriptionUpdate,
    };

    #[test]
    fn subscription_dtos_round_trip_using_camel_case_json_shape() {
        let subscription = Subscription {
            id: "sub-123".to_string(),
            provider: " OpenAI ".to_string(),
            plan: "Plus".to_string(),
            amount: 20.0,
            currency: Currency::Usd,
            cycle: BillingCycle::Monthly,
            renewal_day: 12,
            renewal_month: None,
            updated_at: "2026-08-27T10:00:00.000000001Z".to_string(),
        };
        let json = serde_json::to_string(&subscription).expect("subscription serializes");
        assert!(json.contains("renewalDay"));
        assert!(json.contains("updatedAt"));
        assert!(!json.contains("renewal_day"));
        assert_eq!(
            serde_json::from_str::<Subscription>(&json).unwrap(),
            subscription
        );
    }

    #[test]
    fn draft_update_and_errors_keep_typed_ipc_shape() {
        let draft = SubscriptionDraft {
            provider: "OpenAI".to_string(),
            plan: "Plus".to_string(),
            amount: 20.0,
            currency: Currency::Eur,
            cycle: BillingCycle::Yearly,
            renewal_day: 4,
            renewal_month: Some(3),
        };
        let update = SubscriptionUpdate {
            id: "sub-123".to_string(),
            provider: draft.provider.clone(),
            plan: draft.plan.clone(),
            amount: draft.amount,
            currency: draft.currency,
            cycle: draft.cycle,
            renewal_day: draft.renewal_day,
            renewal_month: draft.renewal_month,
        };
        assert_eq!(serde_json::to_value(&update).unwrap()["renewalMonth"], 3);
        assert_eq!(
            serde_json::to_value(SubscriptionError::InvalidInput {
                field: "amount".to_string()
            })
            .unwrap()["invalidInput"]["field"],
            "amount"
        );
        assert_eq!(
            serde_json::to_value(SubscriptionError::StoreCorrupt {
                reason: "unsupported schema".to_string()
            })
            .unwrap()["storeCorrupt"]["reason"],
            "unsupported schema"
        );
    }
}
