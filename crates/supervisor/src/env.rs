use crate::dispatcher::{DEFAULT_OBTAIN_PROCESS_URL, DEFAULT_REPORT_PROCESS_FINISH_URL};
use std::env;

pub struct EnvParams {
    http_port: u16,
    sigterm_timeout_secs: u64,
    max_children_count: usize,
    obtain_process_url: String,
    report_process_finish_url: String,
    supervisor_id: String,
}

impl EnvParams {
    pub fn http_port(&self) -> u16 {
        self.http_port
    }

    pub fn sigterm_timeout_secs(&self) -> u64 {
        self.sigterm_timeout_secs
    }

    pub fn max_children_count(&self) -> usize {
        self.max_children_count
    }
    pub fn obtain_process_url(&self) -> &str {
        &self.obtain_process_url
    }
    pub fn report_process_finish_url(&self) -> &str {
        &self.report_process_finish_url
    }

    pub fn supervisor_id(&self) -> &str {
        &self.supervisor_id
    }
}

pub fn fetch_env_params() -> EnvParams {
    let http_port: u16 = match env::var("HTTP_PORT") {
        Ok(port) => port.parse::<u16>().unwrap(),
        Err(_) => {
            println!("HTTP_PORT is not set. Using default 8080");
            8080
        }
    };

    let sigterm_timeout_secs: u64 = match env::var("SIGTERM_TIMEOUT_SECS") {
        Ok(timeout) => timeout.parse::<u64>().unwrap(),
        Err(_) => {
            println!("SIGTERM_TIMEOUT_SECS is not set. Using default 20");
            20
        }
    };

    let max_children_count: usize = match env::var("MAX_CHILDREN_COUNT") {
        Ok(count) => count.parse::<usize>().unwrap(),
        Err(_) => {
            println!("MAX_CHILDREN_COUNT is not set. Using default 10");
            10
        }
    };

    let obtain_process_url: String = env::var("OBTAIN_PROCESS_URL").unwrap_or_else(|_| {
        println!(
            "OBTAIN_PROCESS_URL is not set. Using default {}",
            DEFAULT_OBTAIN_PROCESS_URL
        );
        DEFAULT_OBTAIN_PROCESS_URL.to_string()
    });

    let report_process_finish_url: String =
        env::var("REPORT_PROCESS_FINISH_URL").unwrap_or_else(|_| {
            println!(
                "REPORT_PROCESS_FINISH_URL is not set. Using default {}",
                DEFAULT_REPORT_PROCESS_FINISH_URL
            );
            DEFAULT_REPORT_PROCESS_FINISH_URL.to_string()
        });

    let supervisor_id: String =
        env::var("HOST_NAME").expect("HOST_NAME is not set, please set it to supervisor id");

    EnvParams {
        http_port,
        sigterm_timeout_secs,
        max_children_count,
        obtain_process_url,
        report_process_finish_url,
        supervisor_id,
    }
}
