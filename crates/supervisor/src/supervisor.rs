use crate::dispatcher;
use crate::dispatcher::DispatcherClient;
use crate::env::EnvParams;
use nix::sys::signal::{self};
use nix::unistd::Pid;
#[cfg(target_os = "linux")]
use procfs::process::Process;
use results::TerminateResult;
use results::{KillResult, LaunchResult, OldKillResult};
use serde::Serialize;
use std::collections::HashMap;
use std::fmt;
use std::io::Error;
use std::process::{Child, Command};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use tokio::task;
use tokio::time::{sleep, sleep_until, Duration, Instant};
use tracing::{debug, error, info, trace, warn};

mod results;

#[derive(Debug, Serialize)]
pub struct ChildState {
    id: String,
    is_running: bool,
    is_finished: bool,
    exit_code: Option<i32>,
    is_killed: bool,
    rss_anon_memory_kb: Option<u64>,
}

impl ChildState {
    pub fn is_finished(&self) -> bool {
        self.is_finished
    }
}

impl fmt::Display for ChildState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "is_running: {}, is_finished: {}, exit_code: {:?}, is_killed: {}, rss_anon_memory_kb: {:?}",
               self.is_running, self.is_finished, self.exit_code, self.is_killed, self.rss_anon_memory_kb)
    }
}

#[derive(Debug)]
pub struct Supervisor {
    dispatcher_client: DispatcherClient,
    processes: Arc<RwLock<HashMap<String, Child>>>,
    kill_queue: Arc<RwLock<HashMap<String, u64>>>,
    is_drain_mode: Arc<RwLock<bool>>,
    is_terminate_mode: Arc<RwLock<bool>>,
    max_children_count: usize,
    sig_term_timeout: u64,
}

impl Supervisor {
    pub fn new(env_params: &EnvParams) -> Self {
        Self {
            dispatcher_client: DispatcherClient::new(env_params),
            processes: Arc::new(RwLock::new(HashMap::new())),
            kill_queue: Arc::new(RwLock::new(HashMap::new())),
            is_drain_mode: Arc::new(RwLock::new(false)),
            is_terminate_mode: Arc::new(RwLock::new(false)),
            max_children_count: env_params.max_children_count(),
            sig_term_timeout: env_params.sigterm_timeout_secs(),
        }
    }

    pub async fn launch(&self, id: String) -> LaunchResult {
        let mut command = Command::new("php");
        command.arg("worker/worker.php");

        let mut result = LaunchResult::new();

        let spawn_result = command.spawn();
        match spawn_result {
            Ok(child) => {
                let pid = child.id();
                let processes_arc = self.processes.clone();
                processes_arc.write().await.insert(id, child);
                drop(processes_arc);
                result.set_success(pid);
                result
            }
            Err(e) => {
                result.set_error(e.to_string());
                result
            }
        }
    }

    pub async fn terminate(&self, id: String) -> TerminateResult {
        let before_time = Instant::now();

        let processes_arc = self.processes.clone();
        let mut processes_guard = processes_arc.write().await;

        //extract child PID from the processes
        let child = processes_guard.get_mut(&id);

        trace!(elapsed = ?Instant::now().duration_since(before_time), "terminate: after getting child from process list");

        let mut result = TerminateResult::new();
        if child.is_none() {
            result.set_error("Child not found PID for SIGTERM sending".to_owned());
            return result;
        }
        let child = child.unwrap();
        let pid: i32 = child.id() as i32;

        drop(processes_guard);

        trace!(elapsed = ?Instant::now().duration_since(before_time), "terminate: after dropping guard");

        info!(pid, "terminate: sending SIGTERM");
        let signal_result = signal::kill(Pid::from_raw(pid), signal::SIGTERM);
        trace!(elapsed = ?Instant::now().duration_since(before_time), "terminate: after SIGTERM sending");

        match signal_result {
            Ok(_) => {
                let start = SystemTime::now();
                self.kill_queue.clone().write().await.insert(
                    id,
                    start
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                );
                result.set_success();
                result
            }
            Err(e) => {
                result.set_error(e.to_string());
                result
            }
        }
    }

