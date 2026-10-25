## What this changes

<!-- One or two sentences. What is different after this merges? -->

## Why

<!-- The problem it solves. Link the issue: Closes #<n> -->

## Checklist

- [ ] `cargo fmt --all -- --check` leaves no diff
- [ ] `cargo clippy --all-targets -- -D warnings` is clean
- [ ] `cargo test` passes
- [ ] `cargo doc --no-deps` builds, and new public items have doc comments
- [ ] If this fixes a bug: there is a test that fails without the fix
- [ ] A new calculation cites its KP part and has a test built on a worked example with its source
- [ ] No text or design table is copied out of the KP documents
- [ ] README updated if this changes what the crate covers

### AI use

<!-- Required for outside contributors; a check fails without it. See CONTRIBUTING.md. -->

- [ ] None
- [ ] Some (tool or agent, and model: ...)
- [ ] Most (tool or agent, and model: ...)
- [ ] I wrote this description myself and I have read and tested every line.
