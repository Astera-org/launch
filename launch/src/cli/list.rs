use std::{collections::HashMap, fmt::Write as _};

use time::UtcOffset;
use time_local::UtcOffsetExt;

use super::ClusterContext;
use crate::{
    ansi,
    kubectl::{self},
    Result,
};

pub fn list(context: &ClusterContext) -> Result<()> {
    use comfy_table::{Attribute, Cell, ContentArrangement, Table};

    let kubectl = context.kubectl();

    fn cmp_date_then_name(
        a: &kubectl::ResourceMetadata,
        b: &kubectl::ResourceMetadata,
    ) -> std::cmp::Ordering {
        a.creation_timestamp
            .cmp(&b.creation_timestamp)
            .reverse()
            .then_with(|| a.name.cmp(&b.name))
    }

    let jobs = {
        let mut jobs = kubectl.jobs(kubectl::NAMESPACE)?;
        jobs.sort_by(|a, b| cmp_date_then_name(&a.metadata, &b.metadata));
        jobs
    };

    let ray_jobs = {
        let mut ray_jobs = kubectl.ray_jobs(kubectl::NAMESPACE)?;
        ray_jobs.sort_by(|a, b| cmp_date_then_name(&a.metadata, &b.metadata));
        ray_jobs
    };

    let pods = {
        let mut pods = kubectl.pods(kubectl::NAMESPACE)?;
        pods.sort_by(|a, b| cmp_date_then_name(&a.metadata, &b.metadata));
        pods
    };

    #[derive(Default)]
    struct Entry {
        job: Option<kubectl::Job>,
        ray_job: Option<kubectl::RayJob>,
        pods: Vec<kubectl::Pod>,
    }

    let mut map: HashMap<String, Entry> = HashMap::with_capacity({
        // The actual capacity will be somewhere between max(j, r) and j + r.
        jobs.len() + ray_jobs.len()
    });

    let mut ray_cluster_name_to_pods: HashMap<String, Vec<kubectl::Pod>> = HashMap::default();

    for job in jobs {
        assert!(map
            .entry(job.metadata.name.clone())
            .or_default()
            .job
            .replace(job)
            .is_none());
    }

    for job in ray_jobs {
        assert!(map
            .entry(job.metadata.name.clone())
            .or_default()
            .ray_job
            .replace(job)
            .is_none());
    }

    for pod in pods {
        if let Some(owner_reference) = pod.metadata.owner_references.first() {
            match owner_reference.kind.as_str() {
                "Job" => {
                    assert_eq!(
                        Some(&owner_reference.name),
                        pod.metadata.labels.get("job-name"),
                        "owner reference and label `job-name` should be the same"
                    );
                    if let Some(entry) = map.get_mut(&owner_reference.name) {
                        entry.pods.push(pod);
                    }
                }
                "RayCluster" => {
                    ray_cluster_name_to_pods
                        .entry(owner_reference.name.to_owned())
                        .or_default()
                        .push(pod);
                }
                _ => {}
            }
        }
    }

    let rows = {
        let mut rows: Vec<Row> = map
            .into_iter()
            .map(|(name, Entry { job, ray_job, pods })| -> Row {
                Row::new(name, job, ray_job, pods, &ray_cluster_name_to_pods)
            })
            .collect::<Vec<_>>();
        rows.sort_by(|a, b| a.created.cmp(&b.created).reverse());
        rows
    };

    // The `Accessor` type and `accessor` function aid type inference. The type of an array is inferred from the first
    // element. Without the type annotation, the compiler treats the first element's accessor as a closure and not a
    // function pointer. Every closure compiles down to it's own unique type. The elements of an array must all be of
    // the same type. With more than 1 element, compilation fails.  We could also do it by specifying the type of
    // `columns`, but we can not infer the number of items in the array. See
    // https://github.com/rust-lang/rust/issues/85077.
    type Accessor = fn(&Row) -> Result<Option<String>>;

    fn accessor(f: Accessor) -> Accessor {
        f
    }

    fn format_date(value: time::OffsetDateTime) -> Result<String> {
        let fd = time::macros::format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");

        Ok(value
            .to_offset(UtcOffset::cached_local_offset())
            .format(fd)?)
    }

    fn format_offset(value: time::UtcOffset) -> Result<String> {
        let fd = time::macros::format_description!("[offset_hour sign:mandatory]:[offset_minute]");
        Ok(value.format(fd)?)
    }

    // The code below keeps column names together with a function that produces the value from the row data for that
    // column. Unfortunately, it does cause additional work. Perhaps some procedural macro machinery for defining table
    // row types with field annotations for headers and formatting implementations would be better.
    let columns = [
        (
            "name".to_string(),
            accessor(|row| Ok(Some(row.name.clone()))),
        ),
        (
            format!(
                "created ({})",
                format_offset(time::UtcOffset::cached_local_offset())?
            ),
            accessor(|row| Ok(Some(format_date(row.created)?))),
        ),
        (
            "Job status".to_string(),
            accessor(|row| Ok(row.job_status.clone())),
        ),
        (
            "RayJob status".to_string(),
            accessor(|row| Ok(row.ray_job_status.clone())),
        ),
        (
            "launched by".to_string(),
            accessor(|row| {
                Ok(row
                    .user
                    .as_deref()
                    .and_then(|user| user.split('@').next().map(str::to_string)))
            }),
        ),
    ];

    let (column_names, accessors): (Vec<_>, Vec<_>) = columns.into_iter().unzip();

    let mut table = Table::new();
    table
        .load_preset(comfy_table::presets::UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(
            column_names
                .into_iter()
                .map(|name| Cell::new(name).add_attribute(Attribute::Bold)),
        );

    for row in rows {
        // We need to collect here because we need to consume the iterator to filter out errors before we can pass it to
        // `Table::add_row` since it does not accept a Result.
        table.add_row({
            accessors
                .iter()
                .map(|f| f(&row))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .map(|value| value.unwrap_or_default())
        });
    }

    println!("{table}");

    Ok(())
}

struct Row {
    name: String,
    created: time::OffsetDateTime,
    job_status: Option<String>,
    ray_job_status: Option<String>,
    user: Option<String>,
}

impl Row {
    fn new(
        name: String,
        job: Option<kubectl::Job>,
        ray_job: Option<kubectl::RayJob>,
        pods: Vec<kubectl::Pod>,
        ray_cluster_name_to_pods: &HashMap<String, Vec<kubectl::Pod>>,
    ) -> Self {
        Self {
            created: match (&job, &ray_job) {
                (Some(job), Some(ray_job)) => job
                    .metadata
                    .creation_timestamp
                    .min(ray_job.metadata.creation_timestamp),
                (Some(job), None) => job.metadata.creation_timestamp,
                (None, Some(ray_job)) => ray_job.metadata.creation_timestamp,
                (None, None) => pods
                    .first()
                    .map(|pod| pod.metadata.creation_timestamp)
                    .unwrap_or_else(|| {
                        unreachable!(
                        "each entry in the hashmap should contain at least one Job, RayJob or Pod."
                    )
                    }),
            },
            user: determine_user(job.as_ref(), ray_job.as_ref()).map(str::to_string),
            job_status: job.map(|job| {
                let mut out = String::new();
                for condition in &job.status.conditions {
                    if condition.status {
                        append_job_condition(&mut out, condition);
                    }
                }

                for pod in pods {
                    append_pod_status(&mut out, &pod);
                }

                out
            }),
            ray_job_status: ray_job.map(|ray_job| {
                // Running:
                //
                // ```
                // (while true; do kubectl get -n launch rayjob mick-lsm7l  -o json | jq  -c '{job: .status.jobStatus, clus: .status.rayClusterStatus.state, dep: .status.jobDeploymentStatus}'; sleep 1; done) | uniq
                // ```
                //
                // Emitted:
                //
                // ```
                // {"job":null,"clus":null,"dep":"Initializing"}
                // {"job":null,"clus":null,"dep":"Running"}
                // {"job":"RUNNING","clus":"ready","dep":"Running"}
                // {"job":"SUCCEEDED","clus":"ready","dep":"Running"}
                // {"job":"SUCCEEDED","clus":"ready","dep":"Complete"}
                // ```
                //
                // A failing command emits:
                // ```
                // {"job":null,"clus":null,"dep":"Initializing"}
                // {"job":null,"clus":null,"dep":"Running"}
                // {"job":"FAILED","clus":"ready","dep":"Running"}
                // {"job":"FAILED","clus":"ready","dep":"Failed"}
                // ```
                // The cluster deployment status seems to be closely related to the status of the cluster head pod.
                // That information is valuable, but the rest is not.

                let job_deployment_status = ray_job.status.job_deployment_status.as_str();

                let mut out = String::new();

                append_job_deployment_status(&mut out, job_deployment_status);

                if let Some(ray_cluster_pods) = ray_job
                    .status
                    .ray_cluster_name
                    .as_deref()
                    .and_then(|name| ray_cluster_name_to_pods.get(name))
                {
                    for pod in ray_cluster_pods {
                        append_pod_status(&mut out, pod);
                    }
                }

                out
            }),
            name,
        }
    }
}

fn append_job_condition(out: &mut String, condition: &kubectl::JobCondition) {
    if !out.is_empty() {
        out.push('\n');
    }

    let ansii_start = match condition.r#type {
        kubectl::JobConditionType::Failed => ansi::RED,
        kubectl::JobConditionType::Suspended => ansi::YELLOW,
        kubectl::JobConditionType::Complete => ansi::EMPTY,
    };
    let ansii_end = if ansii_start.is_empty() {
        ""
    } else {
        ansi::RESET
    };
    out.push_str(ansii_start);
    out.push_str(condition.r#type.as_str());
    out.push_str(ansii_end);

    if let Some(reason) = condition.reason.as_deref() {
        out.push_str(": ");
        out.push_str(reason);
    }

    // NOTE: Omitting the `condition.message` property to keep the table concise.
}

fn append_job_deployment_status(out: &mut String, job_deployment_status: &str) {
    let ansii_start = match job_deployment_status {
        "Initializing" => ansi::YELLOW, // If you're seeing this and it is not changing, the cluster head is having trouble starting. Maybe the docker image can't be pulled.
        "Running" => ansi::GREEN,
        "Failed" => ansi::RED,
        "Complete" => ansi::EMPTY,
        "Suspended" => ansi::YELLOW, // Guessing this might exist.
        _ => ansi::CYAN,             // Not sure what other states to expect.
    };

    let ansii_end = if ansii_start.is_empty() {
        ""
    } else {
        ansi::RESET
    };

    out.push_str(ansii_start);
    out.push_str(job_deployment_status);
    out.push_str(ansii_end);
}

fn append_pod_status(out: &mut String, pod: &kubectl::Pod) {
    if !out.is_empty() {
        out.push('\n');
    }

    let ansii_start = match pod.status.phase {
        kubectl::PodPhase::Pending => ansi::YELLOW,
        kubectl::PodPhase::Running => ansi::GREEN,
        kubectl::PodPhase::Succeeded => ansi::EMPTY, // It is good but not worthy of attention.
        kubectl::PodPhase::Failed => ansi::RED,
        kubectl::PodPhase::Unknown => ansi::RED,
    };

    let ansii_end = if ansii_start.is_empty() {
        ""
    } else {
        ansi::RESET
    };

    write!(
        out,
        "{}: {ansii_start}{}{ansii_end}",
        &pod.metadata.name, pod.status
    )
    .expect("write to string should succeed");
}

fn determine_user<'a>(
    job: Option<&'a kubectl::Job>,
    ray_job: Option<&'a kubectl::RayJob>,
) -> Option<&'a str> {
    let job_meta = job.as_ref().map(|job| &job.metadata);
    let ray_job_meta = ray_job.as_ref().map(|ray_job| &ray_job.metadata);

    let machine_user_host = Option::or(
        job_meta.and_then(super::common::launched_by_machine_user),
        ray_job_meta.and_then(super::common::launched_by_machine_user),
    );

    let tailscale_user_host = Option::or(
        job_meta.and_then(super::common::launched_by_tailscale_user),
        ray_job_meta.and_then(super::common::launched_by_tailscale_user),
    );

    tailscale_user_host
        .and_then(|value| value.host().is_some().then_some(value.user()))
        .or(machine_user_host.map(|value| value.user()))
}