    pub async fn kill_old(&self, id: String) -> OldKillResult {
        let before_time = Instant::now();

        let processes_arc = self.processes.clone();
        let mut processes_guard = processes_arc.write().await;

        //extract child PID from the processes
        let child = processes_guard.get_mut(&id);

        trace!(elapsed = ?Instant::now().duration_since(before_time), "kill: after getting child from process list");

        if child.is_none() {
            let mut result = OldKillResult::new();
            result.set_error("Child not found PID for SIGTERM sending".to_owned());
            return result;
        }
        let child = child.unwrap();
        let pid: i32 = child.id() as i32;

        drop(processes_guard);

        trace!(elapsed = ?Instant::now().duration_since(before_time), "kill: after dropping guard");

        info!(pid, "kill: sending SIGTERM");
        signal::kill(Pid::from_raw(pid), signal::SIGTERM).unwrap();
        trace!(elapsed = ?Instant::now().duration_since(before_time), "kill: after SIGTERM sending");
        let duration = Duration::from_secs(self.sig_term_timeout);
        let deadline = Instant::now() + duration;
        trace!(?deadline, "kill: sleeping until deadline");
        sleep_until(deadline).await;
        trace!(elapsed = ?Instant::now().duration_since(before_time), "kill: after sleep_until");

        let mut result = OldKillResult::new();
        //after SIGTERM timeout we should send SIGKILL signal to make sure the process will be terminated
        let processes_arc = self.processes.clone();
        trace!(elapsed = ?Instant::now().duration_since(before_time), "kill: after processes_arc cloning");
        let mut processes_guard = processes_arc.write().await;
        trace!(elapsed = ?Instant::now().duration_since(before_time), "kill: after processes_guard awaiting");
        //extract child PID from the processes
        let child = processes_guard.get_mut(&id);
        if child.is_none() {
            result.set_error("Child not found PID for SIGKILL sending".to_owned());
            return result;
        }
        let child = child.unwrap();

        //send SIGKILL (9) signal
        info!(pid, "Sending SIGKILL");
        let kill_result = child.kill();
        trace!(elapsed = ?Instant::now().duration_since(before_time), "after kill");

        match kill_result {
            Ok(_) => {
                let exit_status = child.try_wait();
                trace!(elapsed = ?Instant::now().duration_since(before_time), "kill: after try_wait");

                let ch = processes_guard.remove(&id);
                trace!(elapsed = ?Instant::now().duration_since(before_time), "after remove");
                if ch.is_none() {
                    warn!(id, "Failed to remove child from processes");
                }
                drop(processes_guard);

                match exit_status {
                    Ok(status) => {
                        match status {
                            Some(_) => {
                                debug!(exit_code = ?status.unwrap().code(), "Process exit status");
                                result.set_success(status.unwrap().code());
                            }
                            None => {
                                //probably, the process was finished before the killing signal sending
                                result.set_success(Some(9999999));
                            }
                        };
                    }
                    Err(e) => {
                        error!(%e, "Error getting exit status");
                        result.set_error(e.to_string());
                    }
                }
            }
            Err(e) => {
                result.set_error(e.to_string());
            }
        }
        result
    }

    pub async fn kill(&self, id: String, terminate_signal_time: u64) -> KillResult {
        let now = SystemTime::now();
        let wait_time_elapsed = now
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - terminate_signal_time;

        if wait_time_elapsed < self.sig_term_timeout {
            let sleep_time = self.sig_term_timeout - wait_time_elapsed;
            debug!(sleep_time, "Time left before SIGKILL, sleeping...");
            sleep(Duration::from_secs(sleep_time)).await;
            trace!(elapsed = ?now.elapsed(), "kill: after sleep");
        }

        let before_time = Instant::now();

        let mut result = KillResult::new();
        //after SIGTERM timeout we should send SIGKILL signal to make sure the process will be terminated
        let processes_arc = self.processes.clone();
        trace!(elapsed = ?Instant::now().duration_since(before_time), "kill: after processes_arc cloning");
        let mut processes_guard = processes_arc.write().await;
        trace!(elapsed = ?Instant::now().duration_since(before_time), "kill: after processes_guard awaiting");
        //extract child PID from the processes
        let child = processes_guard.get_mut(&id);
        if child.is_none() {
            result.set_error("Child not found PID for SIGKILL sending.".to_owned());
            return result;
        }
        let child = child.unwrap();

        let state = get_child_state(id.clone(), child).unwrap();
        if state.is_finished {
            info!(id, exit_code = ?state.exit_code, "Process finished itself before SIGKILL");
            result.set_success(state.exit_code);
            return result;
        }

        //send SIGKILL (9) signal
        let child_pid = child.id();
        info!(pid = child_pid, "Sending SIGKILL");
        let kill_result = child.kill();
        trace!(elapsed = ?Instant::now().duration_since(before_time), "after kill");

        match kill_result {
            Ok(_) => {
                let exit_status = child.try_wait();
                trace!(elapsed = ?Instant::now().duration_since(before_time), "kill: after try_wait");

                match exit_status {
                    Ok(status) => {
                        match status {
                            Some(_) => {
                                debug!(exit_code = ?status.unwrap().code(), "Process exit status");
                                result.set_success(status.unwrap().code());
                            }
                            None => {
                                //probably, the process was finished before the killing signal sending
                                result.set_success(Some(9999999));
                            }
                        };
                    }
                    Err(e) => {
                        error!(%e, "Error getting exit status");
                        result.set_error(e.to_string());
                    }
                };
            }
            Err(e) => {
                result.set_error(e.to_string());
            }
        }

        result
    }

