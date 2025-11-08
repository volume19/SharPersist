# Security Documentation

## Authorized Use Only

**WARNING:** This tool creates persistence mechanisms that can be used for malicious purposes if misused.

### Legitimate Use Cases

✅ **ALLOWED:**
- Authorized penetration testing engagements
- Red team exercises with proper authorization
- Security research in controlled environments
- Defensive security training and education
- Security control testing with organizational approval

❌ **PROHIBITED:**
- Unauthorized access to computer systems
- Malware development or distribution
- Any illegal activity
- Use without explicit written authorization

## Security Design Principles

### 1. No Stealth or Evasion Capabilities

This port **explicitly refuses** to implement:
- Anti-forensic techniques (log clearing, artifact wiping)
- Evasion mechanisms (sandbox detection, AV evasion)
- Detection bypasses (API unhooking, direct syscalls)
- Stealth features (hidden registry keys, rootkit behaviors)

### 2. Audit Logging

All operations are logged for accountability:

```bash
# Enable logging
export RUST_LOG=info

# Run with audit trail
sharpersist -t reg -m add -c cmd.exe -k hkcurun -v Test 2>&1 | tee audit.log
```

Log output includes:
- Technique and method used
- Full command with arguments
- Registry keys/service names/task names
- Success/failure status
- Timestamps (via log framework)

### 3. Input Validation

All user inputs are validated before use:

- Technique names checked against allowed list
- Method names validated
- File paths checked for existence/permissions
- Registry keys validated before access
- No command injection vulnerabilities (direct API usage)

### 4. Privilege Checks

Operations requiring elevated privileges check and inform the user:

```rust
if !is_user_admin() {
    return Err(PersistError::PermissionDenied(
        "Administrative privileges required for service operations".to_string()
    ));
}
```

### 5. Error Handling

No errors are silently ignored:

- All Windows API calls checked for errors
- Clear error messages for permission denied
- Distinct errors for not-found vs. permission issues
- No panic on user input errors

## Unsafe Code Audit

### Unsafe Blocks Location

All `unsafe` code is isolated in FFI modules:

1. **`src/ffi/windows_registry.rs`** (6 unsafe blocks)
   - `RegOpenKeyExW` - Registry key opening (line 88)
   - `RegCreateKeyExW` - Registry key creation (line 129)
   - `RegSetValueExW` - Registry value writing (line 173)
   - `RegQueryValueExW` - Registry value reading (lines 207, 238)
   - `RegDeleteValueW` - Registry value deletion (line 268)
   - `RegCloseKey` - Handle cleanup in Drop impl (line 306)

2. **`src/lib/utils.rs`** (1 unsafe block)
   - `IsUserAnAdmin` - Privilege check (line 38)

### Safety Invariants

Each `unsafe` block has a SAFETY comment documenting:
- Why unsafe is necessary
- What invariants are maintained
- How parameters are validated

Example:
```rust
// SAFETY: RegOpenKeyExW is called with valid parameters:
// - hkey is a valid predefined key constant
// - subkey is a valid null-terminated wide string
// - result is written to &mut key with proper lifetime
```

### RAII Pattern

All Windows handles use RAII for automatic cleanup:

```rust
pub struct RegKey(HKEY);

impl Drop for RegKey {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe { let _ = RegCloseKey(self.0); }
        }
    }
}
```

## Dependency Security

### Dependency Vetting

All dependencies are vetted for:
- License compliance (MIT, Apache-2.0)
- Maintenance status (active updates)
- Download count (>1M preferred)
- Known vulnerabilities (cargo-audit)

### Dependency List

See `Cargo.toml` for full list. Key dependencies:

- `windows` (0.51): Official Microsoft Rust bindings
- `clap` (4.4): CLI parsing
- `sha2` (0.10): Cryptographic hashing (RustCrypto)
- `thiserror` (1.0): Error handling
- `log` (0.4): Logging facade
- `env_logger` (0.11): Logger implementation
- `hex` (0.4): Hex encoding

### Security Scanning

```bash
# Scan for known vulnerabilities
cargo audit

# Check dependency licenses
cargo deny check licenses
```

## Minimal Privilege Recommendations

1. **Run as Standard User** when possible (HKCU registry, user tasks)
2. **Elevate Only When Required** (HKLM registry, services, system tasks)
3. **Use Separate Test Environment** - Never test on production systems
4. **No Embedded Credentials** - Commands should not contain hardcoded passwords
5. **Clean Up After Testing** - Always use remove operations to clean test artifacts

## Responsible Disclosure

Security vulnerabilities in this tool should be reported responsibly. Please contact the repository maintainers through GitHub issues or security advisories.

## Compliance

This tool is provided for authorized security testing only. Users are responsible for:
- Obtaining proper authorization before use
- Complying with all applicable laws and regulations
- Using the tool ethically and responsibly
- Not using the tool for malicious purposes

**Unauthorized use of this tool may be illegal and is strictly prohibited.**
