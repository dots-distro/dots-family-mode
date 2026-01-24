use dots_family_common::types::Profile;
use gtk4::prelude::*;
use relm4::prelude::*;

use crate::daemon_client::DaemonClient;

pub struct ChildInterface {
    profile: Profile,
    daemon_client: DaemonClient,
    remaining_time: u32,
    current_activity: String,
    time_limit_warning: bool,
}

#[derive(Debug)]
pub enum ChildInterfaceMsg {
    UpdateProfile(Profile),
    UpdateRemainingTime(u32),
    UpdateActivity(String),
    ShowTimeWarning(bool),
    RefreshData,
    RequestExtraTime,
    RequestPermission(String),
}

#[relm4::component(pub)]
impl SimpleComponent for ChildInterface {
    type Init = Profile;
    type Input = ChildInterfaceMsg;
    type Output = ();

    view! {
        gtk4::Box {
            set_orientation: gtk4::Orientation::Vertical,
            set_spacing: 20,
            set_margin_all: 20,

            gtk4::Box {
                set_orientation: gtk4::Orientation::Horizontal,
                set_spacing: 12,
                set_halign: gtk4::Align::Center,

                gtk4::Image {
                    set_icon_name: Some("face-smile-symbolic"),
                    set_pixel_size: 64,
                },

                gtk4::Box {
                    set_orientation: gtk4::Orientation::Vertical,
                    set_valign: gtk4::Align::Center,

                    gtk4::Label {
                        #[watch]
                        set_label: &format!("Hi {}!", model.profile.name),
                        add_css_class: "title-1",
                        set_halign: gtk4::Align::Start,
                    },

                    gtk4::Label {
                        set_label: "Your screen time status",
                        add_css_class: "subtitle",
                        set_halign: gtk4::Align::Start,
                    }
                }
            },

            gtk4::Frame {
                add_css_class: "card",
                #[wrap(Some)]
                set_child = &gtk4::Box {
                    set_orientation: gtk4::Orientation::Vertical,
                    set_spacing: 16,
                    set_margin_all: 20,

                    gtk4::Box {
                        set_orientation: gtk4::Orientation::Horizontal,
                        set_spacing: 12,

                        gtk4::Image {
                            set_icon_name: Some("preferences-system-time-symbolic"),
                            set_pixel_size: 48,
                        },

                        gtk4::Box {
                            set_orientation: gtk4::Orientation::Vertical,
                            set_hexpand: true,

                            gtk4::Label {
                                #[watch]
                                set_label: &format!("{} minutes left today", model.remaining_time),
                                add_css_class: "title-2",
                                set_halign: gtk4::Align::Start,
                            },

                            gtk4::ProgressBar {
                                #[watch]
                                set_fraction: {
                                    let total = model.profile.config.screen_time.daily_limit_minutes as f64;
                                    let remaining = model.remaining_time as f64;
                                    if total > 0.0 {
                                        remaining / total
                                    } else {
                                        0.0
                                    }
                                },
                                #[watch]
                                add_css_class: if model.remaining_time <= 15 { "warning" } else { "normal" },
                            }
                        }
                    },

                    gtk4::Label {
                        #[watch]
                        set_label: if model.time_limit_warning {
                            "⚠️ You're almost out of screen time for today!"
                        } else if model.remaining_time <= 30 {
                            "💡 Consider wrapping up your current activity soon"
                        } else {
                            "🎯 You're doing great managing your screen time!"
                        },
                        set_wrap: true,
                        add_css_class: "body",
                    }
                }
            },

            gtk4::Frame {
                add_css_class: "card",
                #[wrap(Some)]
                set_child = &gtk4::Box {
                    set_orientation: gtk4::Orientation::Vertical,
                    set_spacing: 12,
                    set_margin_all: 20,

                    gtk4::Label {
                        set_label: "Current Activity",
                        add_css_class: "title-3",
                        set_halign: gtk4::Align::Start,
                    },

                    gtk4::Box {
                        set_orientation: gtk4::Orientation::Horizontal,
                        set_spacing: 12,

                        gtk4::Image {
                            set_icon_name: Some("applications-system-symbolic"),
                            set_pixel_size: 24,
                        },

                        gtk4::Label {
                            #[watch]
                            set_label: &model.current_activity,
                            add_css_class: "body",
                            set_wrap: true,
                            set_hexpand: true,
                        }
                    }
                }
            },

            gtk4::Box {
                set_orientation: gtk4::Orientation::Horizontal,
                set_spacing: 12,
                set_homogeneous: true,
                set_halign: gtk4::Align::Center,

                gtk4::Button {
                    set_label: "Request Extra Time",
                    set_icon_name: "list-add-symbolic",
                    add_css_class: "suggested-action",
                    connect_clicked => ChildInterfaceMsg::RequestExtraTime,
                },

                gtk4::Button {
                    set_label: "Need Help?",
                    set_icon_name: "help-info-symbolic",
                },

                gtk4::Button {
                    set_label: "Take a Break",
                    set_icon_name: "media-playback-pause-symbolic",
                    add_css_class: "destructive-action",
                }
            },

            gtk4::Separator {
                set_margin_top: 20,
                set_margin_bottom: 10,
            },

            gtk4::Box {
                set_orientation: gtk4::Orientation::Vertical,
                set_spacing: 8,

                gtk4::Label {
                    set_label: "🌟 Today's Achievements",
                    add_css_class: "title-3",
                    set_halign: gtk4::Align::Start,
                },

                gtk4::Box {
                    set_orientation: gtk4::Orientation::Horizontal,
                    set_spacing: 8,

                    gtk4::Label {
                        set_label: "✅ Stayed within screen time goals",
                        add_css_class: "body",
                    },

                    gtk4::Label {
                        set_label: "📚 Used educational apps for 60% of time",
                        add_css_class: "body",
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

        let model = ChildInterface {
            profile: init,
            daemon_client,
            remaining_time: 45,
            current_activity: "Reading app".to_string(),
            time_limit_warning: false,
        };

        let sender_clone = sender.clone();
        relm4::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            loop {
                interval.tick().await;
                let _ = sender_clone.input(ChildInterfaceMsg::RefreshData);
            }
        });

        let _ = sender.input(ChildInterfaceMsg::RefreshData);

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            ChildInterfaceMsg::UpdateProfile(profile) => {
                self.profile = profile;
            }
            ChildInterfaceMsg::UpdateRemainingTime(time) => {
                self.remaining_time = time;
                let _ = sender.input(ChildInterfaceMsg::ShowTimeWarning(time <= 15));
            }
            ChildInterfaceMsg::UpdateActivity(activity) => {
                self.current_activity = activity;
            }
            ChildInterfaceMsg::ShowTimeWarning(warning) => {
                self.time_limit_warning = warning;
            }
            ChildInterfaceMsg::RefreshData => {
                let daemon_client = self.daemon_client.clone();
                let sender = sender.clone();
                relm4::spawn(async move {
                    if let Ok(time) = daemon_client.get_remaining_time().await {
                        let _ = sender.input(ChildInterfaceMsg::UpdateRemainingTime(time));
                    }

                    if let Ok(profile) = daemon_client.get_active_profile().await {
                        let _ = sender.input(ChildInterfaceMsg::UpdateActivity(format!(
                            "Current: {}",
                            profile
                        )));
                    }
                });
            }
            ChildInterfaceMsg::RequestExtraTime => {}
            ChildInterfaceMsg::RequestPermission(_request) => {}
        }
    }
}
