use domain::licensing::Feature;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct FeatureResponse {
    #[schema(example = "mev.fitting")]
    pub name: String,

    #[schema(example = "Fit MEV regression models from macroeconomic series.")]
    pub description: String,

    pub enabled: bool,

    /// What a fresh installation would have. Useful when deciding whether a
    /// state was set deliberately or has simply never been touched.
    pub enabled_by_default: bool,
}

impl From<(Feature, bool)> for FeatureResponse {
    fn from((feature, enabled): (Feature, bool)) -> Self {
        Self {
            name: feature.name.as_str().to_owned(),
            description: feature.description.to_owned(),
            enabled,
            enabled_by_default: feature.enabled_by_default,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SetFeatureRequest {
    pub enabled: bool,
}
