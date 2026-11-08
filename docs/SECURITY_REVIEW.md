# Security review — HTTPS interception and tamper/fail-open

Scope: `review-fixes` working tree at `cf1ef07` plus uncommitted changes. Read-only audit
of the two paths below; no build or test was run, so nothing here is a test result.
Line references are the working tree at the time of review.

## A. HTTPS interception

| # | Question | Verdict | Evidence |
|---|----------|---------|----------|
| A1 | Where is the CA key generated/stored, owner and mode? | weak | `ssl-intercept.nix:30-34` writes `/var/lib/dots-family/ssl/ca.key`, 4096-bit, `chmod 600`, dir `0700` (`:29-30`, tmpfiles `:219-221`), service runs as root so owner root. But `security.nix:151` `Z /var/lib/dots-family 0750 ...` re-applies mode recursively on every boot, making `ca.key` group-readable to `dots-family-parents`. The duplicate generator `default-with-ssl.nix:340-347` sets no mode at all. |
| A2 | Is the key written to the repo, a world-readable path, or the DB? | weak | Not in the repo and no DB path references a key. But `ssl-intercept.nix:45-49` exports `ca.p12` protected only by `pkcs12Password`, whose default is `dots-family` (`:210-214`) and is interpolated into a `/nix/store` script readable by every user. `default-with-ssl.nix:347` concatenates cert+key into `ca.pem`. No key is logged. |
| A3 | Which trust stores get the CA, and can an operator remove it? | broken | Copies to `/etc/ssl/certs` (`ssl-intercept.nix:135-139`), `/etc/pki/java` (`:143-146`), `trust anchor`/`/usr/local/share` (`:149-155`). On NixOS `/etc/ssl/certs` is a store symlink and the script runs under `set -euo pipefail` (`:126`), so the oneshot fails or writes into the store. `security.pki.certificateFiles` is explicitly not used (`:254-257`). No uninstall/`ExecStop`/removal path exists, and Firefox NSS is left to manual steps (`:56-65`). |
| A4 | Bypass or proxy failure: fail open or closed? | broken | There is no redirect at all: `security.nix:222-231` is comment-only ("placeholder"), and no nftables/NAT/proxy setting exists in the modules. The filter binary is neither packaged (`flake.nix:199-203`) nor started by any unit. When it does reach the daemon it fails open: connection failure ⇒ "standalone mode" (`filter_engine.rs:35-49`), daemon error ⇒ `warn!` then continue (`:71-77`). |
| A5 | Pinning, non-system-store apps, QUIC/HTTP3? | not implemented | No match for pinning, HTTP/3, Alt-Svc or QUIC in `dots-family-filter`, the daemon or the modules. UDP/443 is never intercepted, and the placeholders at `enforcement.rs:154-162` only build a TCP `iptables` rule. |

## B. Tamper and fail-open

