# Security Report

This document describes security risks and mitigations.

## Discovered Vulnerabilities

### 1. Path Traversal Attack (CWE-22) - 🔴 Critical

**Risk Level**: Critical

**Description**: The `unpack()` function uses filenames from TAR archives without validation, allowing paths containing `../` to write files outside the intended directory.

**Impact**:
- Overwriting system files
- Writing files to arbitrary locations
- Potential privilege escalation

**Proof of Concept**:
```rust
// Creating a malicious TAR archive
let header = TarHeader::new("../../../etc/passwd".to_string(), 0o644, data.len());
```

**Mitigation**:
```rust
// Sanitize paths within the unpack function
fn sanitize_path(path: &str) -> Option<PathBuf> {
    let path = Path::new(path);
    
    // Reject absolute paths
    if path.is_absolute() {
        return None;
    }
    
    // Normalize path components and detect '..'
    let mut safe_path = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::Normal(name) => safe_path.push(name),
            std::path::Component::ParentDir => return None, // Reject '..'
            _ => return None,
        }
    }
    
    Some(safe_path)
}
```

**Tests**: `security_test_unpack_path_traversal`, `security_test_unpack_absolute_path`

---

### 2. Symbolic Link Attack (CWE-59) - 🟡 Medium

**Risk Level**: Medium

**Description**: When symbolic links in TAR archives are processed, they may enable access to the filesystem outside the archive.

**Current Mitigation**: The `read_tar()` function filters out non-regular files, but how the `pack()` function handles symbolic links is unclear.

**Recommended Mitigation**:
- Determine an explicit policy for handling symbolic links
- Don't follow symbolic links, or limit to links valid only within the archive

**Tests**: `security_test_symlink_in_archive`, `security_test_pack_symlink_handling`

---

### 3. Integer Overflow (CWE-190) - 🟡 Medium

**Risk Level**: Medium

**Description**: If the TAR header's `size` field contains a huge value like `u64::MAX`, it may cause issues in memory allocation or buffer calculations.

**Impact**:
- DoS attack (memory exhaustion)
- Buffer overflow
- Panics or crashes

**Current Mitigation**: The `read_tar()` function is partially protected as it doesn't attempt to read beyond available data size.

**Recommended Mitigation**:
```rust
const MAX_FILE_SIZE: u64 = 1024 * 1024 * 1024; // 1GB limit

if header.size > MAX_FILE_SIZE {
    eprintln!("File size too large: {}", header.size);
    continue;
}
```

**Tests**: `security_test_integer_overflow`, `security_test_size_mismatch`

---

### 4. Zip Slip Vulnerability (CWE-23) - 🔴 Critical

**Risk Level**: Critical

**Description**: A variant of path traversal attack that allows writing files to arbitrary locations in the filesystem using absolute or relative paths when extracting compressed archives (.tar.gz).

**Impact**: Same as path traversal attack

**Recommended Mitigation**: Same as path traversal attack mitigation

**Tests**: `security_test_unpack_path_traversal`, `security_test_unpack_absolute_path`

---

### 5. File Overwrite (CWE-73) - 🟢 Low

**Risk Level**: Low

**Description**: The `unpack()` function overwrites existing files without warning.

**Impact**:
- Data loss
- Unintended file replacement

**Recommended Mitigation**:
- Option to prompt for confirmation before overwriting
- Overwrite prevention mode
- Backup creation option

**Tests**: `security_test_unpack_overwrites_existing`

---

### 6. Special Character Injection (CWE-75) - 🟡 Medium

**Risk Level**: Medium

**Description**: If filenames contain NULL bytes, newlines, or special characters, they may have unexpected effects on filesystem operations or logs.

**Impact**:
- File creation errors
- Log injection
- Path parsing confusion

**Current Mitigation**: The `read_tar_str()` function terminates strings at NULL bytes.

**Recommended Mitigation**:
```rust
fn is_safe_filename(name: &str) -> bool {
    !name.contains('\0') && 
    !name.contains('\n') && 
    !name.contains('\r') &&
    !name.is_empty()
}
```

