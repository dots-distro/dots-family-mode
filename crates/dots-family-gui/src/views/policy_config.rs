use dots_family_common::types::{ApplicationMode, Profile};
use gtk4::prelude::*;
use relm4::prelude::*;

use crate::daemon_client::DaemonClient;

#[derive(Debug)]
pub struct PolicyConfig {
    profile: Profile,
    #[allow(dead_code)]
    daemon_client: DaemonClient,
    application_mode: ApplicationMode,
    daily_time_limit_minutes: u32,
    bedtime_hour: u32,
    wakeup_hour: u32,
    allowed_apps: String,
    blocked_apps: String,
}

#[derive(Debug)]
pub enum PolicyConfigMsg {
    UpdateProfile(Profile),
    SetApplicationMode(ApplicationMode),
    SetDailyTimeLimit(u32),
    SetBedtimeHour(u32),
    SetWakeupHour(u32),
    SetAllowedApps(String),
    SetBlockedApps(String),
    SaveConfiguration,
    LoadConfiguration,
}

#[relm4::component(pub)]
impl SimpleComponent for PolicyConfig {
    type Init = Profile;
    type Input = PolicyConfigMsg;
    type Output = ();

    view! {
        gtk4::ScrolledWindow {
            set_policy: (gtk4::PolicyType::Never, gtk4::PolicyType::Automatic),

            gtk4::Box {
                set_orientation: gtk4::Orientation::Vertical,
                set_spacing: 16,
                set_margin_all: 20,

                gtk4::Frame {
                    add_css_class: "card",
                    #[wrap(Some)]
                    set_child = &gtk4::Box {
                        set_orientation: gtk4::Orientation::Vertical,
                        set_spacing: 12,
                        set_margin_all: 16,

                        gtk4::Label {
                            set_label: "Application Control Settings",
                            add_css_class: "heading",
                            set_halign: gtk4::Align::Start,
                        },

                        gtk4::Box {
                            set_orientation: gtk4::Orientation::Horizontal,
                            set_spacing: 8,

                            gtk4::Label {
                                set_label: "Application Mode:",
                                set_hexpand: false,
                                set_halign: gtk4::Align::Start,
                            },

                            gtk4::DropDown::from_strings(&[
                                "Allowlist (Only allowed apps)",
                                "Blocklist (Block specific apps)"
                            ]) {
                                set_selected: match model.application_mode {
                                    ApplicationMode::Allowlist => 0,
                                    ApplicationMode::Blocklist => 1,
                                },
                                connect_selected_notify => PolicyConfigMsg::SetApplicationMode(
                                    ApplicationMode::Allowlist
                                )
                            }
                        },

                        gtk4::Box {
                            set_orientation: gtk4::Orientation::Vertical,
                            set_spacing: 4,

                            gtk4::Label {
                                set_label: "Allowed Applications (comma-separated):",
                                set_halign: gtk4::Align::Start,
                            },

                            gtk4::Entry {
                                set_text: &model.allowed_apps,
                            }
                        },

                        gtk4::Box {
                            set_orientation: gtk4::Orientation::Vertical,
                            set_spacing: 4,

                            gtk4::Label {
                                set_label: "Blocked Applications (comma-separated):",
                                set_halign: gtk4::Align::Start,
                            },

                            gtk4::Entry {
                                set_text: &model.blocked_apps,
                            }
                        }
                    }
                },

                gtk4::Frame {
                    add_css_class: "card",
                    #[wrap(Some)]
                    set_child = &gtk4::Box {
                        set_orientation: gtk4::Orientation::Vertical,
                        set_spacing: 12,
                        set_margin_all: 16,

                        gtk4::Label {
                            set_label: "Time Management Settings",
                            add_css_class: "heading",
                            set_halign: gtk4::Align::Start,
                        },

                        gtk4::Box {
                            set_orientation: gtk4::Orientation::Horizontal,
                            set_spacing: 8,

                            gtk4::Label {
                                set_label: "Daily Time Limit (minutes):",
                                set_hexpand: false,
                            },

                            gtk4::SpinButton::with_range(0.0, 1440.0, 15.0) {
                                set_value: model.daily_time_limit_minutes as f64,
                            }
                        },

                        gtk4::Box {
                            set_orientation: gtk4::Orientation::Horizontal,
                            set_spacing: 8,

                            gtk4::Label {
                                set_label: "Bedtime Hour (24h format):",
                                set_hexpand: false,
                            },

                            gtk4::SpinButton::with_range(0.0, 23.0, 1.0) {
                                set_value: model.bedtime_hour as f64,
                            }
                        },

                        gtk4::Box {
                            set_orientation: gtk4::Orientation::Horizontal,
                            set_spacing: 8,

                            gtk4::Label {
                                set_label: "Wakeup Hour (24h format):",
                                set_hexpand: false,
                            },

                            gtk4::SpinButton::with_range(0.0, 23.0, 1.0) {
                                set_value: model.wakeup_hour as f64,
                            }
                        }
                    }
                },

                gtk4::Box {
                    set_orientation: gtk4::Orientation::Horizontal,
                    set_spacing: 8,
                    set_halign: gtk4::Align::End,

                    gtk4::Button {
                        set_label: "Load Configuration",
                        add_css_class: "suggested-action",
                        connect_clicked => PolicyConfigMsg::LoadConfiguration,
                    },

                    gtk4::Button {
                        set_label: "Save Configuration",
                        add_css_class: "suggested-action",
                        connect_clicked => PolicyConfigMsg::SaveConfiguration,
                    }
                }
            }
        }
    }

    fn init(
        init: Self::Init,
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let daemon_client = relm4::tokio::task::block_in_place(|| {
            relm4::tokio::runtime::Handle::current().block_on(async { DaemonClient::new().await })
        });

        let model = PolicyConfig {
            profile: init.clone(),
            daemon_client,
            application_mode: init.config.applications.mode,
            daily_time_limit_minutes: init.config.screen_time.daily_limit_minutes,
            bedtime_hour: 21,
            wakeup_hour: 7,
            allowed_apps: init.config.applications.allowed.join(", "),
            blocked_apps: init.config.applications.blocked.join(", "),
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            PolicyConfigMsg::UpdateProfile(profile) => {
                self.profile = profile;
            }
            PolicyConfigMsg::SetApplicationMode(mode) => {
                self.application_mode = mode;
                self.profile.config.applications.mode = mode;
            }
            PolicyConfigMsg::SetDailyTimeLimit(minutes) => {
                self.daily_time_limit_minutes = minutes;
                self.profile.config.screen_time.daily_limit_minutes = minutes;
            }
            PolicyConfigMsg::SetBedtimeHour(hour) => {
                self.bedtime_hour = hour;
            }
            PolicyConfigMsg::SetWakeupHour(hour) => {
                self.wakeup_hour = hour;
            }
            PolicyConfigMsg::SetAllowedApps(apps) => {
                self.allowed_apps = apps.clone();
                self.profile.config.applications.allowed = apps
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            PolicyConfigMsg::SetBlockedApps(apps) => {
                self.blocked_apps = apps.clone();
                self.profile.config.applications.blocked = apps
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            PolicyConfigMsg::SaveConfiguration => {
                println!("Saving configuration: {:?}", self.profile);
            }
            PolicyConfigMsg::LoadConfiguration => {
                println!("Loading configuration for profile: {}", self.profile.name);
            }
        }
    }
}
