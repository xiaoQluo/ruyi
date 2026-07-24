# Spec: Path FFI Backend

## ADDED Requirements

### REQ-PATH-01: Path Joining and Normalization

The runtime SHALL provide C ABI functions for constructing and cleaning path strings.

- `__path_join(segments: *mut i8) -> *mut i8` SHALL join an array of path segment strings using the platform separator
- `__path_normalize(path: *const i8) -> *mut i8` SHALL resolve `.` and `..` components and remove redundant separators
- `__path_join` SHALL accept a varargs-compatible array of null-terminated strings as its single argument
- `__path_normalize` SHALL handle absolute and relative paths correctly on both macOS and Linux
- Return values SHALL be heap-allocated null-terminated strings

#### Scenario: Join multiple segments

- **WHEN** `__path_join` is called with segments ["/home", "user", "docs", "file.txt"]
- **THEN** the function returns "/home/user/docs/file.txt" on Unix

#### Scenario: Join with empty segment

- **WHEN** `__path_join` is called with segments ["/home", "", "file.txt"]
- **THEN** the function returns "/home/file.txt" (empty segments are ignored)

#### Scenario: Normalize path with `..`

- **WHEN** `__path_normalize` is called with "/home/user/../docs/./file.txt"
- **THEN** the function returns "/home/docs/file.txt"

#### Scenario: Normalize relative path

- **WHEN** `__path_normalize` is called with "./a/b/../c"
- **THEN** the function returns "a/c"

---

### REQ-PATH-02: Path Decomposition

The runtime SHALL provide functions for extracting components from a path string.

- `__path_basename(path: *const i8) -> *mut i8` SHALL return the last component (filename)
- `__path_dirname(path: *const i8) -> *mut i8` SHALL return the directory portion (everything before the last separator)
- `__path_extname(path: *const i8) -> *mut i8` SHALL return the file extension including the dot, or empty string if none
- Return values SHALL be heap-allocated null-terminated strings

#### Scenario: Extract basename from absolute path

- **WHEN** `__path_basename` is called with "/home/user/file.txt"
- **THEN** the function returns "file.txt"

#### Scenario: Extract dirname from absolute path

- **WHEN** `__path_dirname` is called with "/home/user/file.txt"
- **THEN** the function returns "/home/user"

#### Scenario: Extract extension

- **WHEN** `__path_extname` is called with "/home/user/file.tar.gz"
- **THEN** the function returns ".gz" (last dot only, consistent with stdlib expectation)

#### Scenario: No extension

- **WHEN** `__path_extname` is called with "/home/user/Makefile"
- **THEN** the function returns "" (empty string)

---

### REQ-PATH-03: Path Classification

The runtime SHALL provide a function for testing whether a path is absolute and a function for the platform separator character.

- `__path_is_absolute(path: *const i8) -> bool` SHALL return true if the path starts with `/` on Unix
- `__path_separator() -> *mut i8` SHALL return "/" on macOS and Linux
- `__path_is_absolute` SHALL return false for paths like "relative/path" or "./file"

#### Scenario: Absolute Unix path

- **WHEN** `__path_is_absolute` is called with "/usr/local/bin"
- **THEN** the function returns true

#### Scenario: Relative path

- **WHEN** `__path_is_absolute` is called with "src/main.rs"
- **THEN** the function returns false

#### Scenario: Path separator

- **WHEN** `__path_separator` is called on macOS or Linux
- **THEN** the function returns "/"

---

### REQ-PATH-04: Path Manipulation

The runtime SHALL provide a function for computing relative paths between two locations.

- `__path_relative(from: *const i8, to: *const i8) -> *mut i8` SHALL compute a relative path from `from` to `to`
- Both inputs SHALL be treated as directory paths (trailing content is treated as the final component)
- The result SHALL use `..` components when ascending from `from` to reach a common ancestor
- The function SHALL throw an error for paths on different roots (e.g. different mount points)

#### Scenario: Relative path to sibling

- **WHEN** `__path_relative` is called with from="/home/user/docs" and to="/home/user/photos/vacation.jpg"
- **THEN** the function returns "../photos/vacation.jpg"

#### Scenario: Relative path to child

- **WHEN** `__path_relative` is called with from="/home/user" and to="/home/user/docs/file.txt"
- **THEN** the function returns "docs/file.txt"

#### Scenario: Same path

- **WHEN** `__path_relative` is called with from="/home/user" and to="/home/user"
- **THEN** the function returns "" (empty string)
