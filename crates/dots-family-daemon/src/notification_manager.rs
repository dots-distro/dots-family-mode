use anyhow::Result;
use dots_family_common::types::{
    Notification, NotificationChannel, NotificationPriority, NotificationType,
};
use notify_rust::{Notification as SystemNotification, Urgency};
use tokio::sync::mpsc;
use tracing::{info, warn};

pub struct NotificationManager {
    sender: mpsc::UnboundedSender<NotificationRequest>,
}

struct NotificationRequest {
    notification: Notification,
}

impl NotificationManager {
    pub fn new() -> Self {
        let (sender, mut receiver) = mpsc::unbounded_channel::<NotificationRequest>();

        // Spawn background task to handle notifications
        tokio::spawn(async move {
            while let Some(request) = receiver.recv().await {
                if let Err(e) = Self::send_notification_internal(request.notification).await {
                    warn!("Failed to send notification: {}", e);
                }
            }
        });

        Self { sender }
    }

    pub async fn send_notification(&self, mut notification: Notification) -> Result<()> {
        notification.mark_sent();

        let request = NotificationRequest { notification };

        self.sender
            .send(request)
            .map_err(|e| anyhow::anyhow!("Failed to queue notification: {}", e))?;

        Ok(())
    }

    async fn send_notification_internal(notification: Notification) -> Result<()> {
        for channel in &notification.channels {
            match channel {
                NotificationChannel::Desktop => {
                    Self::send_desktop_notification(&notification).await?;
                }
                NotificationChannel::InApp => {
                    info!("In-app notification: {}", notification.title);
                }
                NotificationChannel::Email => {
                    warn!("Email notifications not yet implemented");
                }
                _ => {
                    warn!("Unsupported notification channel: {:?}", channel);
                }
            }
        }

        Ok(())
    }

    async fn send_desktop_notification(notification: &Notification) -> Result<()> {
        let urgency = match notification.priority {
            NotificationPriority::Low => Urgency::Low,
            NotificationPriority::Normal => Urgency::Normal,
            NotificationPriority::High => Urgency::Critical,
            NotificationPriority::Urgent => Urgency::Critical,
        };

        let mut desktop_notification = SystemNotification::new();
        desktop_notification
            .summary(&notification.title)
            .body(&notification.message)
            .urgency(urgency)
            .timeout(Self::get_timeout_for_priority(&notification.priority));

        // Set icon based on notification type
        if let Some(icon) = Self::get_icon_for_type(&notification.notification_type) {
            desktop_notification.icon(&icon);
        }

        // Add action buttons for approval requests
        if let NotificationType::ApprovalRequest { .. } = &notification.notification_type {
            desktop_notification
                .action("approve", "Approve")
                .action("deny", "Deny")
                .action("view", "View Details");
        }

        desktop_notification.show()?;
        info!("Desktop notification sent: {}", notification.title);

        Ok(())
    }

    fn get_timeout_for_priority(priority: &NotificationPriority) -> notify_rust::Timeout {
        match priority {
            NotificationPriority::Low => notify_rust::Timeout::Milliseconds(5000),
            NotificationPriority::Normal => notify_rust::Timeout::Milliseconds(8000),
            NotificationPriority::High => notify_rust::Timeout::Milliseconds(15000),
            NotificationPriority::Urgent => notify_rust::Timeout::Never,
        }
    }

    fn get_icon_for_type(notification_type: &NotificationType) -> Option<String> {
        match notification_type {
            NotificationType::ApprovalRequest { .. } => Some("dialog-question".to_string()),
            NotificationType::PolicyViolation { .. } => Some("dialog-warning".to_string()),
            NotificationType::ScreenTimeLimitWarning { .. } => Some("appointment-soon".to_string()),
            NotificationType::TimeWindowEnding { .. } => Some("appointment-soon".to_string()),
            NotificationType::UnusualActivity { .. } => Some("security-medium".to_string()),
            NotificationType::SystemAlert { .. } => Some("dialog-error".to_string()),
            NotificationType::UsageReport { .. } => Some("document-properties".to_string()),
            NotificationType::ExceptionCreated { .. } => Some("dialog-information".to_string()),
            NotificationType::ExceptionEnded { .. } => Some("dialog-information".to_string()),
        }
    }

    /// Create a notification for an approval request
    pub fn create_approval_request_notification(
        request_id: uuid::Uuid,
        child_name: &str,
        request_summary: &str,
    ) -> Notification {
        Notification::new(
            None, // System-wide notification
            NotificationType::ApprovalRequest { request_id },
            format!("Approval Request from {}", child_name),
            format!("{} is requesting: {}", child_name, request_summary),
            NotificationPriority::High,
            vec![NotificationChannel::Desktop, NotificationChannel::InApp],
        )
    }

