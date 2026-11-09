//! Peer-credential checks for mutating D-Bus methods.
//!
//! The D-Bus policy in `nixos-modules/dots-family/dbus.nix` is the primary
//! gate: it decides which callers may send a message at all. This module is
//! the second gate. If that policy is wrong (for example, it grants a group
//! that does not exist), the daemon still refuses a mutating call from a
//! caller that is neither root nor one of the configured parent users.
//!
//! The caller's identity comes from the bus, not from the message body.
//! `org.freedesktop.DBus.GetConnectionUnixUser` returns the UID that the bus
//! daemon assigned to the sender's connection, and zbus exposes it as
//! [`zbus::fdo::DBusProxy::get_connection_unix_user`]. The sender's unique
//! bus name is in the message header, so the existing connection is enough
//! and no token scheme is needed.

use std::collections::BTreeSet;

use tracing::warn;
use zbus::{
    fdo::{DBusProxy, Error as FdoError},
    message::Header,
    names::BusName,
    Connection,
};

/// UID of the root account. Root always passes the guard.
pub const ROOT_UID: u32 = 0;

/// The callers that may invoke mutating D-Bus methods.
///
/// Membership is by UID, resolved once at daemon start from the configured
/// parent user names. Root is always allowed. A caller that is not root and
/// not in this set is refused, even if the bus policy let the message through.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CallerPolicy {
    parent_uids: BTreeSet<u32>,
}

impl CallerPolicy {
    /// Resolve the configured parent user names to UIDs.
    ///
    /// A name that does not resolve is logged and skipped; it grants nothing.
    /// If no name resolves the policy allows only root, which fails closed.
    pub fn from_user_names(parent_users: &[String]) -> Self {
        let mut parent_uids = BTreeSet::new();

        for name in parent_users {
            match nix::unistd::User::from_name(name) {
                Ok(Some(user)) => {
                    parent_uids.insert(user.uid.as_raw());
                }
                Ok(None) => {
                    warn!(
                        parent = %name,
                        "Configured parent user does not exist; it will not be accepted as a parent"
                    );
                }
                Err(error) => {
                    warn!(
                        parent = %name,
                        %error,
                        "Failed to resolve configured parent user; it will not be accepted as a parent"
                    );
                }
            }
        }

        if parent_uids.is_empty() {
            warn!("No parent users resolved: only root may call mutating D-Bus methods");
        }

        Self { parent_uids }
    }

    /// Build a policy directly from UIDs. Used by tests and by callers that
    /// already resolved names.
    pub fn from_uids(uids: impl IntoIterator<Item = u32>) -> Self {
        Self { parent_uids: uids.into_iter().collect() }
    }

    /// Whether `caller_uid` may call a mutating method.
    ///
    /// This is the seam the unit tests exercise: it takes the caller's UID
    /// and needs no bus connection.
    pub fn permits_mutation(&self, caller_uid: u32) -> bool {
        caller_uid == ROOT_UID || self.parent_uids.contains(&caller_uid)
    }
}

/// Ask the bus for the UID of the connection that sent `header`.
///
/// Returns `None` when the header has no sender or the bus cannot answer.
/// Callers must treat `None` as a refusal (fail closed).
pub async fn caller_uid(connection: &Connection, header: &Header<'_>) -> Option<u32> {
    let sender = header.sender()?.clone();
    let proxy = DBusProxy::new(connection).await.ok()?;
    proxy.get_connection_unix_user(BusName::from(sender)).await.ok()
}

/// Refuse the call unless the sender is root or a configured parent.
///
/// The refusal is returned as `org.freedesktop.DBus.Error.AccessDenied`, so
/// the client sees an ordinary D-Bus error rather than a made-up string.
pub async fn require_parent_caller(
    policy: &CallerPolicy,
    connection: &Connection,
    header: &Header<'_>,
) -> Result<(), FdoError> {
    let Some(uid) = caller_uid(connection, header).await else {
        warn!("Refusing mutating D-Bus call: the caller's UID is unavailable");
        return Err(FdoError::AccessDenied("caller UID unavailable".to_string()));
    };

    if policy.permits_mutation(uid) {
        return Ok(());
    }

    warn!(caller_uid = uid, "Refusing mutating D-Bus call from a non-parent caller");
    Err(FdoError::AccessDenied(format!("uid {uid} is not root or a configured parent")))
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHILD_UID: u32 = 1000;
    const PARENT_UID: u32 = 1001;

    #[test]
    fn a_child_uid_cannot_create_a_profile() {
        let policy = CallerPolicy::from_uids([PARENT_UID]);

        assert!(
            !policy.permits_mutation(CHILD_UID),
            "a child UID must not be allowed to call create_profile"
        );
    }

    #[test]
    fn root_can_create_a_profile() {
        // Even with no configured parents, root is always allowed.
        let policy = CallerPolicy::from_uids([]);

        assert!(policy.permits_mutation(ROOT_UID), "root must be allowed to call create_profile");
    }

    #[test]
    fn a_configured_parent_uid_can_create_a_profile() {
        let policy = CallerPolicy::from_uids([PARENT_UID]);

        assert!(
            policy.permits_mutation(PARENT_UID),
            "a configured parent UID must be allowed to call create_profile"
        );
    }

    #[test]
    fn an_empty_policy_allows_only_root() {
        let policy = CallerPolicy::default();

        assert!(policy.permits_mutation(ROOT_UID));
        assert!(!policy.permits_mutation(CHILD_UID));
        assert!(!policy.permits_mutation(PARENT_UID));
    }

    #[test]
    fn an_unknown_parent_name_grants_nothing() {
        let policy =
            CallerPolicy::from_user_names(&["dots-family-no-such-user-for-test".to_string()]);

        assert_eq!(policy, CallerPolicy::from_uids([]));
    }
}
