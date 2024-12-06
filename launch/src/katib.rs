#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ObjectiveType {
    Minimize,
    Maximize,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MetricStrategyType {
    Min,
    Max,
    Latest,
}

#[derive(Debug, serde::Deserialize)]
pub struct MetricStrategy {
    pub name: String,
    pub value: MetricStrategyType,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Objective {
    #[serde(rename = "type")]
    pub type_: ObjectiveType,
    pub goal: Option<f64>,
    pub objective_metric_name: String,
    pub additional_metric_names: Option<Vec<String>>,
    pub metric_strategies: Option<Vec<MetricStrategy>>,
}

#[derive(Debug, serde::Deserialize)]
pub struct AlgorithmSetting {
    pub name: String,
    pub value: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Algorithm {
    pub algorithm_name: String,
    pub algorithm_settings: Option<Vec<AlgorithmSetting>>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(
    tag = "parameterType",
    content = "feasibleSpace",
    rename_all = "camelCase"
)]
// Serde lets us be strict about which combinations of fields are allowed.
// Based on the comments here:
// https://github.com/kubeflow/katib/blob/336396436aa49de730887456028e3daa1465e500/pkg/apis/manager/v1beta1/api.proto#L92-L97
pub enum FeasibleSpace {
    Double { min: f64, max: f64 },
    Int { min: i32, max: i32 },
    Discrete { list: Vec<f64> },
    Categorical { list: Vec<String> },
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parameter {
    pub name: String,
    #[serde(flatten)]
    pub feasible_space: FeasibleSpace,
}

/// Part of a Katib ExperimentSpec. Using a custom type rather than the code generated from the
/// Katib API so that we can enforce certain fields are required or prohibited at deserialization
/// time, which means better error messages and it simplifies the rest of the code that consumes
/// this type.
/// Notably this type does not contain a trialTemplate, since the code in launch constructs that.
///
/// This a subset of the Katib API's ExperimentSpec:
/// https://pkg.go.dev/github.com/kubeflow/katib@v0.17.0/pkg/apis/controller/experiments/v1beta1#ExperimentSpec
/// The user documentation:
/// https://www.kubeflow.org/docs/components/katib/user-guides/hp-tuning/configure-experiment/
///
/// We use camelCase for all serialized field names to match the official katib docs and examples.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentSpec {
    pub objective: Objective,
    pub algorithm: Algorithm,
    pub parallel_trial_count: i32,
    pub max_trial_count: i32,
    #[serde(default = "default_max_failed_trial_count")]
    pub max_failed_trial_count: u16,
    #[serde(deserialize_with = "deserialize_parameters")]
    pub parameters: Vec<Parameter>,
}

fn default_max_failed_trial_count() -> u16 {
    1
}

fn deserialize_parameters<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    use serde::{de::Error, Deserialize};
    let vec = Vec::deserialize(deserializer)?;
    if vec.is_empty() {
        return Err(Error::custom("parameters must not be empty"));
    }
    Ok(vec)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_deserialize_parameter() {
        let yaml = r#"
name: foo.bar
parameterType: double
feasibleSpace:
  min: 0.01
  max: 1.0
"#;
        let param_result = serde_yaml::from_str::<Parameter>(yaml);
        param_result.unwrap();
    }

    #[test]
    fn test_deserialize_parameter_bad_feasisble_space() {
        // Double parameter with categorical space
        let yaml = r#"
name: foo
parameterType: double
feasibleSpace:
  list: ["a", "b", "c"]
"#;
        let result = serde_yaml::from_str::<Parameter>(yaml);
        assert!(result.is_err());

        // Categorical parameter with double space
        let yaml = r#"
name: foo
parameterType: categorical
feasibleSpace:
  min: 0.0
  max: 1.0
"#;
        let result = serde_yaml::from_str::<Parameter>(yaml);
        assert!(result.is_err());

        // Int parameter with discrete space
        let yaml = r#"
name: foo
parameterType: int
feasibleSpace:
  list: [1.0, 2.0, 3.0]
"#;
        let result = serde_yaml::from_str::<Parameter>(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_parameters() {
        let yaml = r#"
objective:
  type: maximize
  objectiveMetricName: metric
algorithm:
  algorithmName: random
parallelTrialCount: 1
maxTrialCount: 1
parameters: []
"#;
        let result = serde_yaml::from_str::<ExperimentSpec>(yaml);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("parameters must not be empty"));
    }
}
