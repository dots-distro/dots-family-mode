# Test inventory — `review-fixes`, 2026-11-08

Static read of the tree. Nothing was built or run. This counts test *definitions* and
states what each asserts; it makes no claim that any test passes. The repository has no
green CI record: the README's earlier "222 unit tests (100% pass rate) / Production Ready"
line is not supported by this tree (256 tests, 20 `#[ignore]`, red CI history).

## Counts by crate

`#[test]` + `#[tokio::test]` under `src/` and `tests/`. Ignored = `#[ignore]`, not executed by
`cargo test`.

| Crate | src | tests/ | Total | Ignored | Kind | What it actually asserts |
|---|---|---|---|---|---|---|
| dots-family-common | 30 | 0 | 30 | 0 | unit (pure) | config defaults; error `Display`/`From`; Argon2 hash+verify; 64-char token; PBKDF2 key derivation; in-memory `RateLimiter`; type serde round-trip; `is_in_window`, `find_next_window`, holiday override, empty windows, warning window |
| dots-family-proto | 9 | 0 | 9 | 0 | unit (pure) | serde round-trip of each `Event` variant and `Activity` |
| dots-family-db | 22 | 2 | 24 | 0 | integration vs temp SQLite | create/open/close DB, dir creation; migrations apply/rollback detection/status; profile CRUD incl. duplicate; daily/weekly summary arithmetic; eBPF metric inserts and reads |
| dots-family-daemon | 52 | 72 | 124 | 20 | unit + temp-SQLite integration | policy engine limits/allowlist/blocklist; heartbeat tamper set/clear; password→token auth; session token validate/revoke; time-window add/remove/overlap/format validation (14); report JSON/CSV export (8); D-Bus calls that silently skip when no daemon is on the bus (3+7); 4 tests shell out to `cargo build` |
| dots-family-monitor | 4 | 5 | 9 | 0 | unit + smoke | focus-change activity dedup; D-Bus client "graceful degradation" (`assert!(true)` when daemon absent); `Activity` serde |
| dots-family-filter | 14 | 1 | 15 | 0 | unit + TLS handshake | domain/URL/category rule matching, safe-search, allow override, block-page text; self-signed CA → per-host cert; one loopback TLS handshake with generated cert |
| dots-family-ctl | 1 | 0 | 1 | 0 | unit | `AuthHelper` construction only |
| dots-family-gui | 0 | 0 | 0 | 0 | — | nothing |
| dots-terminal-filter | 33 | 0 | 33 | 0 | unit | shell word/pipe/redirect parsing; risk heuristics; educational message text; script inspection + cache; age-group config |
| dots-wm-bridge | 7 | 4 | 11 | 0 | unit + simulation | adapter construction/availability; one "multi-WM switching" simulation; `CompositorType` display |
| **Workspace total** | **172** | **84** | **256** | **20** | | **236 would execute if it compiled** |

Not in the workspace: `crates/dots-family-ebpf/` (**0** tests) is commented out at
`Cargo.toml:14`, so `cargo test --workspace` never builds it. `tests/mock_daemon.rs` (**2**
tests) sits outside every package and is not a test target, so it never runs either.

## BDD

| Feature | Scenarios | Drives | Needs root/kernel/VM? |
|---|---|---|---|
| `tests/bdd/tests/features/time_windows.feature` | 20 | `dots_family_common::TimeWindowEnforcer` against an in-memory `TimeWindowWorld` | No |
| `tests/bdd/tests/features/smoke_test.feature` | 1 | framework placeholder (`framework_init` does nothing) | No |

21 scenarios, 71 step definitions. They do **not** drive the daemon: "session locked",
"processes suspended", override, audit-log and notification steps assert or mutate world
fields the steps themselves set (`steps.rs:577-622`, `369-451`). Root `features/time_windows.feature`
is a duplicate that the runner (`tests/bdd/tests/time_windows.rs` → `tests/features`) never loads.
No VM/root/kernel dependency; the extensive `tests/nix/*`, `tests/*.sh` VM scripts are not
wired into CI.

## Untested critical paths

| Product claim / path | Status | Evidence |
|---|---|---|
| Encrypted database at rest | **not covered** | only `test_database_initialization_with_encryption` (`database_integration_test.rs:28`) and it is `#[ignore]`; `libsqlite3-sys` here has no SQLCipher, so the `pragma key` in `connection.rs` cannot encrypt |
| eBPF monitor load | **not covered** | no test calls `Bpf::load`; `monitoring_service.rs` has 0 tests; the `dots-family-ebpf` crate is excluded and shipped only as `prebuilt-ebpf/` blobs |
| HTTPS interception/block decision | **partial** | `integration_mitm_accepts_tls_with_generated_cert` proves a handshake; `proxy.rs` has no tests for the allow/block decision, SNI or CONNECT handling |
| Daemon restart / state recovery | **not covered** | `test_bdd_given_profile_when_set_active_then_loaded_on_next_startup` is `#[ignore]` (`profile_manager.rs:1901`); `edge_case_handler::attempt_daemon_recovery` has no test |
| Tamper detection | **covered (in-memory)** | `test_bdd_given_monitor_when_heartbeat_timeout_then_tamper_detected`, `..._reconnects_then_tamper_cleared` |
| Time-window expiry at boundary | **partial** | `test_is_in_window` asserts 08:00 is denied (end-exclusive) and `test_warning_before_window_end` covers 18:55; no test drives `should_lock` → `lock_session`, and `test_enforcement_task_creation` only constructs the task |
| Concurrent profile switching | **not covered** | no test spawns concurrent writers; every DB test is single-threaded |
| Database corruption | **not covered** | no test; `grep -i corrupt` over crates and tests returns nothing |
| Parent-password brute force | **not covered end-to-end** | `test_rate_limiting` exercises the isolated `RateLimiter`; `authenticate_parent` (`profile_manager.rs:457`) never consults it, so no lockout test exists |

## What CI actually runs

`nix develop -c cargo test --workspace` (`.github/workflows/ci.yml`) reaches the 10 workspace
members above plus the `dots-family-bdd-tests` cucumber binary: 236 non-ignored tests and 21
scenarios, **if it compiles**. It skips `crates/dots-family-ebpf` (excluded), `tests/mock_daemon.rs`
(no package), every `tests/nix/*` VM script, and all 20 `#[ignore]` tests. The 4 daemon tests in
`integration_test.rs` spawn a nested `cargo build`, which contends for the cargo lock of the
outer test run; the 10 D-Bus tests print `SKIPPED` and still pass when no daemon is present.
Treat all of this as "would execute", not "passes".
