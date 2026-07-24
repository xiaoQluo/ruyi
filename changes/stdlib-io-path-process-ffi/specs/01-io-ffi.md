# Spec: IO FFI Backend

## ADDED Requirements

### REQ-IO-01: File Reading

The runtime SHALL provide C ABI functions for reading file contents, both synchronously and asynchronously.

- `__io_file_read_text(path: *const i8) -> *mut i8` SHALL read entire file contents into a null-terminated UTF-8 string
- `__io_file_read_text_async(path: *const i8) -> *mut i8` SHALL return a Future handle that resolves to file contents
- `__io_file_read_lines(path: *const i8) -> *mut i8` SHALL read file contents as an array of lines (string array handle)
- `__io_file_read_lines_async(path: *const i8) -> *mut i8` SHALL return a Future handle resolving to array of lines
- All functions SHALL throw an IOError via `ruyi_throw` when the file does not exist or cannot be read
- Returned strings SHALL be heap-allocated via the runtime allocator

#### Scenario: Read existing text file

- **WHEN** `__io_file_read_text` is called with a valid path to an existing UTF-8 text file
- **THEN** the function returns a heap-allocated null-terminated string containing the file's full content

#### Scenario: Read non-existent file

- **WHEN** `__io_file_read_text` is called with a path to a file that does not exist
- **THEN** the function throws an IOError via the Ruyi exception mechanism with a descriptive message

#### Scenario: Read file as lines

- **WHEN** `__io_file_read_lines` is called with a path to a file containing "line1\nline2\nline3"
- **THEN** the function returns an array handle with 3 string elements: "line1", "line2", "line3" (no trailing newline)

#### Scenario: Async read resolves to content

- **WHEN** `__io_file_read_text_async` is called and the returned Future is awaited to completion
- **THEN** the Future resolves to a string containing the file's full content

---

### REQ-IO-02: File Writing

The runtime SHALL provide C ABI functions for writing string content to files.

- `__io_file_write_text(path: *const i8, content: *const i8)` SHALL write content to the specified path, creating the file if it does not exist and truncating if it does
- `__io_file_write_text_async(path: *const i8, content: *const i8) -> *mut i8` SHALL return a Future handle for async write
- Functions SHALL throw IOError when the path is not writable (permission denied, read-only filesystem, etc.)
- The `async` variant SHALL not block the calling thread

#### Scenario: Write to new file

- **WHEN** `__io_file_write_text` is called with a path that does not exist and valid UTF-8 content
- **THEN** the file is created with the given content, and the function returns normally

#### Scenario: Write to read-only location

- **WHEN** `__io_file_write_text` is called with a path in a read-only directory
- **THEN** the function throws an IOError

---

### REQ-IO-03: File Metadata

The runtime SHALL provide functions for querying filesystem metadata.

- `__io_file_exists(path: *const i8) -> bool` SHALL return true if the path exists (file, directory, or symlink)
- `__io_file_exists_async(path: *const i8) -> *mut i8` SHALL return a Future<bool>
- `__io_is_file(path: *const i8) -> bool` SHALL return true only for regular files
- `__io_is_directory(path: *const i8) -> bool` SHALL return true only for directories
- Each SHALL have a corresponding `_async` variant returning `Future<bool>`
- Symlink dereferencing behavior SHALL follow the platform default (dereference on both macOS and Linux)

#### Scenario: Check existing file

- **WHEN** `__io_file_exists` and `__io_is_file` are called with a path to an existing regular file
- **THEN** `__io_file_exists` returns true AND `__io_is_file` returns true AND `__io_is_directory` returns false

#### Scenario: Check existing directory

- **WHEN** `__io_file_exists` and `__io_is_directory` are called with a path to an existing directory
- **THEN** `__io_file_exists` returns true AND `__io_is_directory` returns true AND `__io_is_file` returns false

#### Scenario: Check non-existent path

- **WHEN** `__io_file_exists` is called with a path that does not exist
- **THEN** the function returns false

---

### REQ-IO-04: File System Operations

The runtime SHALL provide functions for deleting files and creating directories.

- `__io_file_delete(path: *const i8)` SHALL delete the specified file
- `__io_file_delete_async(path: *const i8) -> *mut i8` SHALL return a Future handle
- `__io_mkdir(path: *const i8, recursive: bool)` SHALL create a directory, creating parent directories when recursive is true
- `__io_mkdir_async(path: *const i8, recursive: bool) -> *mut i8` SHALL return a Future handle
- `__io_file_delete` SHALL throw IOError if the path does not exist or is a non-empty directory
- `__io_mkdir` SHALL throw IOError if the path already exists (even when recursive=true) or if parent creation fails

#### Scenario: Delete existing file

- **WHEN** `__io_file_delete` is called with a path to an existing regular file
- **THEN** the file is removed from the filesystem and the function returns normally

#### Scenario: Create nested directories

- **WHEN** `__io_mkdir` is called with a path like "/tmp/a/b/c" and recursive=true, where "/tmp/a" does not exist
- **THEN** directories "/tmp/a", "/tmp/a/b", and "/tmp/a/b/c" are all created

---

### REQ-IO-05: Console Input

The runtime SHALL provide a function for reading a single line from standard input.

- `__io_read_line() -> *mut i8` SHALL read a line from stdin, blocking until a newline or EOF
- The returned string SHALL NOT include the trailing newline character
- The function SHALL return a null pointer when EOF is reached with no data

#### Scenario: Read a line from stdin

- **WHEN** `__io_read_line` is called while stdin has "hello world\n" buffered
- **THEN** the function returns a heap-allocated string "hello world" (without the newline)

#### Scenario: Read at EOF

- **WHEN** `__io_read_line` is called while stdin is at EOF
- **THEN** the function returns a null pointer
