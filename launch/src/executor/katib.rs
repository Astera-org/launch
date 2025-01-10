//! The katib experiment backend implementation.

use std::collections::HashMap;

use ::katib::models as km;
use ::kubernetes::models as k8s;
use katib::models::{
    V1beta1AlgorithmSetting, V1beta1AlgorithmSpec, V1beta1CollectorSpec, V1beta1FeasibleSpace,
    V1beta1FileSystemPath, V1beta1MetricStrategy, V1beta1MetricsCollectorSpec,
    V1beta1ObjectiveSpec, V1beta1ParameterSpec, V1beta1SourceSpec,
};
use log::{error, info, warn};

use super::{ExecutionArgs, ExecutionOutput, Executor, Result};
use crate::{cli::ClusterContext, executor::common, kubectl::ResourceHandle};

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

// Prefixed with `__launchKatib` to minimize clashes with existing parameters.
const LAUNCH_KATIB_TRIAL_NAME: &str = "__launchKatibTrialName";
const LAUNCH_KATIB_NAMESPACE: &str = "__launchKatibNamespace";

fn trial_spec(input_exp_spec: &crate::katib::ExperimentSpec, args: &ExecutionArgs) -> k8s::V1Job {
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

    // https://www.kubeflow.org/docs/components/katib/user-guides/trial-template/#use-metadata-in-trial-template
    trial_spec
        .spec
        .as_mut()
        .unwrap()
        .template
        .spec
        .as_mut()
        .unwrap()
        .containers[0]
        .env
        .as_mut()
        .unwrap()
        .extend(
            [
                ("KATIB_BASE_URL", args.context.katib_url().to_owned()),
                (
                    "KATIB_TRIAL_NAME",
                    format!("${{trialParameters.{LAUNCH_KATIB_TRIAL_NAME}}}"),
                ),
                (
                    "KATIB_NAMESPACE",
                    format!("${{trialParameters.{LAUNCH_KATIB_NAMESPACE}}}"),
                ),
            ]
            .into_iter()
            .map(|(k, v)| k8s::V1EnvVar {
                name: k.to_owned(),
                value: Some(v),
                value_from: None,
            }),
        );

    trial_spec
}

fn experiment(
    input_exp_spec: crate::katib::ExperimentSpec,
    args: &mut ExecutionArgs,
) -> Result<km::V1beta1Experiment> {
    let trial_spec = trial_spec(&input_exp_spec, args);

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
                    // https://www.kubeflow.org/docs/components/katib/user-guides/trial-template/#use-metadata-in-trial-template
                    .chain([
                        km::V1beta1TrialParameterSpec {
                            name: Some(LAUNCH_KATIB_TRIAL_NAME.to_owned()),
                            reference: Some("${trialSpec.Name}".to_owned()),
                            ..Default::default()
                        },
                        km::V1beta1TrialParameterSpec {
                            name: Some(LAUNCH_KATIB_NAMESPACE.to_owned()),
                            reference: Some("${trialSpec.Namespace}".to_owned()),
                            ..Default::default()
                        },
                    ])
                    .collect(),
            ),
            retain: Some(true),
            ..Default::default()
        })),
        ..Default::default()
    };

    // Ensure the experiment name is at most [40
    // characters](https://github.com/kubeflow/katib/issues/2454#issuecomment-2508754891) to avoid
    // [an issue with katib](https://github.com/kubeflow/katib/issues/2454).
    const EXPERIMENT_NAME_MAX_LEN: usize = 40;
    let generate_name = if args.generate_name.len() <= EXPERIMENT_NAME_MAX_LEN {
        args.generate_name
    } else {
        warn!("Truncating experiment name to {EXPERIMENT_NAME_MAX_LEN} characters");
        &args.generate_name[..EXPERIMENT_NAME_MAX_LEN]
    };

    Ok(km::V1beta1Experiment {
        api_version: Some("kubeflow.org/v1beta1".to_owned()), // https://github.com/kubeflow/katib/blob/2b41ae62ab3905984e02123218351a703c03bf56/sdk/python/v1beta1/kubeflow/katib/constants/constants.py#L28
        kind: Some("Experiment".to_owned()), // https://github.com/kubeflow/katib/blob/2b41ae62ab3905984e02123218351a703c03bf56/sdk/python/v1beta1/kubeflow/katib/constants/constants.py#L29
        metadata: Some(k8s::V1ObjectMeta {
            annotations: Some(args.annotations().clone()),
            generate_name: Some(generate_name.to_owned()),
            namespace: Some(args.job_namespace.to_owned()),
            ..Default::default()
        }),
        spec: Some(Box::new(exp_spec)),
        ..Default::default()
    })
}

pub struct KatibExecutor {
    pub experiment_spec_path: std::path::PathBuf,
}

fn read_experiment_spec(path: &std::path::Path) -> Result<crate::katib::ExperimentSpec> {
    Ok(serde_yaml::from_slice(
        &std::fs::read(path).map_err(|err| format!("Failed to read Katib experiment spec file {}: {err}", path.display()))?,
    )
    .map_err(|err| format!("Failed to parse Katib experiment spec file {}: {err}\nSee `launch submit --help` for format.", path.display()))?)
}

