use crate::daemon_client::DaemonClient;
use dots_family_common::types::Profile;
use gtk4::prelude::*;
use relm4::prelude::*;

pub struct Dashboard {
    profile: Profile,
    daemon_client: DaemonClient,
    remaining_time: u32,
    current_activity: String,
    connection_status: bool,
}

#[derive(Debug)]
pub enum DashboardMsg {
    UpdateProfile(Profile),
    UpdateRemainingTime(u32),
    UpdateActivity(String),
    UpdateConnectionStatus(bool),
    RefreshData,
    ConnectToDaemon,
}

#[relm4::component(pub)]
impl SimpleComponent for Dashboard {
    type Init = Profile;
    type Input = DashboardMsg;
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

                    gtk4::Image {
                        set_icon_name: Some("avatar-default-symbolic"),
                        set_pixel_size: 64,
                    },

                    gtk4::Box {
                        set_orientation: gtk4::Orientation::Vertical,
                        set_valign: gtk4::Align::Center,
                        set_hexpand: true,

                        gtk4::Label {
                            #[watch]
                            set_label: &model.profile.name,
                            add_css_class: "title-1",
                            set_halign: gtk4::Align::Start,
                        },

                        gtk4::Label {
                            #[watch]
                            set_label: &format!("{:?}", model.profile.age_group),
                            add_css_class: "subtitle",
                            set_halign: gtk4::Align::Start,
                        }
                    },

                    gtk4::Box {
                        set_orientation: gtk4::Orientation::Vertical,
                        set_valign: gtk4::Align::Center,

                        gtk4::Label {
                            #[watch]
                            set_label: if model.connection_status { "Connected" } else { "Disconnected" },
                            add_css_class: if model.connection_status { "success" } else { "error" },
                        },

                        gtk4::Button {
                            set_label: "Refresh",
                            set_icon_name: "view-refresh-symbolic",
                            connect_clicked => DashboardMsg::RefreshData,
                        }
                    }
                },

                gtk4::Box {
                    set_orientation: gtk4::Orientation::Horizontal,
                    set_spacing: 12,
                    set_homogeneous: true,

                    gtk4::Frame {
                        add_css_class: "card",
                        #[wrap(Some)]
                        set_child = &gtk4::Box {
                            set_orientation: gtk4::Orientation::Vertical,
                            set_spacing: 8,
                            set_margin_all: 16,

                            gtk4::Box {
                                set_orientation: gtk4::Orientation::Horizontal,
                                set_spacing: 8,

                                gtk4::Image {
                                    set_icon_name: Some("preferences-system-time-symbolic"),
                                    set_pixel_size: 24,
                                },

                                gtk4::Label {
                                    set_label: "Screen Time",
                                    add_css_class: "heading",
                                    set_hexpand: true,
                                    set_halign: gtk4::Align::Start,
                                }
                            },

                            gtk4::Box {
                                set_orientation: gtk4::Orientation::Vertical,
                                set_spacing: 4,

                                gtk4::Label {
                                    #[watch]
                                    set_label: &format!("{}m remaining", model.remaining_time),
                                    add_css_class: "title-2",
                                },

                                gtk4::ProgressBar {
                                    #[watch]
                                    set_fraction: {
                                        let total = model.profile.config.screen_time.daily_limit_minutes as f64;
                                        let remaining = model.remaining_time as f64;
                                        if total > 0.0 {
                                            (total - remaining) / total
                                        } else {
                                            0.0
                                        }
                                    },
                                }
                            }
                        }
                    },

                    gtk4::Frame {
                        add_css_class: "card",
                        #[wrap(Some)]
                        set_child = &gtk4::Box {
                            set_orientation: gtk4::Orientation::Vertical,
                            set_spacing: 8,
                            set_margin_all: 16,

                            gtk4::Box {
                                set_orientation: gtk4::Orientation::Horizontal,
                                set_spacing: 8,

                                gtk4::Image {
                                    set_icon_name: Some("applications-system-symbolic"),
                                    set_pixel_size: 24,
                                },

                                gtk4::Label {
                                    set_label: "Current Activity",
                                    add_css_class: "heading",
                                    set_hexpand: true,
                                    set_halign: gtk4::Align::Start,
                                }
                            },

                            gtk4::Label {
                                #[watch]
                                set_label: &model.current_activity,
                                add_css_class: "title-3",
                                set_wrap: true,
                            }
                        }
                    },

                    gtk4::Frame {
                        add_css_class: "card",
                        #[wrap(Some)]
                        set_child = &gtk4::Box {
                            set_orientation: gtk4::Orientation::Vertical,
                            set_spacing: 8,
                            set_margin_all: 16,

                            gtk4::Box {
                                set_orientation: gtk4::Orientation::Horizontal,
                                set_spacing: 8,

                                gtk4::Image {
                                    set_icon_name: Some("security-high-symbolic"),
                                    set_pixel_size: 24,
                                },

                                gtk4::Label {
                                    set_label: "Protection Status",
                                    add_css_class: "heading",
                                    set_hexpand: true,
                                    set_halign: gtk4::Align::Start,
                                }
                            },

                            gtk4::Label {
                                #[watch]
                                set_label: if model.profile.active { "Active" } else { "Paused" },
                                add_css_class: "title-2",
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
        let daemon_client = relm4::tokio::task::block_in_place(|| {
            relm4::tokio::runtime::Handle::current().block_on(async { DaemonClient::new().await })
        });

        let model = Dashboard {
            profile: init,
            daemon_client,
            remaining_time: 120,
            current_activity: "No activity detected".to_string(),
            connection_status: false,
        };

        let sender_clone = sender.clone();
        relm4::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
            loop {
                interval.tick().await;
                let _ = sender_clone.input(DashboardMsg::RefreshData);
            }
        });

        let _ = sender.input(DashboardMsg::ConnectToDaemon);

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            DashboardMsg::UpdateProfile(profile) => {
                self.profile = profile;
            }
            DashboardMsg::UpdateRemainingTime(time) => {
                self.remaining_time = time;
            }
            DashboardMsg::UpdateActivity(activity) => {
                self.current_activity = activity;
            }
            DashboardMsg::UpdateConnectionStatus(status) => {
                self.connection_status = status;
            }
            DashboardMsg::ConnectToDaemon => {
                let daemon_client = self.daemon_client.clone();
                let sender = sender.clone();
                relm4::spawn(async move {
                    match daemon_client.connect().await {
                        Ok(_) => {
                            let _ = sender.input(DashboardMsg::UpdateConnectionStatus(true));
                        }
                        Err(_) => {
                            let _ = sender.input(DashboardMsg::UpdateConnectionStatus(false));
                        }
                    }
                });
            }
            DashboardMsg::RefreshData => {
                if self.connection_status {
                    let daemon_client = self.daemon_client.clone();
                    let sender = sender.clone();
                    relm4::spawn(async move {
                        if let Ok(time) = daemon_client.get_remaining_time().await {
                            let _ = sender.input(DashboardMsg::UpdateRemainingTime(time));
                        }

                        if let Ok(active_profile) = daemon_client.get_active_profile().await {
                            let _ = sender.input(DashboardMsg::UpdateActivity(format!(
                                "Profile: {}",
                                active_profile
                            )));
                        }
                    });
                } else {
                    let _ = sender.input(DashboardMsg::ConnectToDaemon);
                }
            }
        }
    }
}
