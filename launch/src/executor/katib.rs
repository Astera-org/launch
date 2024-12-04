//! The katib experiment backend implementation.

use ::katib::models as km;
use ::kubernetes::models as k8s;
use itertools::Itertools;
use log::info;

use super::{ExecutionArgs, ExecutionOutput, Executor, Result};
use crate::{executor::common, kubectl::ResourceHandle};

fn sanitize_param_name_for_reference(param_name: &str) -> String {
    param_name.replace('.', "__")
}

fn experiment(args: &mut ExecutionArgs) -> Result<km::V1beta1Experiment> {
    let mut exp_spec = args
        .katib_experiment_spec
        .take()
        .ok_or("args.katib_experiment_spec must be set when calling KatibExecutionBackend")?;
    let container_args = {
        let param_args = exp_spec
            .parameters
            .as_ref()
            .ok_or("Katib experiment spec missing parameters")?
            .iter()
            .map(|p| {
                p.name.as_deref().map(|name| {
                    // Respect the name from the spec for the flag name, but
                    // use the sanitized reference name in the value so that Katib can
                    // do the substitution.
                    format!(
                        "--{name}=${{trialParameters.{reference}}}",
                        reference = sanitize_param_name_for_reference(name)
                    )
                })
            });

        args.container_args
            .iter()
            .cloned()
            .map(Some)
            .chain(param_args)
            .collect::<Option<Vec<String>>>()
            .ok_or("there is a parameter with no name")?
    };
    let mut trial_spec = common::job_spec(
        args,
        // NOTE: we intentionally set command here.
        // See https://github.com/Astera-org/obelisk/issues/705
        Some(args.image_metadata.entrypoint.to_owned()),
        Some(container_args),
    );
    // Katib doesn't allow metadata in the trial spec
    trial_spec.metadata = None;
    let trial_spec_json_value = serde_json::to_value(trial_spec)?;
    exp_spec
        .parameters
        .as_mut()
        .ok_or("Katib experiment spec missing parameters")?
        .iter_mut()
        .for_each(|p| p.name = p.name.as_deref().map(sanitize_param_name_for_reference));
    let trial_parameters = exp_spec
        .parameters
        .as_ref()
        .ok_or("Katib experiment spec missing parameters")?
        .iter()
        .map(|p| km::V1beta1TrialParameterSpec {
            name: p.name.clone(),
            reference: p.name.clone(),
            ..Default::default()
        })
        .collect_vec();
    exp_spec.trial_template = Some(Box::new(km::V1beta1TrialTemplate {
        primary_container_name: Some(common::PRIMARY_CONTAINER_NAME.to_owned()),
        trial_spec: Some(trial_spec_json_value),
        trial_parameters: Some(trial_parameters),
        ..Default::default()
    }));
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
