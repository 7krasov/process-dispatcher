use process_supervisor::env::fetch_env_params;
use process_supervisor::k8s::k8s_common;
use process_supervisor::k8s::k8s_supervisor::{
    mark_itself_as_finished, remove_supervisor_finalizer, start_k8s_cycle,
};
use process_supervisor::server::http::http_server::start_http_server;
use process_supervisor::supervisor::{SlotsPopulationError, Supervisor};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    //obtain environment parameters
    let env_params = fetch_env_params();
    //obtain k8s parameters
    let k8s_params = k8s_common::get_k8s_params().await;

    let supervisor_arc = Arc::new(RwLock::new(Supervisor::new(&env_params)));

    //prepare the kill queue processing task
    let sv_arc = Arc::clone(&supervisor_arc);
    tokio::task::spawn(async move {
        loop {
            let svg = sv_arc.read().await;
            svg.process_kill_queue().await;
            drop(svg);
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });

    //run the k8s cycle if we're within Kubernetes
    if k8s_params.is_some() {
        println!("Kubernetes parameters are available, proceeding with k8s cycle.");
        start_k8s_cycle(
            supervisor_arc.clone(),
            //send just a copy
            Arc::new(k8s_params.clone().unwrap()),
            // pod_name.clone(),
        )
        .await;
    } else {
        println!("Running outside Kubernetes, skipping k8s cycle.");
    }

    //preparing a task to:
    //- populate empty slots with dispatcher processes
    //- clean finished processes
    //- run dispatcher processes if empty slots are available
    let sv_arc = Arc::clone(&supervisor_arc);
    let k8s_params_option_arc = Arc::new(k8s_params);
    tokio::task::spawn(async move {
        let mut is_drain_mode = false;
        loop {
            let sv_g = sv_arc.read().await;
            //clean list from finished processes
            let working_processes_cnt = sv_g.process_states().await;

            //perform only if pod name is available (we're in k8s)
            if k8s_params_option_arc.is_some() {
                //just copy the value from Arc
                let k8s_params = k8s_params_option_arc.as_ref().clone().unwrap();
                let k8s_params_arc = Arc::new(&k8s_params);

                if !is_drain_mode {
                    //obtain new processes if there are empty slots and detect if the drain mode is enabled
                    let result = sv_g.populate_empty_slots().await;
                    drop(sv_g);
                    if let Err(SlotsPopulationError::DrainModeObtained) = result {
                        is_drain_mode = true;
                        println!("Drain mode is caught. Will not populate anymore");
                        continue;
                    }
                } else if working_processes_cnt == 0 {
                    //terminate supervisor pod if is_drain_mode and there are no any working processes left
                    let res = mark_itself_as_finished(Arc::clone(&k8s_params_arc)).await;
                    if res.is_err() {
                        println!("Unable to mark pod as finished: {:?}", res.err());
                        // tokio::time::sleep(Duration::from_secs(5)).await;
                        // continue;
                    }

                    //remove finalizer from the pod so it can be deleted by Kubernetes
                    remove_supervisor_finalizer(Arc::clone(&k8s_params_arc)).await;

                    println!("Terminating supervisor pod...");
                    std::process::exit(0);
                }
            }

            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    });

    let addr: SocketAddr = ([0, 0, 0, 0], env_params.http_port()).into();
    start_http_server(addr, supervisor_arc).await
}
