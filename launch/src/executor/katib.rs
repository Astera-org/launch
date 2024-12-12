//! The katib experiment backend implementation.

use ::katib::models as km;
use ::kubernetes::models as k8s;
use itertools::Itertools;
use katib::models::{
    V1beta1AlgorithmSetting, V1beta1AlgorithmSpec, V1beta1CollectorSpec, V1beta1FeasibleSpace,
    V1beta1FileSystemPath, V1beta1MetricStrategy, V1beta1MetricsCollectorSpec,
    V1beta1ObjectiveSpec, V1beta1ParameterSpec, V1beta1SourceSpec,
};
use log::info;

use super::{ExecutionArgs, ExecutionOutput, Executor, Result};
use crate::{executor::common, kubectl::ResourceHandle};

fn sanitize_param_name(param_name: &str) -> String {
    // '.' is special because it's used in the template substitution that katib does on
    // the args.
    param_name.replace('.', "__")
}

impl From<&crate::katib::MetricStrategyType> for String {
    fn from(strategy_type: &crate::katib::MetricStrategyType) -> Self {
        match strategy_type {
            crate::katib::MetricStrategyType::Min => "min",
            crate::katib::MetricStrategyType::Max => "max",
            crate::katib::MetricStrategyType::Latest => "latest",
        }
        .to_owned()
    }
}

impl From<&crate::katib::ObjectiveType> for String {
    fn from(obj_type: &crate::katib::ObjectiveType) -> Self {
        match obj_type {
            crate::katib::ObjectiveType::Minimize => "minimize",
            crate::katib::ObjectiveType::Maximize => "maximize",
        }
        .to_owned()
    }
}

impl crate::katib::FeasibleSpace {
    pub fn parameter_type_string(&self) -> String {
        match self {
            Self::Double { .. } => "double",
            Self::Int { .. } => "int",
            Self::Discrete { .. } => "discrete",
            Self::Categorical { .. } => "categorical",
        }
        .to_owned()
    }
}

impl From<&crate::katib::FeasibleSpace> for V1beta1FeasibleSpace {
    fn from(feasible_space: &crate::katib::FeasibleSpace) -> Self {
        use crate::katib::FeasibleSpace;
        match feasible_space {
            FeasibleSpace::Double { min, max } => Self {
                max: Some(max.to_string()),
                min: Some(min.to_string()),
                list: None,
                step: None,
            },
            FeasibleSpace::Int { min, max } => Self {
                max: Some(max.to_string()),
                min: Some(min.to_string()),
                list: None,
                step: None,
            },
            FeasibleSpace::Discrete { list } => Self {
                max: None,
                min: None,
                list: Some(list.iter().map(|x| x.to_string()).collect()),
                step: None,
            },
            FeasibleSpace::Categorical { list } => Self {
                max: None,
                min: None,
                list: Some(list.clone()),
                step: None,
            },
        }
    }
}

// According to the Katib docs this is the default for the TensorFlowEvent collector, but we
// specify it explicitly for clarity.
const TENSORBOARD_DIR: &str = "/var/log/katib/tfevent/";
const TENSORBOARD_DIR_FLAG: &str = "--tensorboard_dir";