    /// Create a notification for policy violation
    pub fn create_policy_violation_notification(
        profile_id: uuid::Uuid,
        violation_type: String,
        details: String,
    ) -> Notification {
        Notification::new(
            Some(profile_id),
            NotificationType::PolicyViolation {
                violation_type: violation_type.clone(),
                details: details.clone(),
            },
            "Policy Violation Detected".to_string(),
            format!("{}: {}", violation_type, details),
            NotificationPriority::High,
            vec![NotificationChannel::Desktop],
        )
    }

    /// Create a notification for screen time warning
    pub fn create_screen_time_warning_notification(
        profile_id: uuid::Uuid,
        child_name: &str,
        minutes_remaining: u32,
    ) -> Notification {
        Notification::new(
            Some(profile_id),
            NotificationType::ScreenTimeLimitWarning { minutes_remaining },
            format!("{}'s Screen Time Warning", child_name),
            format!("{} minutes of screen time remaining for {}", minutes_remaining, child_name),
            NotificationPriority::Normal,
            vec![NotificationChannel::Desktop, NotificationChannel::InApp],
        )
    }

    /// Create a notification for time window ending
    pub fn create_time_window_ending_notification(
        profile_id: uuid::Uuid,
        child_name: &str,
        minutes_remaining: u32,
    ) -> Notification {
        Notification::new(
            Some(profile_id),
            NotificationType::TimeWindowEnding { minutes_remaining },
            format!("{}'s Time Window Ending", child_name),
            format!("Allowed time window ends in {} minutes for {}", minutes_remaining, child_name),
            NotificationPriority::Normal,
            vec![NotificationChannel::Desktop, NotificationChannel::InApp],
        )
    }

    /// Create a notification for unusual activity
    pub fn create_unusual_activity_notification(
        profile_id: uuid::Uuid,
        activity_description: String,
    ) -> Notification {
        Notification::new(
            Some(profile_id),
            NotificationType::UnusualActivity {
                activity_description: activity_description.clone(),
            },
            "Unusual Activity Detected".to_string(),
            format!("Detected: {}", activity_description),
            NotificationPriority::High,
            vec![NotificationChannel::Desktop],
        )
    }

    /// Create a system alert notification
    pub fn create_system_alert_notification(
        severity: dots_family_common::types::AlertSeverity,
        message: String,
    ) -> Notification {
        let priority = match severity {
            dots_family_common::types::AlertSeverity::Info => NotificationPriority::Low,
            dots_family_common::types::AlertSeverity::Warning => NotificationPriority::Normal,
            dots_family_common::types::AlertSeverity::Error => NotificationPriority::High,
            dots_family_common::types::AlertSeverity::Critical => NotificationPriority::Urgent,
        };

        Notification::new(
            None,
            NotificationType::SystemAlert { severity, message: message.clone() },
            "DOTS Family System Alert".to_string(),
            message,
            priority,
            vec![NotificationChannel::Desktop],
        )
    }

    /// Create a usage report notification
    pub fn create_usage_report_notification(
        profile_id: uuid::Uuid,
        report_type: String,
        period: String,
    ) -> Notification {
        Notification::new(
            Some(profile_id),
            NotificationType::UsageReport {
                report_type: report_type.clone(),
                period: period.clone(),
            },
            format!("{} Usage Report Available", report_type),
            format!("Your {} usage report for {} is ready to view", report_type, period),
            NotificationPriority::Low,
            vec![NotificationChannel::Desktop, NotificationChannel::InApp],
        )
    }

    /// Create an exception notification
    pub fn create_exception_notification(
        profile_id: uuid::Uuid,
        exception_id: uuid::Uuid,
        exception_type: &str,
        is_created: bool,
    ) -> Notification {
        let (notification_type, title, message) = if is_created {
            (
                NotificationType::ExceptionCreated { exception_id },
                "Exception Created".to_string(),
                format!("New {} exception has been granted", exception_type),
            )
        } else {
            (
                NotificationType::ExceptionEnded { exception_id, reason: "Expired".to_string() },
                "Exception Ended".to_string(),
                format!("{} exception has expired", exception_type),
            )
        };

        Notification::new(
            Some(profile_id),
            notification_type,
            title,
            message,
            NotificationPriority::Normal,
            vec![NotificationChannel::Desktop, NotificationChannel::InApp],
        )
    }
}

impl Default for NotificationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_notification_manager_creation() {
        let manager = NotificationManager::new();

        let notification = NotificationManager::create_system_alert_notification(
            dots_family_common::types::AlertSeverity::Info,
            "Test message".to_string(),
        );

        let result = manager.send_notification(notification).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_notification_creation_helpers() {
        let approval_notification = NotificationManager::create_approval_request_notification(
            uuid::Uuid::new_v4(),
            "Alice",
            "Extra 30 minutes screen time",
        );
        assert_eq!(approval_notification.title, "Approval Request from Alice");

        let violation_notification = NotificationManager::create_policy_violation_notification(
            uuid::Uuid::new_v4(),
            "Blocked Application".to_string(),
            "Attempted to access Discord".to_string(),
        );
        assert_eq!(violation_notification.title, "Policy Violation Detected");
    }
}
