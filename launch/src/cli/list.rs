use std::collections::HashMap;

use itertools::Itertools;

use crate::{kubectl, time_ext, Result};

pub fn list() -> Result<()> {
    use comfy_table::{Attribute, Cell, ContentArrangement, Table};
    use time_ext::OffsetDateTimeExt;

    let kubectl = kubectl::berkeley();

    let jobs = kubectl.jobs(kubectl::NAMESPACE)?;
    let ray_jobs = kubectl.ray_jobs(kubectl::NAMESPACE)?;

    let mut map: HashMap<String, (Option<kubectl::Job>, Option<kubectl::RayJob>)> =
        HashMap::with_capacity(jobs.len() + ray_jobs.len());

    for job in jobs {
        assert!(map
            .entry(job.metadata.name.clone())
            .or_default()
            .0
            .replace(job)
            .is_none());
    }

    for job in ray_jobs {
        assert!(map
            .entry(job.metadata.name.clone())
            .or_default()
            .1
            .replace(job)
            .is_none());
    }

    struct Row {
        name: String,
        created: time::OffsetDateTime,
        user: Option<String>,
        job_status: Option<String>,
        ray_job_status: Option<String>,
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

    let rows = {
        let mut rows: Vec<Row> = map
            .into_iter()
            .map(|(name, (job, ray_job))| Row {
                name,
                created: match (&job, &ray_job) {
                    (Some(job), Some(ray_job)) => job
                        .metadata
                        .creation_timestamp
                        .min(ray_job.metadata.creation_timestamp),
                    (Some(job), None) => job.metadata.creation_timestamp,
                    (None, Some(ray_job)) => ray_job.metadata.creation_timestamp,
                    (None, None) => unreachable!(
                        "each entry in the hashmap should contain either a Job or a RayJob, or both"
                    ),
                },
                user: determine_user(job.as_ref(), ray_job.as_ref()).map(str::to_string),
                job_status: job.map(|job| {
                    job.status
                        .conditions
                        .iter()
                        .map(|condition| match &condition.reason {
                            Some(reason) => format!("{}: {reason}", &condition.r#type),
                            None => condition.r#type.to_string(),
                        })
                        .join("\n")
                }),
                ray_job_status: ray_job.map(|ray_job| ray_job.status.job_deployment_status),
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

        Ok(value.to_local()?.format(fd)?)
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
            format!("created ({})", format_offset(time_ext::local_offset()?)?),
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