fn experiment(args: &mut ExecutionArgs) -> Result<km::V1beta1Experiment> {
    let input_exp_spec = args
        .katib_experiment_spec
        .take()
        .ok_or("args.katib_experiment_spec must be set when calling KatibExecutionBackend")?;

    let container_args = {
        let param_args = input_exp_spec.parameters.iter().map(|p| {
            let name = p.name.as_str();
            // Use the sanitized name in the value so that Katib can do the substitution.
            format!(
                "--{name}=${{trialParameters.{sanitized}}}",
                sanitized = sanitize_param_name(name)
            )
        });

        args.container_args
            .iter()
            .cloned()
            .chain(param_args)
            .chain([TENSORBOARD_DIR_FLAG.to_owned(), TENSORBOARD_DIR.to_owned()])
            .collect()
    };
    let mut trial_spec = common::job_spec(args, None, Some(container_args));
    // Katib doesn't allow metadata in the trial spec
    trial_spec.metadata = None;

    let exp_spec = km::V1beta1ExperimentSpec {
        objective: Some(Box::new(V1beta1ObjectiveSpec {
            _type: Some((&input_exp_spec.objective.type_).into()),
            goal: input_exp_spec.objective.goal,
            additional_metric_names: input_exp_spec.objective.additional_metric_names,
            objective_metric_name: Some(input_exp_spec.objective.objective_metric_name),
            metric_strategies: input_exp_spec.objective.metric_strategies.map(|vec| {
                vec.iter()
                    .map(|strategy| V1beta1MetricStrategy {
                        name: Some(strategy.name.clone()),
                        value: Some((&strategy.value).into()),
                    })
                    .collect()
            }),
        })),
        algorithm: Some(Box::new(V1beta1AlgorithmSpec {
            algorithm_name: Some(input_exp_spec.algorithm.algorithm_name),
            algorithm_settings: input_exp_spec.algorithm.algorithm_settings.map(|vec| {
                vec.iter()
                    .map(|setting| V1beta1AlgorithmSetting {
                        name: Some(setting.name.clone()),
                        value: Some(setting.value.clone()),
                    })
                    .collect()
            }),
        })),
        metrics_collector_spec: Some(Box::new(V1beta1MetricsCollectorSpec {
            collector: Some(Box::new(V1beta1CollectorSpec {
                kind: Some("TensorFlowEvent".to_owned()),
                custom_collector: None,
            })),
            source: Some(Box::new(V1beta1SourceSpec {
                file_system_path: Some(Box::new(V1beta1FileSystemPath {
                    path: Some(TENSORBOARD_DIR.to_owned()),
                    kind: Some("Directory".to_owned()),
                    format: None,
                })),
                filter: None,
                http_get: None,
            })),
        })),
        parallel_trial_count: Some(input_exp_spec.parallel_trial_count),
        max_trial_count: Some(input_exp_spec.max_trial_count),
        max_failed_trial_count: Some(input_exp_spec.max_failed_trial_count as i32),
        parameters: Some(
            input_exp_spec
                .parameters
                .iter()
                .map(|param| V1beta1ParameterSpec {
                    feasible_space: Some(Box::new((&param.feasible_space).into())),
                    name: Some(sanitize_param_name(&param.name)),
                    parameter_type: Some(param.feasible_space.parameter_type_string()),
                })
                .collect(),
        ),
        trial_template: Some(Box::new(km::V1beta1TrialTemplate {
            primary_container_name: Some(common::PRIMARY_CONTAINER_NAME.to_owned()),
            trial_spec: Some(serde_json::to_value(trial_spec)?),
            trial_parameters: Some(
                input_exp_spec
                    .parameters
                    .iter()
                    .map(|p| km::V1beta1TrialParameterSpec {
                        name: Some(sanitize_param_name(&p.name)),
                        reference: Some(sanitize_param_name(&p.name)),
                        ..Default::default()
                    })
                    .collect_vec(),
            ),
            ..Default::default()
        })),
        ..Default::default()
    };

    Ok(km::V1beta1Experiment {
        api_version: Some("kubeflow.org/v1beta1".to_owned()), // https://github.com/kubeflow/katib/blob/2b41ae62ab3905984e02123218351a703c03bf56/sdk/python/v1beta1/kubeflow/katib/constants/constants.py#L28
        kind: Some("Experiment".to_owned()), // https://github.com/kubeflow/katib/blob/2b41ae62ab3905984e02123218351a703c03bf56/sdk/python/v1beta1/kubeflow/katib/constants/constants.py#L29
        metadata: Some(k8s::V1ObjectMeta {
            annotations: Some(args.annotations().clone()),
            generate_name: Some(args.generate_name.to_owned()),
            namespace: Some(args.job_namespace.to_owned()),
            ..Default::default()
        }),
        spec: Some(Box::new(exp_spec)),
        ..Default::default()
    })
}

pub struct KatibExecutionBackend;

impl Executor for KatibExecutionBackend {
    fn execute(&self, mut args: ExecutionArgs) -> Result<ExecutionOutput> {
        let kubectl = args.context.kubectl();

        let (exp_namespace, exp_name) = {
            let exp = experiment(&mut args)?;
            let serialized_exp = serde_json::to_string(&exp)?;
            let ResourceHandle { namespace, name } = kubectl.create(&serialized_exp)?;
            (namespace, name)
        };

        let katib_url = args.context.katib_url();
        info!(
            "Created Experiment {:?}",
            format!("{katib_url}/experiment/{exp_namespace}/{exp_name}")
        );

        // TODO: Wait for the experiment to at least run one trial successfully. If it doesn't, check for common problems.

        Ok(ExecutionOutput {})
    }
}
