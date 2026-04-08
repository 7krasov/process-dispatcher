use k8s_openapi::api::core::v1::Pod;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::{Api, Client};
use std::sync::Arc;

const ENV_HOSTNAME: &str = "HOSTNAME";
const NAMESPACE_FILE: &str = "/var/run/secrets/kubernetes.io/serviceaccount/namespace";

pub const FINALIZER_NAME: &str = "process-supervisor/finalizer";

#[derive(Clone)]
pub struct K8sParams {
    pod_name: String,
    namespace: String,
    client: Client,
}

impl K8sParams {
    // pub fn new(pod_name: String, namespace: String, client: Client) -> Self {
    pub fn new(namespace: String, pod_name: String, client: Client) -> Self {
        K8sParams {
            pod_name,
            namespace,
            client,
        }
    }

    // pub fn get_pod_name(&self) -> String {
    //     self.pod_name.clone()
    // }
    // pub fn set_pod_name(&mut self, pod_name: String) {
    //     self.pod_name = pod_name;
    // }
    pub fn get_namespace(&self) -> String {
        self.namespace.clone()
    }
    pub fn get_client(&self) -> Client {
        self.client.clone()
    }

    pub fn get_pod_name(&self) -> String {
        self.pod_name.clone()
    }
}

pub async fn get_k8s_params() -> Option<K8sParams> {
    // let pod_name = get_pod_name();
    // if pod_name.is_err() {
    //     println!("Unable to get pod name");
    //     std::process::exit(1);
    // }
    // let pod_name = pod_name.unwrap();    // let pod_name = get_pod_name();
    // if pod_name.is_err() {
    //     println!("Unable to get pod name");
    //     std::process::exit(1);
    // }
    // let pod_name = pod_name.unwrap();

    //namespace
    let namespace = match get_namespace().await {
        Ok(ns) => ns,
        Err(e) => {
            println!("Unable to get namespace: {:?}", e);
            return None;
        }
    };

    //pod name
    let pod_name = match get_current_pod_name() {
        Ok(pod_name) => pod_name,
        Err(e) => {
            println!("Unable to get pod name: {:?}", e);
            return None;
        }
    };

    //K8s client
    let client = get_client().await;
    if client.is_err() {
        println!("Unable to create kube client");
        // std::process::exit(1);
        return None;
    }
    let client = client.unwrap();

    // K8sParams::new(pod_name, namespace, client)
    Some(K8sParams {
        namespace,
        pod_name,
        client,
    })
}

pub fn get_current_pod_name() -> Result<String, std::io::Error> {
    let pod_name = std::env::var(ENV_HOSTNAME);

    match pod_name {
        Ok(pn) => Ok(pn),
        Err(_) => {
            println!("Unable to get {} env variable value", ENV_HOSTNAME);
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Unable to get pod name",
            ))
        }
    }
}

async fn get_namespace() -> Result<String, std::io::Error> {
    let namespace = tokio::fs::read_to_string(NAMESPACE_FILE).await;

    match namespace {
        Ok(ns) => Ok(ns.trim().to_string()),
        Err(_) => {
            println!("Unable to get namespace");
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Unable to get namespace",
            ))
        }
    }
}

async fn get_client() -> Result<Client, kube::Error> {
    let client = Client::try_default().await;

    match client {
        Ok(c) => Ok(c),
        Err(e) => {
            println!("Unable to create kube client: {:?}", e);
            Err(e)
        }
    }
}

pub struct SupervisorPodAnnotations {
    drain: Option<bool>,
    terminate: Option<bool>,
    finished: Option<bool>,
}

impl SupervisorPodAnnotations {
    pub fn new(drain: Option<bool>, terminate: Option<bool>, finished: Option<bool>) -> Self {
        SupervisorPodAnnotations {
            drain,
            terminate,
            finished,
        }
    }

    pub fn is_drain_mode(&self) -> bool {
        self.drain.unwrap_or(false)
    }
    pub fn is_terminate_mode(&self) -> bool {
        self.terminate.unwrap_or(false)
    }
    pub fn is_finished(&self) -> bool {
        self.finished.unwrap_or(false)
    }
}

pub async fn get_pod_annotations(
    k8s_params: Arc<K8sParams>,
    pod_name: &str,
) -> Result<SupervisorPodAnnotations, anyhow::Error> {
    //getting pod
    let pods: Api<Pod> =
        Api::namespaced(k8s_params.get_client(), k8s_params.get_namespace().as_ref());
    let pod = pods.get(pod_name.as_ref()).await;
    if pod.is_err() {
        // println!("Unable to get pod: {:?}", pod.err());
        return Err(pod.err().unwrap().into());
    }
    Ok(extract_pod_meta_annotations(pod.unwrap().metadata))
}

pub fn extract_pod_meta_annotations(metadata: ObjectMeta) -> SupervisorPodAnnotations {
    let annotations = metadata.annotations.unwrap_or_default();
    SupervisorPodAnnotations::new(
        matches!(annotations.get("drain"), Some(val) if val == "true").into(),
        matches!(annotations.get("terminate"), Some(val) if val == "true").into(),
        matches!(annotations.get("finished"), Some(val) if val == "true").into(),
    )
}
