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
                // println!("Failed to start command");
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

        println!(
            "terminate: After getting of child from the process list time: {:?}",
            Instant::now().duration_since(before_time)
        );

        let mut result = TerminateResult::new();
        if child.is_none() {
            result.set_error("Child not found PID for SIGTERM sending".to_owned());
            return result;
        }
        let child = child.unwrap();
        let pid: i32 = child.id() as i32;

        drop(processes_guard);

        println!(
            "terminate: After dropping time: {:?}",
            Instant::now().duration_since(before_time)
        );

        println!("terminate: Sending SIGTERM to PID: {}", pid);
        let signal_result = signal::kill(Pid::from_raw(pid), signal::SIGTERM);
        println!(
            "terminate: After SIGTERM sending: {:?}",
            Instant::now().duration_since(before_time)
        );

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

        println!(
            "kill: After getting of child from the process list time: {:?}",
            Instant::now().duration_since(before_time)
        );

        if child.is_none() {
            let mut result = OldKillResult::new();
            result.set_error("Child not found PID for SIGTERM sending".to_owned());
            return result;
        }
        let child = child.unwrap();
        let pid: i32 = child.id() as i32;

        drop(processes_guard);

        println!(
            "kill: After dropping time: {:?}",
            Instant::now().duration_since(before_time)
        );

        println!("kill: Sending SIGTERM to PID: {}", pid);
        signal::kill(Pid::from_raw(pid), signal::SIGTERM).unwrap();
        println!(
            "kill: After SIGTERM sending: {:?}",
            Instant::now().duration_since(before_time)
        );
        let duration = Duration::from_secs(self.sig_term_timeout);
        let deadline = Instant::now() + duration;
        println!("kill: Sleeping until: {:?}", deadline);
        sleep_until(deadline).await;
        println!(
            "kill: After sleep_until awaiting: {:?}",
            Instant::now().duration_since(before_time)
        );

        let mut result = OldKillResult::new();
        //after SIGTERM timeout we should send SIGKILL signal to make sure the process will be terminated
        let processes_arc = self.processes.clone();
        println!(
            "kill: After new processes_arc cloning: {:?}",
            Instant::now().duration_since(before_time)
        );
        let mut processes_guard = processes_arc.write().await;
        println!(
            "kill: After new processes_guard awaiting: {:?}",
            Instant::now().duration_since(before_time)
        );
        println!(
            "kill: After new result_guard awaiting: {:?}",
            Instant::now().duration_since(before_time)
        );
        //extract child PID from the processes
        let child = processes_guard.get_mut(&id);
        if child.is_none() {
            result.set_error("Child not found PID for SIGKILL sending".to_owned());
            return result;
        }
        let child = child.unwrap();

        //send SIGKILL (9) signal
        println!("Sending SIGKILL to PID: {}", pid);
        let kill_result = child.kill();
        println!(
            "After kill time: {:?}",
            Instant::now().duration_since(before_time)
        );

        match kill_result {
            Ok(_) => {
                let exit_status = child.try_wait();
                println!(
                    "kill: After try_wait time: {:?}",
                    Instant::now().duration_since(before_time)
                );

                let ch = processes_guard.remove(&id);
                println!(
                    "After remove time: {:?}",
                    Instant::now().duration_since(before_time)
                );
                if ch.is_none() {
                    println!("Failed to remove child {} from processes", id);
                }
                drop(processes_guard);

                match exit_status {
                    Ok(status) => {
                        match status {
                            Some(_) => {
                                println!("Status code; {:?}", status.unwrap().code());
                                result.set_success(status.unwrap().code());
                            }
                            None => {
                                //probably, the process was finished before the killing signal sending
                                result.set_success(Some(9999999));
                            }
                        };
                    }
                    Err(e) => {
                        println!("Error: {}", e);
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
            println!("There is some time left before SIGKILL sending. Sleeping...");
            let sleep_time = self.sig_term_timeout - wait_time_elapsed;
            println!("kill: Sleeping seconds: {:?}", sleep_time);
            sleep(Duration::from_secs(sleep_time)).await;
            println!("kill: After sleep awaiting: {:?}", now.elapsed());
        }

        let before_time = Instant::now();

        let mut result = KillResult::new();
        //after SIGTERM timeout we should send SIGKILL signal to make sure the process will be terminated
        let processes_arc = self.processes.clone();
        println!(
            "kill: After new processes_arc cloning: {:?}",
            Instant::now().duration_since(before_time)
        );
        let mut processes_guard = processes_arc.write().await;
        println!(
            "kill: After new processes_guard awaiting: {:?}",
            Instant::now().duration_since(before_time)
        );
        //extract child PID from the processes
        let child = processes_guard.get_mut(&id);
        if child.is_none() {
            result.set_error("Child not found PID for SIGKILL sending.".to_owned());
            return result;
        }
        let child = child.unwrap();

        let state = get_child_state(id.clone(), child).unwrap();
        if state.is_finished {
            println!(
                "It seems the process finished itself. Exit code: {:?}",
                state.exit_code
            );
            result.set_success(state.exit_code);
            //we use process_states() to clean up the processes list
            // let ch = processes_guard.remove(&id);
            // println!(
            //     "After child remove time: {:?}",
            //     Instant::now().duration_since(before_time)
            // );
            // if ch.is_none() {
            //     println!("Failed to remove child for process {}", id);
            // }
            return result;
        }

        //send SIGKILL (9) signal
        println!("Sending SIGKILL to PID: {}", child.id());
        let kill_result = child.kill();
        println!(
            "After kill time: {:?}",
            Instant::now().duration_since(before_time)
        );

        match kill_result {
            Ok(_) => {
                let exit_status = child.try_wait();
                println!(
                    "kill: After try_wait time: {:?}",
                    Instant::now().duration_since(before_time)
                );

                //we use process_states() to clean up the processes list
                // let ch = processes_guard.remove(&id);
                // println!(
                //     "After child remove time: {:?}",
                //     Instant::now().duration_since(before_time)
                // );
                // if ch.is_none() {
                //     println!("Failed to remove child for processes {}", id);
                // }
                // drop(processes_guard);

                match exit_status {
                    Ok(status) => {
                        match status {
                            Some(_) => {
                                println!("Status code; {:?}", status.unwrap().code());
                                result.set_success(status.unwrap().code());
                            }
                            None => {
                                //probably, the process was finished before the killing signal sending
                                result.set_success(Some(9999999));
                            }
                        };
                    }
                    Err(e) => {
                        println!("Error: {}", e);
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
        println!(
            "get_state_list: Before self.processes.clone(), time: {:?}",
            Instant::now().duration_since(before_time)
        );
        let processes_arc = self.processes.clone();
        println!(
            "get_state_list: After self.processes.clone(), time: {:?}",
            Instant::now().duration_since(before_time)
        );
        let processes_guard = processes_arc.read().await;
        println!(
            "get_state_list: After processes_arc.read().await, time: {:?}",
            Instant::now().duration_since(before_time)
        );
        let keys: Vec<String> = processes_guard.keys().cloned().collect();
        drop(processes_guard);

        let futures: Vec<_> = keys
            .into_iter()
            .map(|id| {
                let supervisor = self.clone();
                task::spawn(async move {
                    let res = supervisor.get_process_state(id.clone()).await;
                    res.unwrap_or_else(|e| {
                        println!("get_child_state returned error: {}", e);
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

        println!(
            "get_state_list: After futures collect, time: {:?}",
            Instant::now().duration_since(before_time)
        );

        let mut states = HashMap::new();
        for future in futures {
            match future.await {
                Ok(state) => {
                    println!(
                        "get_state_list: Before states.insert, time: {:?}",
                        Instant::now().duration_since(before_time)
                    );
                    states.insert(state.id.clone(), state);
                }
                Err(e) => {
                    println!("get_state_list: Task Join Error: {}", e);
                }
            }
        }

        states

        // return processes_guard.keys().map(|source_id| {
        //     self.get_child_state(*source_id).await;
        // }).collect();
        // let keys = processes_guard.keys();
        // return keys.map(|(source_id)| {
        //     self.get_child_state(*source_id).unwrap()
        // }).collect();
    }

    pub async fn get_process_state(&self, id: String) -> Result<ChildState, Error> {
        let before_time = Instant::now();
        println!(
            "get_process_state: Before self.processes.clone(), time: {:?}",
            Instant::now().duration_since(before_time)
        );
        let processes_arc = self.processes.clone();
        println!(
            "get_process_state: After self.processes.clone(), time: {:?}",
            Instant::now().duration_since(before_time)
        );
        let mut processes_guard = processes_arc.write().await;
        println!(
            "get_process_state: After processes_arc.write().await, time: {:?}",
            Instant::now().duration_since(before_time)
        );
        let child = processes_guard
            .get_mut(&id)
            .ok_or_else(|| Error::new(std::io::ErrorKind::NotFound, "Child not found"))?;
        println!(
            "get_process_state: After processes_guard.get_mut, time: {:?}",
            Instant::now().duration_since(before_time)
        );

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
        // drop(kill_queue_guard);
        Some((id, terminate_signal_time))
    }

    //cleans up the processes list from finished processes and returns the number of processes left
    pub async fn process_states(&self) -> usize {
        println!("Processing child states...");
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
                println!("Child {} not found in the process list", id.clone());
                drop(ps_g);
                continue;
            }

            let state = get_child_state(id.clone(), child.unwrap());
            drop(ps_g);
            if state.is_err() {
                println!("Error getting child state: {}", state.err().unwrap());
                continue;
            }
            let state = state.unwrap();

            if !state.is_finished {
                println!("Process {} is still running.", id);
                continue;
            }

            println!(
                "Process {} finished with exit code: {:?}. Reporting to the dispatcher...",
                id, state.exit_code
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
                println!("Failed to report process finish: {:?}", report_result.err());
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                continue;
            }
            println!("Process {:?} finish reported successfully. Removing...", id);
            let mut ps_g = ps_arc.write().await;
            ps_g.remove(&id);
            working_processes_cnt -= 1;
            drop(ps_g);
            println!("Process {:?} removed successfully.", id);
        }
        println!("Child states processing is finished.");
        working_processes_cnt
    }

    ///if empty processed slots exist, fetches new processes from dispatcher and run them
    pub async fn populate_empty_slots(&self) -> Result<(), SlotsPopulationError> {
        println!("Populating empty slots...");
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
            //all slots are occupied
            println!("All slots are occupied. Nothing to do.");
            return Ok(());
        }

        if processes_count < self.max_children_count {
            //some slots are empty
            println!(
                "Empty slots available: {}. Populating...",
                self.max_children_count - processes_count
            );
        }

        for _ in processes_count..self.max_children_count {
            //check if we are in drain mode
            let is_drain_mode_guard = self.is_drain_mode.read().await;
            if *is_drain_mode_guard {
                return Err(SlotsPopulationError::DrainModeObtained);
            }

            println!("Sleeping...");
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            let assigned_process = self.dispatcher_client.obtain_new_process().await;
            if assigned_process.is_err() {
                println!("Failed to obtain new process: {:?}", assigned_process.err());
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                continue;
            }
            let assigned_process = assigned_process.unwrap();
            // let supervisor = self.clone();
            // let result = supervisor.launch(new_process.id.clone()).await;
            let result = self.launch(assigned_process.id.clone()).await;
            // drop(supervisor);
            if result.is_success() {
                println!(
                    "Process {:?} for source {:?} launched successfully",
                    assigned_process.id, assigned_process.source_id
                );
                continue;
            }
            println!("Failed to launch child: {:?}", result.error_message());
        }
        println!("Populating empty slots is finished.");
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
    println!(
        "get_child_state: After child.try_wait()?, time: {:?}",
        Instant::now().duration_since(before_time)
    );
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
