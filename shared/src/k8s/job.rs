use super::super::akri::API_NAMESPACE;
use super::{
    pod::modify_pod_spec,
    pod::{
        self, AKRI_CONFIGURATION_LABEL_NAME, AKRI_INSTANCE_LABEL_NAME, AKRI_TARGET_NODE_LABEL_NAME,
        APP_LABEL_ID, CONTROLLER_LABEL_ID,
    },
    KubeInterface, OwnershipInfo, OwnershipType, ERROR_CONFLICT, ERROR_NOT_FOUND,
};
use either::Either;
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::batch::v1::{Job, JobSpec};
use k8s_openapi::api::core::v1::Pod;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{ObjectMeta, OwnerReference};
use kube::{
    api::{Api, DeleteParams, ListParams, ObjectList, PostParams},
    client::Client,
};
use log::{error, info, trace};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Length of time a Pod can be pending before we give up and retry
pub const PENDING_POD_GRACE_PERIOD_MINUTES: i64 = 5;
/// Length of time a Pod can be in an error state before we retry
pub const FAILED_POD_GRACE_PERIOD_MINUTES: i64 = 0;

/// Get Kubernetes Jobs with a given label or field selector
///
/// Example:
///
/// ```no_run
/// use akri_shared::k8s::job;
/// use kube::client::Client;
/// use kube::config;
///
/// # #[tokio::main]
/// # async fn main() {
/// let label_selector = Some("environment=production,app=nginx".to_string());
/// let api_client = Client::try_default().await.unwrap();
/// for job in job::find_jobs_with_selector(label_selector, None, api_client).await.unwrap() {
///     println!("found job: {}", job.metadata.name.unwrap())
/// }
/// # }
/// ```
///
/// ```no_run
/// use akri_shared::k8s::job;
/// use kube::client::Client;
/// use kube::config;
///
/// # #[tokio::main]
/// # async fn main() {
/// let field_selector = Some("spec.nodeName=node-a".to_string());
/// let api_client = Client::try_default().await.unwrap();
/// for job in job::find_jobs_with_selector(None, field_selector, api_client).await.unwrap() {
///     println!("found job: {}", job.metadata.name.unwrap())
/// }
/// # }
/// ```
pub async fn find_jobs_with_selector(
    label_selector: Option<String>,
    field_selector: Option<String>,
    kube_client: Client,
) -> Result<ObjectList<Job>, anyhow::Error> {
    trace!(
        "find_jobs_with_selector with label_selector={:?} field_selector={:?}",
        &label_selector,
        &field_selector
    );
    let jobs: Api<Job> = Api::all(kube_client);
    let job_list_params = ListParams {
        label_selector,
        field_selector,
        ..Default::default()
    };
    trace!("find_jobs_with_selector PRE jobs.list(...).await?");
    let result = jobs.list(&job_list_params).await;
    trace!("find_jobs_with_selector return");
    Ok(result?)
}

/// Create Kubernetes Job based on Instance & Config.
///
/// Example:
///
/// ```no_run
/// use akri_shared::k8s::{
///     OwnershipInfo,
///     OwnershipType,
///     job
/// };
/// use kube::client::Client;
/// use kube::config;
/// use k8s_openapi::api::batch::v1::JobSpec;
///
/// # #[tokio::main]
/// # async fn main() {
/// let api_client = Client::try_default().await.unwrap();
/// let svc = job::create_new_job_from_spec(
///     "job_namespace",
///     "capability_instance",
///     "capability_config",
///     OwnershipInfo::new(
///         OwnershipType::Instance,
///         "capability_instance".to_string(),
///         "instance_uid".to_string()
///     ),
///     "akri.sh/capability_name",
///     true,
///     &JobSpec::default()).unwrap();
/// # }
/// ```
pub fn create_new_job_from_spec(
    job_namespace: &str,
    instance_name: &str,
    configuration_name: &str,
    ownership: OwnershipInfo,
    resource_limit_name: &str,
    capability_is_shared: bool,
    job_spec: &JobSpec,
) -> anyhow::Result<Job> {
    trace!("create_new_job_from_spec enter");

    let app_name = pod::create_broker_app_name(
        instance_name,
        None,
        capability_is_shared,
        &"job".to_string(),
    );
    let mut labels: BTreeMap<String, String> = BTreeMap::new();
    labels.insert(APP_LABEL_ID.to_string(), app_name.clone());
    labels.insert(CONTROLLER_LABEL_ID.to_string(), API_NAMESPACE.to_string());
    labels.insert(
        AKRI_CONFIGURATION_LABEL_NAME.to_string(),
        configuration_name.to_string(),
    );
    labels.insert(
        AKRI_INSTANCE_LABEL_NAME.to_string(),
        instance_name.to_string(),
    );

    let owner_references: Vec<OwnerReference> = vec![OwnerReference {
        api_version: ownership.get_api_version(),
        kind: ownership.get_kind(),
        controller: ownership.get_controller(),
        block_owner_deletion: ownership.get_block_owner_deletion(),
        name: ownership.get_name(),
        uid: ownership.get_uid(),
    }];
    let mut modified_job_spec = job_spec.clone();
    let mut pod_spec = modified_job_spec.template.spec.clone().unwrap();
    modify_pod_spec(&mut pod_spec, resource_limit_name, None);
    modified_job_spec.template.spec = Some(pod_spec);
    let result = Job {
        spec: Some(modified_job_spec),
        metadata: ObjectMeta {
            name: Some(app_name),
            namespace: Some(job_namespace.to_string()),
            labels: Some(labels),
            owner_references: Some(owner_references),
            ..Default::default()
        },
        ..Default::default()
    };

    trace!("create_new_job_from_spec return");
    Ok(result)
}

