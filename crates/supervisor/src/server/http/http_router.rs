use async_trait::async_trait;
use bytes::Bytes;
use core::str;
use http_body_util::Full;
use hyper::http::Error;
use hyper::Response;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::supervisor::Supervisor;
#[async_trait]
// pub trait Handleable: Send + Sync + Debug + Clone {
pub trait Handleable: Send + Sync + Debug {
    fn data(&self) -> RouteData;

    fn clone_box(&self) -> Box<dyn Handleable>;
    fn method(&self) -> String {
        String::from(self.data().method.as_str())
    }
    fn path(&self) -> String {
        String::from(self.data().path.as_str())
    }
    fn params(&self) -> Option<HashMap<String, ParamType>> {
        self.data().params.clone()
    }
    async fn handle_data(
        &self,
        _route_req_params: HashMap<String, String>,
        _body: String,
        _supervisor_arc: Arc<RwLock<Supervisor>>,
    ) -> Result<Response<Full<Bytes>>, Error>;

    fn prepare_response(
        &self,
        message: String,
        http_status_code: u16,
    ) -> Result<Response<Full<Bytes>>, Error> {
        let bytes = bytes::Bytes::from(message);
        let body = Full::new(bytes);
        Response::builder()
            .status(http_status_code)
            .body(body.to_owned())
    }
}

impl Clone for Box<dyn Handleable> {
    fn clone(&self) -> Box<dyn Handleable> {
        self.clone_box()
    }
}

#[derive(Clone, Debug)]
pub enum ParamType {
    AnyString,
    Integer,
}

#[derive(Debug, Clone)]
pub struct RouteData {
    pub method: String,
    pub path: String,
    pub params: Option<HashMap<String, ParamType>>,
}

pub fn route_request_params(
    req_path: String,
    route: &Box<dyn Handleable>,
) -> HashMap<String, String> {
    let route_params = match route.params() {
        Some(params) => params,
        None => return HashMap::new(),
    };
    if route_params.is_empty() {
        return HashMap::new();
    }

    let mut route_req_params: HashMap<String, String> = HashMap::new();

    let req_path_segments: Vec<&str> = req_path.trim_matches('/').split('/').collect();
    let path = route.path();
    let route_path_segments: Vec<&str> = path.trim_matches('/').split('/').collect();

    route_path_segments
        .iter()
        .zip(req_path_segments.iter())
        .for_each(|(route_segment, req_segment)| {
            if route_segment.starts_with('{') && route_segment.ends_with('}') {
                let name = route_segment.trim_matches(|c| c == '{' || c == '}');

                let param_type = route_params.get(name);
                if param_type.is_none() {
                    return;
                }

                let param_type = param_type.unwrap();
                match param_type {
                    ParamType::AnyString => {
                        route_req_params.insert(name.to_string(), req_segment.to_string());
                    }
                    ParamType::Integer => {
                        let segment = req_segment.parse::<i32>();
                        if segment.is_err() {
                            return;
                        }
                        route_req_params.insert(name.to_string(), req_segment.to_string());
                    }
                }
            }
        });

    route_req_params
}

pub async fn route(
    req_method: String,
    req_path: String,
    routes: Arc<Vec<Box<dyn Handleable>>>,
) -> Option<Box<dyn Handleable>> {
    let req_path_segments: Vec<&str> = req_path.trim_matches('/').split('/').collect();

    let route_result = routes.iter().find(|route| {
        let path = route.path();
        let route_segments: Vec<&str> = path.trim_matches('/').split('/').collect();
        if route.method() != req_method || req_path_segments.len() != route_segments.len() {
            return false;
        }

        route_segments
            .iter()
            .zip(req_path_segments.iter())
            .all(|(route_segment, path_segment)| {
                if route_segment.starts_with('{') && route_segment.ends_with('}') {
                    // This is a variable segment, consider it a match for any value.
                    true
                } else {
                    // This is a static segment, it must match exactly.
                    route_segment == path_segment
                }
            })
    });

    route_result.map(|route| route.clone_box())
}
