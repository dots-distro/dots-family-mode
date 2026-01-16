pub mod activities;
pub mod app_info_cache;
pub mod approval_requests;
pub mod audit;
pub mod custom_rules;
pub mod daily_summaries;
pub mod events;
pub mod exceptions;
pub mod filter_lists;
pub mod filter_rules;
pub mod network_activity;
pub mod policy_cache;
pub mod policy_versions;
pub mod profiles;
pub mod sessions;
// pub mod terminal;  // Disabled due to missing table migrations
pub mod terminal_activity;
pub mod weekly_summaries;

pub use activities::ActivityQueries;
pub use approval_requests::ApprovalRequestQueries;
pub use audit::AuditQueries;
pub use daily_summaries::DailySummaryQueries;
pub use events::EventQueries;
pub use exceptions::ExceptionQueries;
pub use network_activity::NetworkActivityQueries;
pub use policy_versions::PolicyVersionQueries;
pub use profiles::ProfileQueries;
pub use sessions::SessionQueries;
// pub use terminal::*;  // Disabled due to missing table migrations
pub use terminal_activity::TerminalActivityQueries;
pub use weekly_summaries::WeeklySummaryQueries;