| # | Question | Verdict | Evidence |
|---|----------|---------|----------|
| B1 | Daemon death ⇒ silent loss of enforcement? | broken | `daemon.nix:121-124`: `Restart=on-failure`, `RestartSec=5`, `StartLimitBurst=3`/300s — after three failures systemd gives up, and a clean exit (normal shutdown, `daemon.rs:298-327`) is never restarted. No `WatchdogSec`/`sd_notify` anywhere. eBPF is monitor-only and its fds die with the process (`ebpf/mod.rs:79-127`); load failure is only logged (`daemon.rs:168-192`, `:227-231`). Tamper heartbeat code is dead: `check_monitor_health`/`is_tampered` have no non-test caller (`profile_manager.rs:586-614`). |
| B2 | Can a child stop the service or reach a mutating D-Bus method? | broken | D-Bus itself denies children: they are in `dots-family-children` (`default.nix:338,347`), not the allowed groups, and the trailing default deny matches all (`dbus.nix:131-135`). But `security.wrappers.dots-family-daemon` grants `cap_bpf,cap_sys_admin` with no `permissions` restriction (`security.nix:9-14`), so the default world-executable wrapper lets **any** user run it with CAP_SYS_ADMIN. The daemon also does no peer-credential check: `create_profile`, `set_active_profile`, `lock_session`, `process_activity_for_policy` take no token (`dbus_impl.rs:299,330,709,573`). Group names are mismatched (`dots-parents` at `dbus.nix:95,187` vs `dots-family-parents`), so admins are denied too. |
| B3 | Does a DB/policy error fail open? | broken | Explicit fail-open branches: no active profile ⇒ allow (`policy_engine.rs:94-101`), network activity ⇒ allow (`:118-124`), no enforcer ⇒ allow (`time_window_manager.rs:60-67`), no profile ⇒ no-op (`time_window_enforcement_task.rs:321-326`), policy error ⇒ `"blocked":false` (`dbus_impl.rs:543-569`, `:586-595`), activity read error ⇒ only `warn!` (`daemon.rs:388` + `:262-267`). The app query itself fails closed on DB error (`dbus_impl.rs:55-60`), but the caller ignores that error and continues (`filter_engine.rs:71-77`). |
| B4 | Is the parent hash readable by non-parents, and is login rate-limited? | weak | Only the Argon2 hash is in TOML now; the DB key moved to `db.key` 0600 (`config.rs:34-81`, `profile_manager.rs:550-560`). But `config.rs:215` writes the TOML with no explicit mode, and `security.nix:155` hardens the wrong path until this review (real path is `/var/lib/dots-family/config/daemon.toml`, `daemon.nix:18`). Login has no rate limit: `authenticate_parent` has no attempt counter (`profile_manager.rs:457-499`), and the existing `RateLimiter` (`common/src/security.rs:143-181`) is never constructed. |

## Ranked findings (child-escape impact, highest first)

1. **World-executable CAP_SYS_ADMIN wrapper** (`security.nix:9-14`) — a child can execute
   `/run/wrappers/bin/dots-family-daemon` and obtain CAP_SYS_ADMIN/CAP_BPF, i.e. root-equivalent.
2. **The filter crate does not build at HEAD, so nothing filters anything** —
   `crates/dots-family-filter/src/proxy.rs` is committed as a single fragment
   starting mid-match (`Ok(mut client_tls) => {`). The last version that parses is
   `0d8a8ff` (2026-01-28, 697 lines); the next commit, `5b354dc` ("fix(proxy):
   Box::pin SslStream client_tls …"), replaced it with one line and every commit
   since carries that. `WebProxy` is therefore undefined, so the crate cannot
   compile. No other crate depends on `dots-family-filter` and the flake does not
   package it (`flake.nix:199-203`), so `nix build` and the NixOS module still
   succeed while shipping no content filter at all. There is also no redirect
   (`security.nix:222-231`), so filtering is bypassable by doing nothing.
3. **Enforcement is fail-open by construction** — no active profile, network events, policy
   errors and daemon unavailability all resolve to "allow" (B3 evidence).
4. **Enforcement dies with the daemon** — no watchdog, restart burst capped at 3, monitor
   tamper detection never invoked (B1 evidence).
5. **CA handling implies protection that is not installed and cannot be removed** — store-path
   install fails, default PKCS#12 password is public in the Nix store, no uninstall unit (A2/A3).
6. **Parent login is unthrottled and the hash file mode depends on umask** (B4 evidence).

## Fix order

1. Remove `capabilities` from the wrapper (or make it `0700`); systemd already grants the same caps.
2. Implement the MITM proxy or stop shipping `sslIntercept`/CA generation until it exists.
3. Make every policy error, missing profile and missing enforcer fail closed for configured children.
4. Use `Restart=always` + `WatchdogSec`/`sd_notify`, alert on start-limit trip, and schedule `check_monitor_health`.
5. Grant `CAP_KILL` (or a privileged helper) so `terminate_process` can act on another uid.
6. Write the TOML 0600, add the parent-login rate limit, and add a CA-removal unit.