    pub async fn get_state_list(self: Arc<Self>) -> HashMap<String, ChildState> {
        let before_time = Instant::now();
        let processes_arc = self.processes.clone();
        let processes_guard = processes_arc.read().await;
        trace!(elapsed = ?Instant::now().duration_since(before_time), "get_state_list: after read lock");
        let keys: Vec<String> = processes_guard.keys().cloned().collect();
        drop(processes_guard);

        let futures: Vec<_> = keys
            .into_iter()
            .map(|id| {
                let supervisor = self.clone();
                task::spawn(async move {
                    let res = supervisor.get_process_state(id.clone()).await;
                    res.unwrap_or_else(|e| {
                        error!(%e, "get_child_state returned error");
                        ChildState {
                            id: id.clone(),
                            is_running: false,
                            is_finished: false,
                            exit_code: None,
                            is_killed: false,
                            rss_anon_memory_kb: None,
                        }
                    })
                })
            })
            .collect();

        trace!(elapsed = ?Instant::now().duration_since(before_time), "get_state_list: after futures collect");

        let mut states = HashMap::new();
        for future in futures {
            match future.await {
                Ok(state) => {
                    trace!(elapsed = ?Instant::now().duration_since(before_time), "get_state_list: before states.insert");
                    states.insert(state.id.clone(), state);
                }
                Err(e) => {
                    error!(%e, "get_state_list: task join error");
                }
            }
        }

        states
    }

    pub async fn get_process_state(&self, id: String) -> Result<ChildState, Error> {
        let before_time = Instant::now();
        let processes_arc = self.processes.clone();
        let mut processes_guard = processes_arc.write().await;
        trace!(elapsed = ?Instant::now().duration_since(before_time), "get_process_state: after write lock");
        let child = processes_guard
            .get_mut(&id)
            .ok_or_else(|| Error::new(std::io::ErrorKind::NotFound, "Child not found"))?;
        trace!(elapsed = ?Instant::now().duration_since(before_time), "get_process_state: after get_mut");

        get_child_state(id, child)
    }

    pub async fn process_kill_queue(&self) {
        let option: Option<(String, u64)> = self.pop_kill_queue().await;
        if option.is_none() {
            return;
        }

        let (id, terminate_signal_time) = option.unwrap();
        self.kill(id, terminate_signal_time).await;
    }

    pub async fn pop_kill_queue(&self) -> Option<(String, u64)> {
        let mut kill_queue_guard = self.kill_queue.write().await;
        let (id, terminate_signal_time) = kill_queue_guard
            .iter()
            .next()
            .map(|(k, v)| (k.clone(), *v))?;
        kill_queue_guard.remove(&id);
        Some((id, terminate_signal_time))
    }

    //cleans up the processes list from finished processes and returns the number of processes left
    pub async fn process_states(&self) -> usize {
        debug!("Processing child states...");
        let ps_arc = self.processes.clone();

        let ps_g = ps_arc.read().await;
        let ids: Vec<String> = ps_g.keys().cloned().collect();
        drop(ps_g);

        let mut working_processes_cnt = ids.len();
        for id in ids {
            let mut ps_g = ps_arc.write().await;
            let child = ps_g.get_mut(&id);
            if child.is_none() {
                working_processes_cnt -= 1;
                warn!(id, "Child not found in the process list");
                drop(ps_g);
                continue;
            }

            let state = get_child_state(id.clone(), child.unwrap());
            drop(ps_g);
            if state.is_err() {
                error!(err = %state.err().unwrap(), "Error getting child state");
                continue;
            }
            let state = state.unwrap();

            if !state.is_finished {
                debug!(id, "Process is still running");
                continue;
            }

            info!(
                id,
                exit_code = ?state.exit_code,
                "Process finished. Reporting to the dispatcher..."
            );
            let exit_code = state.exit_code.unwrap_or(0);
            //TODO: report kill status to dispatcher
            let process_result = match exit_code {
                0 => dispatcher::REPORT_STATUS_SUCCESS.to_string(),
                _ => dispatcher::REPORT_STATUS_ERROR.to_string(),
            };
            let report = dispatcher::ProcessFinishReport::new(id.clone(), process_result);
            let report_result = self.dispatcher_client.report_process_finish(report).await;
            if report_result.is_err() {
                error!(err = ?report_result.err(), "Failed to report process finish");
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                continue;
            }
            info!(id, "Process finish reported successfully. Removing...");
            let mut ps_g = ps_arc.write().await;
            ps_g.remove(&id);
            working_processes_cnt -= 1;
            drop(ps_g);
            debug!(id, "Process removed successfully");
        }
        debug!("Child states processing is finished");
        working_processes_cnt
    }

