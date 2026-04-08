pub struct LaunchResult {
    pid: Option<u32>,
    is_success: bool,
    error_message: Option<String>,
}

impl LaunchResult {
    pub fn new() -> Self {
        Self {
            pid: None,
            is_success: false,
            error_message: None,
        }
    }

    pub fn set_success(&mut self, pid: u32) {
        self.is_success = true;
        self.pid = Some(pid);
    }

    pub fn set_error(&mut self, error_message: String) {
        self.is_success = false;
        self.error_message = Some(error_message);
    }

    pub fn is_success(&self) -> bool {
        self.is_success
    }

    pub fn pid(&self) -> Option<u32> {
        self.pid
    }

    pub fn error_message(&self) -> Option<&String> {
        self.error_message.as_ref()
    }
}

pub struct OldKillResult {
    is_success: bool,
    exit_code: Option<i32>,
    error_message: Option<String>,
}

impl OldKillResult {
    pub fn new() -> Self {
        Self {
            is_success: false,
            exit_code: None,
            error_message: None,
        }
    }

    pub fn set_success(&mut self, exit_code: Option<i32>) {
        self.is_success = true;
        self.exit_code = exit_code;
    }

    pub fn set_error(&mut self, error_message: String) {
        self.is_success = false;
        self.error_message = Some(error_message);
    }

    pub fn is_success(&self) -> bool {
        self.is_success
    }

    pub fn error_message(&self) -> Option<&String> {
        self.error_message.as_ref()
    }
}

pub struct KillResult {
    is_success: bool,
    exit_code: Option<i32>,
    error_message: Option<String>,
}

impl KillResult {
    pub fn new() -> Self {
        Self {
            is_success: false,
            exit_code: None,
            error_message: None,
        }
    }

    pub fn set_success(&mut self, exit_code: Option<i32>) {
        self.is_success = true;
        self.exit_code = exit_code;
    }

    pub fn set_error(&mut self, error_message: String) {
        self.is_success = false;
        self.error_message = Some(error_message);
    }

    pub fn is_success(&self) -> bool {
        self.is_success
    }

    pub fn error_message(&self) -> Option<&String> {
        self.error_message.as_ref()
    }
}

impl Clone for KillResult {
    fn clone(&self) -> Self {
        let mut clone = KillResult::new();
        clone.is_success = self.is_success;
        clone.exit_code = self.exit_code;
        clone.error_message = self.error_message.clone();
        clone
    }
}

pub struct TerminateResult {
    is_success: bool,
    error_message: Option<String>,
}

impl TerminateResult {
    pub fn new() -> Self {
        Self {
            is_success: false,
            error_message: None,
        }
    }

    pub fn set_success(&mut self) {
        self.is_success = true;
    }

    pub fn set_error(&mut self, error_message: String) {
        self.is_success = false;
        self.error_message = Some(error_message);
    }

    pub fn is_success(&self) -> bool {
        self.is_success
    }

    pub fn error_message(&self) -> Option<&String> {
        self.error_message.as_ref()
    }
}
