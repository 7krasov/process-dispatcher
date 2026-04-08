use crate::k8s::k8s_common::{extract_pod_meta_annotations, K8sParams, FINALIZER_NAME};
use crate::supervisor::Supervisor;
use futures_util::StreamExt;
use k8s_openapi::api::core::v1::Pod;
use kube::api::{Api, Patch, PatchParams};
use kube::ResourceExt;
use kube_runtime::controller::Action;
use kube_runtime::{watcher, Controller};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::RwLock;

struct ReconcileContext {
    pods: Arc<Api<Pod>>,
    k8s_params: Arc<K8sParams>,
    supervisor: Arc<RwLock<Supervisor>>,
}

#[derive(Debug, Error)]
enum ReconcileError {
    #[error("Kubernetes API error: {0}")]
    KubeError(#[from] kube::Error),

    #[error("Serde JSON error: {0}")]
    SerdeError(#[from] serde_json::Error),
}

pub async fn start_k8s_cycle(
    supervisor_arc: Arc<RwLock<Supervisor>>,
    // k8s_params: Arc<K8sParams>,
    k8s_params: Arc<K8sParams>,
    // pod_name: String,
) {
    //start listening pod events and reacting on them:
    //add finalizer on the first run, don't allow k8s to delete the pod until the supervisor will finish itself
    //accept deletion, switch to drain mode
    //accept termination, switch to terminate mode
    start_controller(
        Arc::clone(&k8s_params),
        Some(format!("metadata.name={}", k8s_params.get_pod_name())),
        Arc::clone(&supervisor_arc),
    )
    .await;
}

pub async fn remove_supervisor_finalizer(k8s_params: Arc<&K8sParams>) {
    println!(
        "Removing finalizer from pod {}...",
        k8s_params.get_pod_name()
    );
    let pod_api: Api<Pod> =
        Api::namespaced(k8s_params.get_client(), k8s_params.get_namespace().as_ref()).clone();

    let remove_finalizers_patch = json!({
    "metadata": {
        "finalizers": null
    }
    });

    let res = pod_api
        .patch(
            k8s_params.get_pod_name().as_str(),
            &PatchParams::apply(FINALIZER_NAME),
            &Patch::Merge(&remove_finalizers_patch),
        )
        .await;
    match res {
        Ok(_) => {
            println!(
                "Pod {} finalizer has been removed",
                k8s_params.get_pod_name()
            );
        }
        Err(e) => {
            println!(
                "Failed to remove a finalizer for pod {}: {}",
                k8s_params.get_pod_name(),
                e
            );
        }
    }
}

pub async fn mark_itself_as_finished(
    k8s_params: Arc<&K8sParams>,
    // pod_name: &str,
) -> Result<(), anyhow::Error> {
    println!("Marking pod as finished");

    let patch = serde_json::json!({
        "metadata": {
            "annotations": {
                "finished": "true"
            }
        }
    });
    let pods: Api<Pod> =
        Api::namespaced(k8s_params.get_client(), k8s_params.get_namespace().as_ref());
    let res = pods
        .patch(
            k8s_params.get_pod_name().as_str(),
            &kube::api::PatchParams::apply("process-supervisor/finished"),
            &kube::api::Patch::Merge(&patch),
        )
        .await;

    match res {
        Ok(_) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

async fn reconcile(pod: Arc<Pod>, ctx: Arc<ReconcileContext>) -> Result<Action, ReconcileError> {
    println!("Reconciling...");
    let name = pod.name_any();

    let metadata = &pod.metadata;
    let finalizers = metadata.finalizers.clone().unwrap_or_default();

    // add finalizer if it doesn't exist. It does not allow k8s to delete the pod
    if metadata.deletion_timestamp.is_none() && !finalizers.contains(&FINALIZER_NAME.to_string()) {
        let res = add_finalizer(ctx, &name, finalizers).await;
        if let Err(ReconcileError::KubeError(e)) = res {
            println!("Failed to add finalizer to pod {}: {}", name, e);
            return Err(ReconcileError::KubeError(e));
        }
        return Ok(Action::await_change());
    }

    let annotations = extract_pod_meta_annotations(metadata.clone());

    // if a pod is marked to be deleted by k8s, switch to "drain" mode.
    // supervisor will not get new processes and will terminate itself after all processes are finished
    if metadata.deletion_timestamp.is_some() && !annotations.is_drain_mode() {
        println!(
            "Pod {} is market to be deleted. Adding drain annotation...",
            name
        );
        if let Err(err) = add_drain_pod_annotation(&ctx, &name).await {
            return Err(ReconcileError::KubeError(err));
        }

        ctx.supervisor.write().await.set_is_drain_mode().await;
    }

    // on "terminate" annotation a pod should remove finalizer and kill itself without waiting
    // for processes to finish
    if annotations.is_terminate_mode() && !ctx.supervisor.read().await.is_terminate_mode().await {
        println!(
            "Pod {} is marked for termination. Setting 'terminate' mode...",
            name
        );
        ctx.supervisor.write().await.set_is_terminate_mode().await;
    }

    // Reconcile with changes awaiting
    Ok(Action::await_change())
}

//error processing
fn error_policy(pod: Arc<Pod>, err: &ReconcileError, _ctx: Arc<ReconcileContext>) -> Action {
    println!("Error occurred for Pod {}: {}", pod.name_any(), err);
    Action::requeue(Duration::from_secs(10)) //try again after some delay
}

async fn start_controller(
    k8s_params: Arc<K8sParams>,
    field_selector: Option<String>,
    supervisor: Arc<RwLock<Supervisor>>,
) {
    println!("Starting supervisor pod controller...");

    let pod_api =
        Api::namespaced(k8s_params.get_client(), k8s_params.get_namespace().as_ref()).clone();

    //prepare controller for supervisor pods
    let ctrl = Controller::new(
        pod_api.clone(),
        watcher::Config {
            // label_selector: Some("supervisor=true".to_string()),
            // field_selector: Some(format!("metadata.name={}", get_current_pod_name().unwrap())),
            field_selector,
            ..watcher::Config::default()
        },
    )
    .shutdown_on_signal();

    //running controller. It will watch for changes in supervisor pods and call reconcile function
    //for each pod change (like creation, deletion, update, etc.)
    ctrl.run(
        reconcile,
        error_policy,
        Arc::new(ReconcileContext {
            pods: Arc::new(pod_api),
            k8s_params: k8s_params.clone(),
            supervisor: Arc::clone(&supervisor),
        }),
    )
    .for_each(|event| async {
        match event {
            Ok(obj) => {
                println!(
                    "Reconcile triggered for pod: {}/{}",
                    obj.0.namespace.as_deref().unwrap_or("<no-ns>"),
                    obj.0.name
                );
            }
            Err(e) => {
                println!("Controller error: {:?}", e);
            }
        }
    })
    .await;
}

async fn add_drain_pod_annotation(ctx: &Arc<ReconcileContext>, name: &String) -> kube::Result<Pod> {
    let patch = json!({
        "metadata": {
            "annotations": {
                "drain": "true"
            }
        }
    });
    ctx.pods
        .patch(
            &name,
            &PatchParams::apply("process-supervisor/drain"),
            &Patch::Merge(&patch),
        )
        .await
}

async fn add_finalizer(
    ctx: Arc<ReconcileContext>,
    pod_name: &String,
    finalizers: Vec<String>,
) -> Result<(), ReconcileError> {
    println!("Adding finalizer to pod {}", pod_name);
    let patch = json!({
        "metadata": {
            "finalizers": finalizers
                .into_iter()
                .chain(std::iter::once(FINALIZER_NAME.to_string()))
                .collect::<Vec<_>>()
        }
    });

    let res = ctx
        .pods
        .patch(
            pod_name,
            &PatchParams::apply(FINALIZER_NAME),
            &Patch::Merge(&patch),
        )
        .await;
    match res {
        Ok(_) => {
            println!("Finalizer added to pod {}", pod_name);
            Ok(())
        }
        Err(e) => {
            println!("Failed to add finalizer to pod {}: {}", pod_name, e);
            Err(ReconcileError::KubeError(e))
        }
    }
}