    ///if empty processed slots exist, fetches new processes from dispatcher and run them
    pub async fn populate_empty_slots(&self) -> Result<(), SlotsPopulationError> {
        debug!("Populating empty slots...");
        //check if we are in drain mode
        let is_drain_mode_guard = self.is_drain_mode.read().await;
        if *is_drain_mode_guard {
            return Err(SlotsPopulationError::DrainModeObtained);
        }

        let processes_arc = self.processes.clone();
        let processes_guard = processes_arc.read().await;
        let processes_count = processes_guard.len();
        drop(processes_guard);

        if processes_count >= self.max_children_count {
            debug!("All slots are occupied. Nothing to do.");
            return Ok(());
        }

        info!(
            empty_slots = self.max_children_count - processes_count,
            "Populating empty slots..."
        );

        for _ in processes_count..self.max_children_count {
            //check if we are in drain mode
            let is_drain_mode_guard = self.is_drain_mode.read().await;
            if *is_drain_mode_guard {
                return Err(SlotsPopulationError::DrainModeObtained);
            }

            debug!("Sleeping before obtaining new process...");
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            let assigned_process = self.dispatcher_client.obtain_new_process().await;
            if assigned_process.is_err() {
                error!(err = ?assigned_process.err(), "Failed to obtain new process");
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                continue;
            }
            let assigned_process = assigned_process.unwrap();
            let result = self.launch(assigned_process.id.clone()).await;
            if result.is_success() {
                info!(
                    process_id = assigned_process.id,
                    source_id = assigned_process.source_id,
                    "Process launched successfully"
                );
                continue;
            }
            error!(err = ?result.error_message(), "Failed to launch child");
        }
        debug!("Populating empty slots is finished");
        Ok(())
    }

    pub async fn set_is_drain_mode(&self) {
        let mut is_drain_mode_guard = self.is_drain_mode.write().await;
        *is_drain_mode_guard = true;
    }

    pub async fn is_drain_mode(&self) -> bool {
        let is_drain_mode_guard = self.is_drain_mode.read().await;
        *is_drain_mode_guard
    }

    pub async fn set_is_terminate_mode(&self) {
        let mut is_terminate_mode_guard = self.is_terminate_mode.write().await;
        *is_terminate_mode_guard = true;
    }
    pub async fn is_terminate_mode(&self) -> bool {
        let is_terminate_mode_guard = self.is_terminate_mode.read().await;
        *is_terminate_mode_guard
    }
}

impl Clone for Supervisor {
    fn clone(&self) -> Self {
        Self {
            dispatcher_client: self.dispatcher_client.clone(),
            processes: Arc::clone(&self.processes),
            kill_queue: Arc::clone(&self.kill_queue),
            is_drain_mode: Arc::clone(&self.is_drain_mode),
            is_terminate_mode: Arc::clone(&self.is_terminate_mode),
            max_children_count: self.max_children_count,
            sig_term_timeout: self.sig_term_timeout,
        }
    }
}

pub enum SlotsPopulationError {
    DrainModeObtained,
}

fn get_child_state(id: String, child: &mut Child) -> Result<ChildState, Error> {
    let before_time = Instant::now();
    let exit_status = child.try_wait()?;
    trace!(elapsed = ?Instant::now().duration_since(before_time), "get_child_state: after try_wait");
    let is_finished = exit_status.is_some();
    let exit_code = exit_status.and_then(|status| status.code());

    #[cfg(not(target_os = "linux"))]
    let memory_kb = None;
    #[cfg(target_os = "linux")]
    let memory_kb = get_memory_usage(child.id()).ok();

    Ok(ChildState {
        id,
        is_running: !is_finished,
        is_finished,
        exit_code,
        is_killed: false,
        rss_anon_memory_kb: memory_kb,
    })
}

///returns size in kilobytes
#[cfg(target_os = "linux")]
fn get_memory_usage(pid: u32) -> std::io::Result<u64> {
    if cfg!(target_os = "linux") {
        let process = Process::new(pid as i32);
        if process.is_err() {
            return Ok(0);
        }
        let process = process.unwrap();

        let status = process.status();
        if status.is_err() {
            return Ok(0);
        }
        let status = status.unwrap();
        let rssanon = status.rssanon;
        if rssanon.is_none() {
            return Ok(0);
        }
        return Ok(status.rssanon.unwrap());
    }

    //not linux
    return Ok(0);
}