**Tests**: `security_test_null_byte_injection`, `security_test_special_characters`

---

### 7. Device File Attack (CWE-367) - 🟢 Low

**Risk Level**: Low

**Description**: If a TAR archive contains device files, FIFOs, or directory entries, they could be exploited for privilege escalation or DoS attacks.

**Current Mitigation**: The `read_tar()` function only processes regular files (typeflag '0' or 0) and filters out other types.

**Tests**: `security_test_device_file_in_archive`

---

### 8. Checksum Bypass (CWE-354) - 🟢 Low

**Risk Level**: Low

**Description**: Checksum verification is not performed automatically, making it impossible to detect corrupted or tampered archives.

**Impact**:
- Lack of data integrity
- Missing unintended data corruption

**Recommended Mitigation**:
- Enable checksum verification by default
- Explicit error handling on verification failure

**Tests**: `security_test_invalid_checksum`

---

### 9. Field Overflow Protection - ✅ Mitigated

**Description**: Excessively long field values (name, prefix, username, etc.) are properly trimmed.

**Mitigation**: The `create_tar_header()` function enforces maximum length for each field.

**Tests**: `security_test_oversized_name`, `security_test_oversized_prefix`, `security_test_all_fields_oversized`

---

### 10. Deep Nested Paths (CWE-400) - 🟢 Low

**Risk Level**: Low

**Description**: Extremely deep directory structures may cause resource exhaustion.

**Impact**:
- Disk space waste
- inode exhaustion
- Path length limit issues

**Recommended Mitigation**:
```rust
const MAX_PATH_DEPTH: usize = 100;

if path.components().count() > MAX_PATH_DEPTH {
    eprintln!("Path too deep: {}", path);
    continue;
}
```

**Tests**: `security_test_deeply_nested_path`

---

## Recommended Mitigation Priority

### High Priority (Immediate Action Recommended)

1. **Implement path traversal protection** - Add path sanitization to `unpack()` function
2. **Maximum file size limit** - To prevent DoS attacks

### Medium Priority

3. **Clarify symbolic link policy** - Documentation and implementation
4. **Special character filtering** - Strengthen filename validation
5. **Enable checksum verification by default** - Ensure data integrity

### Low Priority

6. **Overwrite protection option** - Improve usability
7. **Path depth limit** - Resource protection

---

## Security Best Practices

### Recommendations for Usage

1. **Handle TAR archives from untrusted sources with caution**
2. **Check archive contents with `list()` before extraction**
3. **Extract to dedicated isolated directories**
4. **Verify file permissions after extraction**

### Recommendations for Implementation

1. **Validate all user input (filenames)**
2. **Implement proper error handling**
3. **Set resource limits**
4. **Regularly check for security updates**

---

## Test Coverage

The following security tests are implemented:

### tar.rs Module
- `security_test_path_traversal_attack` - Path traversal detection
- `security_test_size_mismatch` - Size mismatch handling
- `security_test_integer_overflow` - Integer overflow handling
- `security_test_null_byte_injection` - NULL byte injection
- `security_test_invalid_checksum` - Invalid checksum handling
- `security_test_symlink_in_archive` - Symbolic link filtering
- `security_test_device_file_in_archive` - Device file filtering
- `security_test_deeply_nested_path` - Deep path handling
- `security_test_malformed_archive_early_termination` - Corrupted archive handling
- `security_test_oversized_name` - Oversized name field
- `security_test_oversized_prefix` - Oversized prefix field
- `security_test_all_fields_oversized` - All fields oversized

### lib.rs Module
- `security_test_unpack_path_traversal` - Path traversal in unpack
- `security_test_unpack_absolute_path` - Absolute path handling
- `security_test_unpack_large_file_size` - Large file size
- `security_test_unpack_empty_filename` - Empty filename
- `security_test_unpack_special_characters` - Special character handling
- `security_test_pack_symlink_handling` - Symbolic link handling in pack
- `security_test_unpack_overwrites_existing` - File overwrite behavior

---

## Report Date

November 28, 2025

## Last Updated

November 28, 2025
