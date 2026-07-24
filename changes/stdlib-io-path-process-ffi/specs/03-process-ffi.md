# Spec: Process FFI Backend

## ADDED Requirements

### REQ-PROC-01: Command Execution

The runtime SHALL provide C ABI functions for executing system commands, both synchronously and asynchronously.

- `__process_exec(command: *const i8) -> *mut i8` SHALL execute a command via the system shell and return a `ProcessResult` handle
- `__process_exec_with(command: *const i8, cwd: *const i8, env: *mut i8, shell: bool) -> *mut i8` SHALL execute with options
- `__process_create(command: *const i8, cwd: *const i8, env: *mut i8, shell: bool) -> *mut i8` SHALL spawn a child process and return a `Process` handle
- The `ProcessResult` SHALL contain stdout (string), stderr (string), and exit_code (i64) fields
- The `Process` handle SHALL be an opaque pointer to a Rust `Child` process struct
- `__process_exec` SHALL block until the command completes
- `__process_create` SHALL return immediately after spawning
- Functions SHALL throw a ProcessException if the command cannot be launched (e.g., command not found)

#### Scenario: Execute simple command

- **WHEN** `__process_exec` is called with "echo hello"
- **THEN** the function returns a ProcessResult with stdout="hello\n", stderr="", exit_code=0

#### Scenario: Execute failing command

- **WHEN** `__process_exec` is called with a command that exits with code 1
- **THEN** the function returns a ProcessResult with exit_code=1 and stderr containing the error output

#### Scenario: Execute with working directory

- **WHEN** `__process_exec_with` is called with command="ls", cwd="/tmp", and default env/shell
- **THEN** the function lists contents of /tmp in stdout

#### Scenario: Spawn long-running process

- **WHEN** `__process_create` is called with command="sleep 10"
- **THEN** the function returns immediately with a non-null Process handle

---

### REQ-PROC-02: Process Lifecycle

The runtime SHALL provide functions for waiting on and terminating child processes.

- `__process_wait(proc: *mut i8) -> i64` SHALL block until the process exits and return the exit code
- `__process_wait_async(proc: *mut i8) -> *mut i8` SHALL return a Future<i64> for async waiting
- `__process_kill(proc: *mut i8, signal: i64)` SHALL send the specified signal to the process
- `__process_wait` on an already-exited process SHALL return the cached exit code immediately
- `__process_kill` with signal=9 (SIGKILL) SHALL forcefully terminate the process
- `__process_kill` with signal=15 (SIGTERM) SHALL request graceful termination

#### Scenario: Wait for process to exit

- **WHEN** `__process_wait` is called on a Process handle for a short-running command
- **THEN** the function blocks until exit and returns the exit code (0 for success)

#### Scenario: Kill running process

- **WHEN** `__process_kill` is called with signal=9 on a running Process handle
- **THEN** the process is terminated and subsequent `__process_wait` returns quickly with a non-zero exit code

#### Scenario: Wait on already-exited process

- **WHEN** `__process_wait` is called twice on the same Process handle
- **THEN** both calls return the same exit code, with the second call returning immediately

---

### REQ-PROC-03: Process I/O

The runtime SHALL provide functions for interacting with a child process's standard streams.

- `__process_write_input(proc: *mut i8, input: *const i8)` SHALL write data to the process's stdin
- `__process_close_input(proc: *mut i8)` SHALL close the process's stdin pipe
- `__process_read_output(proc: *mut i8) -> *mut i8` SHALL read available data from stdout (non-blocking), returning null if no data
- `__process_read_error(proc: *mut i8) -> *mut i8` SHALL read available data from stderr (non-blocking), returning null if no data
- Functions SHALL throw ProcessException if the process has already exited and pipes are closed

#### Scenario: Write to process stdin

- **WHEN** `__process_write_input` is called with "hello\n" on a Process created for "cat"
- **THEN** the data is written to the process's stdin pipe successfully

#### Scenario: Read process stdout

- **WHEN** `__process_read_output` is called on a Process created for "echo hello" after the process has produced output
- **THEN** the function returns a string containing "hello\n" or null if output hasn't arrived yet

#### Scenario: Read from exited process

- **WHEN** `__process_read_output` is called on a Process that has already exited
- **THEN** the function throws a ProcessException

---

### REQ-PROC-04: Environment Variables

The runtime SHALL provide functions for reading and writing process environment variables.

- `__process_get_env(name: *const i8) -> *mut i8` SHALL return the value of the named environment variable, or null if not set
- `__process_set_env(name: *const i8, value: *const i8)` SHALL set the environment variable for the current process
- `__process_get_all_env() -> *mut i8` SHALL return a Map<string, string> handle containing all environment variables
- All keys and values SHALL be null-terminated UTF-8 strings

#### Scenario: Get existing environment variable

- **WHEN** `__process_get_env` is called with "HOME" on a Unix system where HOME is set
- **THEN** the function returns a non-null string containing the home directory path

#### Scenario: Get non-existent variable

- **WHEN** `__process_get_env` is called with a name that is not set
- **THEN** the function returns a null pointer

#### Scenario: Set and get environment variable

- **WHEN** `__process_set_env("RUYI_TEST_VAR", "hello")` followed by `__process_get_env("RUYI_TEST_VAR")`
- **THEN** the get returns "hello"

---

### REQ-PROC-05: System Information

The runtime SHALL provide functions for querying system and process metadata.

- `__process_get_pid() -> i64` SHALL return the current process ID
- `__process_get_ppid() -> i64` SHALL return the parent process ID
- `__process_get_platform() -> *mut i8` SHALL返回 "macos", "linux", or "unknown"
- `__process_get_cpu_count() -> i64` SHALL return the number of logical CPU cores
- `__process_get_total_memory() -> i64` SHALL return total system memory in bytes
- `__process_get_free_memory() -> i64` SHALL return available system memory in bytes
- All integer returns SHALL be non-negative

#### Scenario: Get process ID

- **WHEN** `__process_get_pid` is called
- **THEN** the function returns a positive integer matching the OS process ID

#### Scenario: Get platform on macOS

- **WHEN** `__process_get_platform` is called on macOS
- **THEN** the function returns the string "macos"

#### Scenario: Get CPU count

- **WHEN** `__process_get_cpu_count` is called
- **THEN** the function returns a positive integer (at least 1)

---

### REQ-PROC-06: Signal Handling

The runtime SHALL provide a function for querying signal availability on the current platform.

- `__process_signal_available(signal: i64) -> bool` SHALL return true if the signal number is valid on the current platform
- SIGKILL (9) and SIGTERM (15) SHALL be available on all Unix platforms
- SIGUSR1 (10) and SIGUSR2 (12) SHALL be available on macOS and Linux

#### Scenario: Check SIGKILL availability

- **WHEN** `__process_signal_available` is called with signal=9 on macOS or Linux
- **THEN** the function returns true

#### Scenario: Check Windows-only signal on Unix

- **WHEN** `__process_signal_available` is called with a Windows-specific signal number on Unix
- **THEN** the function returns false
