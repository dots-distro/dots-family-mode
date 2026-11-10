//! The single daemon-policy decision path.
//!
//! Both the terminal wrapper and `--check-only` ask the daemon the same
//! question and reduce its answer with this function, so the two cannot
//! disagree about whether a command is permitted.

use tracing::warn;

/// Turn the daemon's profile answer into a terminal verdict.
///
/// The daemon is the policy authority, so not getting an answer is not
/// permission. `Ok(true)` allows the command and `Ok(false)` blocks it. `None`
/// means no proxy was established (the daemon is unreachable) and `Some(Err)`
/// means the query failed; both deny, with a reason naming the cause, instead
/// of silently falling back to allow.
pub(crate) fn profile_verdict(
    command: &str,
    answer: Option<zbus::Result<bool>>,
) -> anyhow::Result<bool> {
    match answer {
        Some(Ok(true)) => Ok(true),
        Some(Ok(false)) => {
            warn!("Command '{}' blocked by active profile", command);
            Ok(false)
        }
        Some(Err(e)) => {
            warn!("Failed to check profile restrictions for '{}': {}", command, e);
            anyhow::bail!(
                "Policy could not be checked for command '{}': daemon unreachable ({})",
                command,
                e
            )
        }
        None => {
            warn!("Command '{}' cannot be checked: no daemon proxy", command);
            anyhow::bail!("Policy could not be checked for command '{}': no daemon proxy", command)
        }
    }
}
