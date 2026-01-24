use chrono::{Datelike, Duration, Local};
use dots_family_common::types::Profile;
use gtk4::prelude::*;
use relm4::prelude::*;

use crate::daemon_client::{ActivityReport, DaemonClient, WeeklyReport};

pub struct Reports {
    profile: Profile,
    daemon_client: DaemonClient,
    current_report: Option<ActivityReport>,
    current_weekly: Option<WeeklyReport>,
    view_mode: ReportViewMode,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReportViewMode {
    Daily,
    Weekly,
    Monthly,
}

#[derive(Debug, Clone)]
pub struct ReportEntry {
    pub date: String,
    pub screen_time: u32,
    pub top_activity: String,
    pub violations: u32,
}

#[derive(Debug)]
pub enum ReportsMsg {
    UpdateProfile(Profile),
    RefreshReports,
    ExportReports,
    ViewDaily,
    ViewWeekly,
    ViewMonthly,
    UpdateDailyReport(ActivityReport),
    UpdateWeeklyReport(WeeklyReport),
    ExportComplete(String),
}

#[relm4::component(pub)]
impl SimpleComponent for Reports {
    type Init = Profile;
    type Input = ReportsMsg;
    type Output = ();

    view! {
        gtk4::ScrolledWindow {
            set_policy: (gtk4::PolicyType::Never, gtk4::PolicyType::Automatic),

            #[wrap(Some)]
            set_child = &gtk4::Box {
                set_orientation: gtk4::Orientation::Vertical,
                set_spacing: 20,
                set_margin_all: 20,

                gtk4::Box {
                    set_orientation: gtk4::Orientation::Horizontal,
                    set_spacing: 12,

                    gtk4::Label {
                        #[watch]
                        set_label: &format!("Activity Reports - {}", model.profile.name),
                        add_css_class: "title-1",
                        set_halign: gtk4::Align::Start,
                        set_hexpand: true,
                    },

                    gtk4::Box {
                        set_orientation: gtk4::Orientation::Horizontal,
                        set_spacing: 8,

                        gtk4::Button {
                            set_label: "Daily",
                            connect_clicked => ReportsMsg::ViewDaily,
                        },

                        gtk4::Button {
                            set_label: "Weekly",
                            connect_clicked => ReportsMsg::ViewWeekly,
                        },

                        gtk4::Button {
                            set_label: "Monthly",
                            connect_clicked => ReportsMsg::ViewMonthly,
                        },

                        gtk4::Button {
                            set_label: "Export",
                            set_icon_name: "document-save-symbolic",
                            connect_clicked => ReportsMsg::ExportReports,
                        },

                        gtk4::Button {
                            set_label: "Refresh",
                            set_icon_name: "view-refresh-symbolic",
                            connect_clicked => ReportsMsg::RefreshReports,
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
                            set_label: "Recent Activity Summary",
                            add_css_class: "title-2",
                            set_halign: gtk4::Align::Start,
                        },

                        gtk4::ListBox {
                            set_css_classes: &["boxed-list"],

                            gtk4::ListBoxRow {
                                #[wrap(Some)]
                                set_child = &gtk4::Box {
                                    set_orientation: gtk4::Orientation::Horizontal,
                                    set_spacing: 12,
                                    set_margin_all: 12,

                                    gtk4::Box {
                                        set_orientation: gtk4::Orientation::Vertical,
                                        set_hexpand: true,

                                        gtk4::Label {
                                            set_label: "Today",
                                            add_css_class: "heading",
                                            set_halign: gtk4::Align::Start,
                                        },

                                        gtk4::Label {
                                            #[watch]
                                            set_label: &if let Some(ref report) = model.current_report {
                                                format!("{} hours {} minutes screen time",
                                                    report.screen_time_minutes / 60,
                                                    report.screen_time_minutes % 60)
                                            } else {
                                                "Loading...".to_string()
                                            },
                                            add_css_class: "body",
                                            set_halign: gtk4::Align::Start,
                                        }
                                    },

                                    gtk4::Label {
                                        #[watch]
                                        set_label: &if let Some(ref report) = model.current_report {
                                            format!("{}%", (report.screen_time_minutes * 100) / 300)
                                        } else {
                                            "...".to_string()
                                        },
                                        add_css_class: "title-3",
                                    }
                                }
                            },

                            gtk4::ListBoxRow {
                                #[wrap(Some)]
                                set_child = &gtk4::Box {
                                    set_orientation: gtk4::Orientation::Horizontal,
                                    set_spacing: 12,
                                    set_margin_all: 12,

                                    gtk4::Box {
                                        set_orientation: gtk4::Orientation::Vertical,
                                        set_hexpand: true,

                                        gtk4::Label {
                                            set_label: "This Week",
                                            add_css_class: "heading",
                                            set_halign: gtk4::Align::Start,
                                        },

                                        gtk4::Label {
                                            #[watch]
                                            set_label: &if let Some(ref weekly) = model.current_weekly {
                                                format!("{} hours {} minutes total",
                                                    weekly.total_screen_time_minutes / 60,
                                                    weekly.total_screen_time_minutes % 60)
                                            } else {
                                                "Loading...".to_string()
                                            },
                                            add_css_class: "body",
                                            set_halign: gtk4::Align::Start,
                                        }
                                    },

                                    gtk4::Label {
                                        #[watch]
                                        set_label: &if let Some(ref weekly) = model.current_weekly {
                                            format!("{}%", weekly.educational_percentage as u32)
                                        } else {
                                            "...".to_string()
                                        },
                                        add_css_class: "title-3",
                                    }
                                }
                            },

                            gtk4::ListBoxRow {
                                #[wrap(Some)]
                                set_child = &gtk4::Box {
                                    set_orientation: gtk4::Orientation::Horizontal,
                                    set_spacing: 12,
                                    set_margin_all: 12,

                                    gtk4::Box {
                                        set_orientation: gtk4::Orientation::Vertical,
                                        set_hexpand: true,

                                        gtk4::Label {
                                            set_label: "Top Activity",
                                            add_css_class: "heading",
                                            set_halign: gtk4::Align::Start,
                                        },

                                         gtk4::Label {
                                             #[watch]
                                             set_label: &if let Some(ref report) = model.current_report {
                                                 format!("{} ({}%)",
                                                     report.top_activity,
                                                     report.apps_used.first().map(|app| app.percentage as u32).unwrap_or(0))
                                             } else {
                                                 "Loading...".to_string()
                                             },
                                             add_css_class: "body",
                                             set_halign: gtk4::Align::Start,
                                         },

                                         gtk4::Box {
                                             set_orientation: gtk4::Orientation::Vertical,
                                             set_spacing: 8,
                                             set_margin_top: 8,

                                             gtk4::Box {
                                                 set_orientation: gtk4::Orientation::Horizontal,
                                                 set_spacing: 8,

                                                 gtk4::Label {
                                                     set_label: "Education",
                                                     add_css_class: "caption",
                                                     set_width_chars: 12,
                                                     set_halign: gtk4::Align::Start,
                                                 },

                                                 gtk4::ProgressBar {
                                                     #[watch]
                                                     set_fraction: if let Some(ref report) = model.current_report {
                                                         report.apps_used.iter()
                                                             .find(|app| app.category == "Education")
                                                             .map(|app| app.percentage / 100.0)
                                                             .unwrap_or(0.0) as f64
                                                     } else { 0.0 },
                                                     set_hexpand: true,
                                                 },

                                                 gtk4::Label {
                                                     #[watch]
                                                     set_label: &if let Some(ref report) = model.current_report {
                                                         format!("{}%",
                                                             report.apps_used.iter()
                                                                 .find(|app| app.category == "Education")
                                                                 .map(|app| app.percentage as u32)
                                                                 .unwrap_or(0))
                                                     } else { "0%".to_string() },
                                                     add_css_class: "caption",
                                                     set_width_chars: 4,
                                                 }
                                             },

                                             gtk4::Box {
                                                 set_orientation: gtk4::Orientation::Horizontal,
                                                 set_spacing: 8,

                                                 gtk4::Label {
                                                     set_label: "Browser",
                                                     add_css_class: "caption",
                                                     set_width_chars: 12,
                                                     set_halign: gtk4::Align::Start,
                                                 },

                                                 gtk4::ProgressBar {
                                                     #[watch]
                                                     set_fraction: if let Some(ref report) = model.current_report {
                                                         report.apps_used.iter()
                                                             .find(|app| app.category == "Web Browser")
                                                             .map(|app| app.percentage / 100.0)
                                                             .unwrap_or(0.0) as f64
                                                     } else { 0.0 },
                                                     set_hexpand: true,
                                                 },

                                                 gtk4::Label {
                                                     #[watch]
                                                     set_label: &if let Some(ref report) = model.current_report {
                                                         format!("{}%",
                                                             report.apps_used.iter()
                                                                 .find(|app| app.category == "Web Browser")
                                                                 .map(|app| app.percentage as u32)
                                                                 .unwrap_or(0))
                                                     } else { "0%".to_string() },
                                                     add_css_class: "caption",
                                                     set_width_chars: 4,
                                                 }
                                             }
                                         }
                                    },

                                    gtk4::Image {
                                        set_icon_name: Some("applications-education-symbolic"),
                                        set_pixel_size: 24,
                                    }
                                }
                            },

                            gtk4::ListBoxRow {
                                #[wrap(Some)]
                                set_child = &gtk4::Box {
                                    set_orientation: gtk4::Orientation::Horizontal,
                                    set_spacing: 12,
                                    set_margin_all: 12,

                                    gtk4::Box {
                                        set_orientation: gtk4::Orientation::Vertical,
                                        set_hexpand: true,

                                        gtk4::Label {
                                            set_label: "Policy Violations",
                                            add_css_class: "heading",
                                            set_halign: gtk4::Align::Start,
                                        },

                                        gtk4::Label {
                                            #[watch]
                                            set_label: &if let Some(ref report) = model.current_report {
                                                if report.blocked_attempts > 0 {
                                                    format!("{} blocked attempts today", report.blocked_attempts)
                                                } else {
                                                    "No violations today".to_string()
                                                }
                                            } else {
                                                "Loading...".to_string()
                                            },
                                            add_css_class: "body",
                                            set_halign: gtk4::Align::Start,
                                        }
                                    },

                                    gtk4::Image {
                                        set_icon_name: Some("security-medium-symbolic"),
                                        set_pixel_size: 24,
                                    }
                                }
                            }
                        }
                    }
                },

                gtk4::Frame {
                    add_css_class: "card",
                    set_margin_top: 20,
                    #[wrap(Some)]
                    set_child = &gtk4::Box {
                        set_orientation: gtk4::Orientation::Vertical,
                        set_spacing: 12,
                        set_margin_all: 16,

                        gtk4::Label {
                            set_label: "Weekly Screen Time Chart",
                            add_css_class: "title-2",
                            set_halign: gtk4::Align::Start,
                        },

                        gtk4::Box {
                            set_orientation: gtk4::Orientation::Vertical,
                            set_spacing: 8,

                            gtk4::Box {
                                set_orientation: gtk4::Orientation::Horizontal,
                                set_spacing: 8,

                                gtk4::Label {
                                    set_label: "Mon",
                                    add_css_class: "caption",
                                    set_width_chars: 4,
                                },
                                gtk4::ProgressBar {
                                    set_fraction: 0.75,
                                    set_hexpand: true,
                                },
                                gtk4::Label {
                                    set_label: "2h 15m",
                                    add_css_class: "caption",
                                    set_width_chars: 6,
                                }
                            },

                            gtk4::Box {
                                set_orientation: gtk4::Orientation::Horizontal,
                                set_spacing: 8,

                                gtk4::Label {
                                    set_label: "Tue",
                                    add_css_class: "caption",
                                    set_width_chars: 4,
                                },
                                gtk4::ProgressBar {
                                    set_fraction: 0.60,
                                    set_hexpand: true,
                                },
                                gtk4::Label {
                                    set_label: "1h 48m",
                                    add_css_class: "caption",
                                    set_width_chars: 6,
                                }
                            },

                            gtk4::Box {
                                set_orientation: gtk4::Orientation::Horizontal,
                                set_spacing: 8,

                                gtk4::Label {
                                    set_label: "Wed",
                                    add_css_class: "caption",
                                    set_width_chars: 4,
                                },
                                gtk4::ProgressBar {
                                    set_fraction: 0.85,
                                    set_hexpand: true,
                                },
                                gtk4::Label {
                                    set_label: "2h 33m",
                                    add_css_class: "caption",
                                    set_width_chars: 6,
                                }
                            },

                            gtk4::Box {
                                set_orientation: gtk4::Orientation::Horizontal,
                                set_spacing: 8,

                                gtk4::Label {
                                    #[watch]
                                    set_label: &format!("Today ({})",
                                        chrono::Local::now().format("%a")),
                                    add_css_class: "caption",
                                    set_width_chars: 8,
                                },
                                gtk4::ProgressBar {
                                    #[watch]
                                    set_fraction: if let Some(ref report) = model.current_report {
                                        (report.screen_time_minutes as f64) / 180.0 // Max 3 hours = 180 minutes
                                    } else { 0.0 },
                                    set_hexpand: true,
                                },
                                gtk4::Label {
                                    #[watch]
                                    set_label: &if let Some(ref report) = model.current_report {
                                        format!("{}h {}m",
                                            report.screen_time_minutes / 60,
                                            report.screen_time_minutes % 60)
                                    } else { "0h 0m".to_string() },
                                    add_css_class: "caption",
                                    set_width_chars: 6,
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn init(
        init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Reports {
            profile: init.clone(),
            daemon_client: tokio::runtime::Handle::current().block_on(DaemonClient::new()),
            current_report: None,
            current_weekly: None,
            view_mode: ReportViewMode::Daily,
        };

        let daemon_client = model.daemon_client.clone();
        let profile_id = init.id.to_string();
        let sender_clone = sender.clone();

        tokio::spawn(async move {
            let today = Local::now().date_naive();
            if let Ok(report) = daemon_client.get_daily_report(&profile_id, today).await {
                sender_clone.input(ReportsMsg::UpdateDailyReport(report));
            }
        });

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            ReportsMsg::UpdateProfile(profile) => {
                self.profile = profile;
                sender.input(ReportsMsg::RefreshReports);
            }
            ReportsMsg::RefreshReports => {
                let daemon_client = self.daemon_client.clone();
                let profile_id = self.profile.id.to_string();
                let view_mode = self.view_mode.clone();

                tokio::spawn(async move {
                    match view_mode {
                        ReportViewMode::Daily => {
                            let today = Local::now().date_naive();
                            if let Ok(report) =
                                daemon_client.get_daily_report(&profile_id, today).await
                            {
                                sender.input(ReportsMsg::UpdateDailyReport(report));
                            }
                        }
                        ReportViewMode::Weekly => {
                            let week_start = Local::now().date_naive() - Duration::days(6);
                            if let Ok(report) =
                                daemon_client.get_weekly_report(&profile_id, week_start).await
                            {
                                sender.input(ReportsMsg::UpdateWeeklyReport(report));
                            }
                        }
                        ReportViewMode::Monthly => {
                            let month_start = Local::now().date_naive().with_day(1).unwrap();
                            if let Ok(report) =
                                daemon_client.get_weekly_report(&profile_id, month_start).await
                            {
                                sender.input(ReportsMsg::UpdateWeeklyReport(report));
                            }
                        }
                    }
                });
            }
            ReportsMsg::ExportReports => {
                let daemon_client = self.daemon_client.clone();
                let profile_id = self.profile.id.to_string();

                tokio::spawn(async move {
                    let start_date = Local::now().date_naive() - Duration::days(30);
                    let end_date = Local::now().date_naive();

                    if let Ok(json_data) = daemon_client
                        .export_reports(&profile_id, "json", start_date, end_date)
                        .await
                    {
                        sender.input(ReportsMsg::ExportComplete(json_data));
                    }
                });
            }
            ReportsMsg::ViewDaily => {
                self.view_mode = ReportViewMode::Daily;
                sender.input(ReportsMsg::RefreshReports);
            }
            ReportsMsg::ViewWeekly => {
                self.view_mode = ReportViewMode::Weekly;
                sender.input(ReportsMsg::RefreshReports);
            }
            ReportsMsg::ViewMonthly => {
                self.view_mode = ReportViewMode::Monthly;
                sender.input(ReportsMsg::RefreshReports);
            }
            ReportsMsg::UpdateDailyReport(report) => {
                self.current_report = Some(report);
            }
            ReportsMsg::UpdateWeeklyReport(report) => {
                self.current_weekly = Some(report);
            }
            ReportsMsg::ExportComplete(data) => {
                println!("Export completed: {} bytes", data.len());
            }
        }
    }
}