impl Executor for KatibExecutor {
    fn execute(&self, mut args: ExecutionArgs) -> Result<ExecutionOutput> {
        let kubectl = args.context.kubectl();

        let experiment_spec = read_experiment_spec(&self.experiment_spec_path)?;

        let ResourceHandle { namespace, name } = kubectl.create(&serde_json::to_string(
            &experiment(experiment_spec, &mut args)?,
        )?)?;

        let experiment_url = experiment_url(args.context.katib_url(), &namespace, &name);
        info!("Created experiment {experiment_url}",);

        let mut trial_to_state: HashMap<String, TrialState> = Default::default();

        loop {
            let experiment = kubectl.katib_experiment(&namespace, &name)?;

            if let Some(status) = experiment.status.as_deref() {
                log_trial_state_changes(
                    args.context,
                    &namespace,
                    &name,
                    &mut trial_to_state,
                    status,
                );

                if let Some(status) = terminal_experiment_status(status) {
                    match status {
                        TerminalExperimentStatus::Succeeded => {
                            info!("Succesfully completed experiment {experiment_url}")
                        }
                        TerminalExperimentStatus::Failed(message) => {
                            error!("Failed to complete experiment {experiment_url}: {message}",)
                        }
                    }
                    break;
                }
            }

            std::thread::sleep(super::POLLING_INTERVAL);
        }

        Ok(ExecutionOutput {})
    }
}

fn log_trial_state_changes(
    context: &ClusterContext,
    namespace: &str,
    experiment_name: &str,
    trial_to_state: &mut HashMap<String, TrialState>,
    status: &km::V1beta1ExperimentStatus,
) {
    for (trial_name, state) in trial_state_iter(status) {
        let prev_state = trial_to_state.insert(trial_name.to_owned(), state);

        if prev_state == Some(state) {
            continue;
        }

        let trial_url = trial_url(context.katib_url(), namespace, experiment_name, trial_name);
        let trial_job_url = trial_job_url(context.headlamp_url(), namespace, trial_name);
        match state {
            TrialState::Pending => info!("Awaiting pending trial {trial_url}"),
            TrialState::Running => info!("Running trial {trial_url}"),
            TrialState::Failed => error!("Failed trial {trial_url}. View logs for details: {trial_job_url}."),
            TrialState::Killed => error!("Killed trial {trial_url}. View logs for details: {trial_job_url}."),
            TrialState::EarlyStopped => info!("Early-stopped trial {trial_url}"),
            TrialState::Succeeded => info!("Succesfully completed trial {trial_url}"),
            TrialState::MetricsUnavailable => error!("Metrics unavailable for trial {trial_url}. View logs for details: {trial_job_url}."),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum TrialState {
    Pending,
    Running,
    Failed,
    Killed,
    EarlyStopped,
    Succeeded,
    MetricsUnavailable,
}

fn trial_state_iter(
    status: &::katib::models::V1beta1ExperimentStatus,
) -> impl Iterator<Item = (&str, TrialState)> {
    // The order here determines when events are printed. We want completion to be printed before
    // the starting of new trials for a more chronological order.
    status
        .succeeded_trial_list
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|trial| (trial.as_str(), TrialState::Succeeded))
        .chain(
            status
                .failed_trial_list
                .as_deref()
                .unwrap_or_default()
                .iter()
                .map(|trial| (&**trial, TrialState::Failed)),
        )
        .chain(
            status
                .killed_trial_list
                .as_deref()
                .unwrap_or_default()
                .iter()
                .map(|trial| (&**trial, TrialState::Killed)),
        )
        .chain(
            status
                .early_stopped_trial_list
                .as_deref()
                .unwrap_or_default()
                .iter()
                .map(|trial| (&**trial, TrialState::EarlyStopped)),
        )
        .chain(
            status
                .metrics_unavailable_trial_list
                .as_deref()
                .unwrap_or_default()
                .iter()
                .map(|trial| (&**trial, TrialState::MetricsUnavailable)),
        )
        .chain(
            status
                .pending_trial_list
                .as_deref()
                .unwrap_or_default()
                .iter()
                .map(|trial| (&**trial, TrialState::Pending)),
        )
        .chain(
            status
                .running_trial_list
                .as_deref()
                .unwrap_or_default()
                .iter()
                .map(|trial| (&**trial, TrialState::Running)),
        )
}

enum TerminalExperimentStatus<'a> {
    Succeeded,
    Failed(&'a str),
}

fn terminal_experiment_status(
    status: &km::V1beta1ExperimentStatus,
) -> Option<TerminalExperimentStatus<'_>> {
    let condition = status
        .conditions
        .as_deref()
        .and_then(<[_]>::last)
        .expect("experiment status should have condition");

    match condition._type.as_str() {
        "Succeeded" => Some(TerminalExperimentStatus::Succeeded),
        "Failed" => Some(TerminalExperimentStatus::Failed(
            condition.message.as_ref().unwrap(),
        )),
        "Created" | "Running" => None,
        unknown => {
            warn!("Unknown status condition type {unknown}");
            None
        }
    }
}

fn experiment_url(katib_url: &str, namespace: &str, experiment_name: &str) -> String {
    format!("{katib_url}/katib/experiment/{namespace}/{experiment_name}",)
}

fn trial_url(katib_url: &str, namespace: &str, experiment_name: &str, trial_name: &str) -> String {
    format!("{katib_url}/katib/experiment/{namespace}/{experiment_name}/trial/{trial_name}",)
}

fn trial_job_url(headlamp_url: &str, namespace: &str, trial_name: &str) -> String {
    format!("{headlamp_url}/c/main/jobs/{namespace}/{trial_name}")
}
