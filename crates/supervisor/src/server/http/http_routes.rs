use super::http_router::{Handleable, RouteData};
use crate::supervisor::Supervisor;
use async_trait::async_trait;
use bytes::Bytes;
use http_body_util::Full;
use hyper::http::Error;
use hyper::Response;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::Instant;

//"launch" route
#[derive(Debug, Clone)]
pub struct LaunchRoute {
    pub data: RouteData,
}

#[async_trait]
impl Handleable for LaunchRoute {
    fn data(&self) -> RouteData {
        self.data.clone()
    }

    fn clone_box(&self) -> Box<dyn Handleable> {
        Box::new(self.clone())
    }

    async fn handle_data(
        &self,
        route_req_params: HashMap<String, String>,
        _body: String,
        supervisor_arc: Arc<RwLock<Supervisor>>,
    ) -> Result<Response<Full<Bytes>>, Error> {
        let id = route_req_params.get("id").unwrap().parse::<String>()?;

        let supervisor_guard = supervisor_arc.read().await;
        let future = supervisor_guard.launch(id.clone());
        let result = future.await;
        let http_status_code = match result.is_success() {
            true => 200,
            false => 500,
        };
        let message = match result.is_success() {
            true => format!(
                "A process {} was started, PID={}",
                id,
                result.pid().unwrap()
            ),
            false => format!(
                "Failed to start a process for source {}. Error: {}",
                id,
                result.error_message().unwrap()
            ),
        };

        self.prepare_response(message, http_status_code)
    }
}

#[derive(Debug, Clone)]
pub struct GetStateList {
    pub data: RouteData,
}

#[async_trait]
impl Handleable for GetStateList {
    fn data(&self) -> RouteData {
        self.data.clone()
    }
    fn clone_box(&self) -> Box<dyn Handleable> {
        Box::new(self.clone())
    }
    async fn handle_data(
        &self,
        _route_req_params: HashMap<String, String>,
        _body: String,
        supervisor_arc: Arc<RwLock<Supervisor>>,
    ) -> Result<Response<Full<Bytes>>, Error> {
        let before_time = Instant::now();
        println!(
            "GetStateList: before supervisor_clone: {:?}",
            Instant::now().duration_since(before_time)
        );
        let supervisor_arc_clone = {
            println!(
                "GetStateList: before read lock: {:?}",
                Instant::now().duration_since(before_time)
            );
            let supervisor_guard = supervisor_arc.read().await;
            println!(
                "GetStateList: after read lock: {:?}",
                Instant::now().duration_since(before_time)
            );
            Arc::new(supervisor_guard.clone())
        };
        println!(
            "GetStateList: after supervisor_clone: {:?}",
            Instant::now().duration_since(before_time)
        );

        let processes = supervisor_arc_clone.get_state_list().await;
        let json_message = serde_json::to_string(&processes).unwrap();
        self.prepare_response(json_message, 200)
    }
}

//"404" route
#[derive(Debug, Clone)]
pub struct Route404 {
    pub data: RouteData,
}

#[async_trait]
impl Handleable for Route404 {
    fn data(&self) -> RouteData {
        self.data.clone()
    }
    fn clone_box(&self) -> Box<dyn Handleable> {
        Box::new(self.clone())
    }
    // async fn handle_data(&self, body: String) -> Result<Response<Full<Bytes>>, Error> {
    async fn handle_data(
        &self,
        _route_req_params: HashMap<String, String>,
        _body: String,
        _supervisor_arc: Arc<RwLock<Supervisor>>,
    ) -> Result<Response<Full<Bytes>>, Error> {
        self.prepare_response("404".to_owned(), 404)
    }
}

//"terminate" route
#[derive(Debug, Clone)]
pub struct TerminateRoute {
    pub data: RouteData,
}

#[async_trait]
impl Handleable for TerminateRoute {
    fn data(&self) -> RouteData {
        self.data.clone()
    }
    fn clone_box(&self) -> Box<dyn Handleable> {
        Box::new(self.clone())
    }
    async fn handle_data(
        &self,
        route_req_params: HashMap<String, String>,
        _body: String,
        supervisor_arc: Arc<RwLock<Supervisor>>,
    ) -> Result<Response<Full<Bytes>>, Error> {
        let id = route_req_params.get("id").unwrap().parse::<String>()?;
        let supervisor_guard = supervisor_arc.read().await;
        let result = supervisor_guard.terminate(id.clone()).await;

        let http_status_code = match result.is_success() {
            true => 200,
            false => 500,
        };
        let message = match result.is_success() {
            true => format!("A process got the termination signal for source {}", id),
            false => format!(
                "Failed to start a termination of the process for source {}. Error: {:?}",
                id,
                result
                    .error_message()
                    .unwrap_or(&"Unknown error".to_owned())
            ),
        };

        self.prepare_response(message, http_status_code)
    }
}

//"kill" route
#[derive(Debug, Clone)]
pub struct KillRoute {
    pub data: RouteData,
}

#[async_trait]
impl Handleable for KillRoute {
    fn data(&self) -> RouteData {
        self.data.clone()
    }
    fn clone_box(&self) -> Box<dyn Handleable> {
        Box::new(self.clone())
    }
    async fn handle_data(
        &self,
        route_req_params: HashMap<String, String>,
        _body: String,
        supervisor_arc: Arc<RwLock<Supervisor>>,
    ) -> Result<Response<Full<Bytes>>, Error> {
        let id = route_req_params.get("id").unwrap().parse::<String>()?;
        let supervisor_guard = supervisor_arc.read().await;
        let result = supervisor_guard.kill_old(id.clone()).await;

        let http_status_code = match result.is_success() {
            true => 200,
            false => 500,
        };
        let message = match result.is_success() {
            true => format!("A process {} was killed", id),
            false => format!(
                "Failed to kill a process {}. Error: {:?}",
                id,
                result
                    .error_message()
                    .unwrap_or(&"Unknown error".to_owned())
            ),
        };

        self.prepare_response(message, http_status_code)
    }
}