/// Create Kubernetes Job
///
/// Example:
///
/// ```no_run
/// use akri_shared::k8s::job;
/// use kube::client::Client;
/// use kube::config;
/// use k8s_openapi::api::batch::v1::Job;
///
/// # #[tokio::main]
/// # async fn main() {
/// let api_client = Client::try_default().await.unwrap();
/// job::create_job(&Job::default(), "job_namespace", api_client).await.unwrap();
/// # }
/// ```
pub async fn create_job(
    job_to_create: &Job,
    namespace: &str,
    kube_client: Client,
) -> Result<(), anyhow::Error> {
    trace!("create_job enter");
    let jobs: Api<Job> = Api::namespaced(kube_client, namespace);
    info!("create_job jobs.create(...).await?:");
    match jobs.create(&PostParams::default(), job_to_create).await {
        Ok(created_job) => {
            info!(
                "create_job jobs.create return: {:?}",
                created_job.metadata.name
            );
            Ok(())
        }
        Err(kube::Error::Api(ae)) => {
            if ae.code == ERROR_CONFLICT {
                trace!("create_job - job already exists");
                Ok(())
            } else {
                error!(
                    "create_job jobs.create [{:?}] returned kube error: {:?}",
                    serde_json::to_string(&job_to_create),
                    ae
                );
                Err(anyhow::anyhow!(ae))
            }
        }
        Err(e) => {
            error!(
                "create_job jobs.create [{:?}] error: {:?}",
                serde_json::to_string(&job_to_create),
                e
            );
            Err(anyhow::anyhow!(e))
        }
    }
}

/// Remove Kubernetes Job
///
/// Example:
///
/// ```no_run
/// use akri_shared::k8s::job;
/// use kube::client::Client;
/// use kube::config;
///
/// # #[tokio::main]
/// # async fn main() {
/// let api_client = Client::try_default().await.unwrap();
/// job::remove_job("job_to_remove", "job_namespace", api_client).await.unwrap();
/// # }
/// ```
pub async fn remove_job(
    job_to_remove: &str,
    namespace: &str,
    kube_client: Client,
) -> Result<(), anyhow::Error> {
    trace!("remove_job enter");
    let jobs: Api<Job> = Api::namespaced(kube_client, namespace);
    info!("remove_job jobs.delete(...).await?:");
    match jobs.delete(job_to_remove, &DeleteParams::default()).await {
        Ok(deleted_job) => match deleted_job {
            Either::Left(spec) => {
                info!("remove_job jobs.delete return: {:?}", &spec.metadata.name);
                Ok(())
            }
            Either::Right(status) => {
                info!("remove_job jobs.delete return: {:?}", &status.status);
                Ok(())
            }
        },
        Err(kube::Error::Api(ae)) => {
            if ae.code == ERROR_NOT_FOUND {
                trace!("remove_job - job already removed");
                Ok(())
            } else {
                error!(
                    "remove_job jobs.delete [{:?}] returned kube error: {:?}",
                    &job_to_remove, ae
                );
                Err(anyhow::anyhow!(ae))
            }
        }
        Err(e) => {
            error!(
                "remove_job jobs.delete [{:?}] error: {:?}",
                &job_to_remove, e
            );
            Err(anyhow::anyhow!(e))
        }
    }
}