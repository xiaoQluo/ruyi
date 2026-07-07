# Problems: Implement map builtins

## Unresolved issues
1. **Email notification blocked**: Resend sandbox domain `onboarding@resend.dev` can only send to the account owner's email (`xiaoq.luo@foxmail.com`), not the required recipient `feather.lzg@foxmail.com`. No verified domain is available in this environment.
2. **Workspace test suite broken**: Pre-existing missing functions (`ruyi_array_create`, `ruyi_array_slice`) in `builtins.rs` tests prevent the full test suite from compiling. This should be fixed separately.
3. **LLVM dependency prevents full workspace check**: The environment lacks LLVM 14, so `cargo check --workspace` and `cargo test --workspace` cannot be run. This is an environment setup issue.

## Technical debt
- The map implementation does not implement a custom destructor for GC tracing. The GC uses a dummy `TypeInfo`, so map entries may not be properly traced during collection. This is acceptable for the baseline but should be revisited when the GC gains precise type information.
