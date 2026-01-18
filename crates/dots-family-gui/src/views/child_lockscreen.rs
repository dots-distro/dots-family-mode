use crate::daemon_client::DaemonClient;
use dots_family_common::types::Profile;
use gtk4::prelude::*;
use relm4::prelude::*;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum LockscreenReason {
    ScreenTimeLimitExceeded,
    BedtimeEnforcement,
    BlockedAppAccess(String),
    ParentalLock,
}

impl std::fmt::Display for LockscreenReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LockscreenReason::ScreenTimeLimitExceeded => {
                write!(f, "Screen time limit reached for today")
            }
            LockscreenReason::BedtimeEnforcement => {
                write!(f, "It's bedtime! Time to rest")
            }
            LockscreenReason::BlockedAppAccess(app) => {
                write!(f, "Access to {} is not allowed", app)
            }
            LockscreenReason::ParentalLock => {
                write!(f, "Computer locked by parent")
            }
        }
    }
}

pub struct ChildLockscreen {
    profile: Profile,
    daemon_client: DaemonClient,
    reason: LockscreenReason,
    countdown_seconds: u32,
    show_parent_override: bool,
    parent_password: String,
    authentication_failed: bool,
    authentication_attempts: u32,
}

#[derive(Debug)]
pub enum ChildLockscreenMsg {
    UpdateProfile(Profile),
    SetReason(LockscreenReason),
    UpdateCountdown(u32),
    ShowParentOverride,
    HideParentOverride,
    ParentPasswordChanged(String),
    AttemptParentAuth,
    AuthenticationResult(bool),
    RequestEmergencyAccess,
    AcceptRestriction,
    Tick,
}

#[relm4::component(pub)]
impl SimpleComponent for ChildLockscreen {
    type Init = (Profile, LockscreenReason);
    type Input = ChildLockscreenMsg;
    type Output = bool; // true = unlock granted, false = stay locked

    view! {
        #[root]
        gtk4::Box {
            set_orientation: gtk4::Orientation::Vertical,
            set_spacing: 0,
            add_css_class: "lockscreen",
            set_vexpand: true,
            set_hexpand: true,

            // Header with lock icon and reason
            gtk4::Box {
                set_orientation: gtk4::Orientation::Vertical,
                set_spacing: 20,
                set_margin_all: 40,
                set_valign: gtk4::Align::Center,
                set_halign: gtk4::Align::Center,
                set_vexpand: true,

                gtk4::Image {
                    set_icon_name: Some("changes-prevent-symbolic"),
                    set_pixel_size: 128,
                    add_css_class: "lockscreen-icon",
                },

                gtk4::Label {
                    #[watch]
                    set_label: &model.reason.to_string(),
                    add_css_class: "title-1",
                    set_wrap: true,
                    set_justify: gtk4::Justification::Center,
                },

                // Countdown or time information
                gtk4::Label {
                    #[watch]
                    set_label: &match model.reason {
                        LockscreenReason::ScreenTimeLimitExceeded => {
                            format!("You can use the computer again tomorrow")
                        },
                        LockscreenReason::BedtimeEnforcement => {
                            if model.countdown_seconds > 0 {
                                let hours = model.countdown_seconds / 3600;
                                let minutes = (model.countdown_seconds % 3600) / 60;
                                format!("Good morning in {}h {}m", hours, minutes)
                            } else {
                                "Good morning! You can use the computer now".to_string()
                            }
                        },
                        LockscreenReason::BlockedAppAccess(ref app) => {
                            format!("{} is not available during your current time window", app)
                        },
                        LockscreenReason::ParentalLock => {
                            "Ask a parent to unlock the computer".to_string()
                        },
                    },
                    add_css_class: "title-2",
                    set_wrap: true,
                    set_justify: gtk4::Justification::Center,
                },

                // Helpful message
                gtk4::Label {
                    #[watch]
                    set_label: &match model.reason {
                        LockscreenReason::ScreenTimeLimitExceeded => {
                            "💤 Great job managing your screen time today!\nTry reading, playing outside, or spending time with family."
                        },
                        LockscreenReason::BedtimeEnforcement => {
                            "😴 Getting enough sleep helps you learn and grow!\nTry reading a book or listening to calm music."
                        },
                        LockscreenReason::BlockedAppAccess(_) => {
                            "🕒 This app isn't available right now.\nTry an educational app or ask a parent for help."
                        },
                        LockscreenReason::ParentalLock => {
                            "🔒 Your parent has locked the computer.\nLet them know when you need to use it."
                        },
                    },
                    add_css_class: "body-large",
                    set_wrap: true,
                    set_justify: gtk4::Justification::Center,
                    set_margin_top: 20,
                },

                // Action buttons
                gtk4::Box {
                    set_orientation: gtk4::Orientation::Horizontal,
                    set_spacing: 12,
                    set_halign: gtk4::Align::Center,
                    set_margin_top: 40,

                    gtk4::Button {
                        set_label: "I Understand",
                        set_icon_name: "emblem-ok-symbolic",
                        add_css_class: "suggested-action",
                        connect_clicked => ChildLockscreenMsg::AcceptRestriction,
                    },

                    gtk4::Button {
                        #[watch]
                        set_visible: !matches!(model.reason, LockscreenReason::ParentalLock),
                        set_label: "Emergency Access",
                        set_icon_name: "dialog-warning-symbolic",
                        connect_clicked => ChildLockscreenMsg::RequestEmergencyAccess,
                    },

                    gtk4::Button {
                        set_label: "Parent Override",
                        set_icon_name: "system-users-symbolic",
                        connect_clicked => ChildLockscreenMsg::ShowParentOverride,
                    },
                },
            },

            // Parent override section (initially hidden)
            gtk4::Revealer {
                #[watch]
                set_reveal_child: model.show_parent_override,
                set_transition_type: gtk4::RevealerTransitionType::SlideUp,

                #[wrap(Some)]
                set_child = &gtk4::Frame {
                    add_css_class: "card",
                    set_margin_all: 20,

                    #[wrap(Some)]
                    set_child = &gtk4::Box {
                        set_orientation: gtk4::Orientation::Vertical,
                        set_spacing: 16,
                        set_margin_all: 20,

                        gtk4::Label {
                            set_label: "Parent Override",
                            add_css_class: "title-3",
                            set_halign: gtk4::Align::Start,
                        },

                        gtk4::Box {
                            set_orientation: gtk4::Orientation::Horizontal,
                            set_spacing: 12,

                            gtk4::Entry {
                                set_placeholder_text: Some("Enter parent password"),
                                set_input_purpose: gtk4::InputPurpose::Password,
                                set_visibility: false,
                                #[watch]
                                set_text: &model.parent_password,
                                connect_changed[sender] => move |entry| {
                                    sender.input(ChildLockscreenMsg::ParentPasswordChanged(
                                        entry.text().to_string()
                                    ));
                                },
                                connect_activate => ChildLockscreenMsg::AttemptParentAuth,
                            },

                            gtk4::Button {
                                set_label: "Unlock",
                                set_icon_name: "changes-allow-symbolic",
                                add_css_class: "suggested-action",
                                #[watch]
                                set_sensitive: !model.parent_password.is_empty(),
                                connect_clicked => ChildLockscreenMsg::AttemptParentAuth,
                            },
                        },

                        gtk4::Label {
                            #[watch]
                            set_visible: model.authentication_failed,
                            #[watch]
                            set_label: &format!("❌ Incorrect password. {} attempts remaining.",
                                3_u32.saturating_sub(model.authentication_attempts)),
                            add_css_class: "error",
                            set_wrap: true,
                        },

                        gtk4::Box {
                            set_orientation: gtk4::Orientation::Horizontal,
                            set_spacing: 12,
                            set_halign: gtk4::Align::End,

                            gtk4::Button {
                                set_label: "Cancel",
                                connect_clicked => ChildLockscreenMsg::HideParentOverride,
                            },
                        },
                    }
                }
            },
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let (profile, reason) = init;

        let daemon_client = relm4::tokio::task::block_in_place(|| {
            relm4::tokio::runtime::Handle::current().block_on(async { DaemonClient::new().await })
        });

        // Set countdown based on reason
        let countdown_seconds = match &reason {
            LockscreenReason::BedtimeEnforcement => {
                // Calculate seconds until wake time (e.g., 7 AM)
                // For demo, use 8 hours = 28800 seconds
                28800
            }
            _ => 0,
        };

        let model = ChildLockscreen {
            profile,
            daemon_client,
            reason,
            countdown_seconds,
            show_parent_override: false,
            parent_password: String::new(),
            authentication_failed: false,
            authentication_attempts: 0,
        };

        // Start countdown timer if needed
        if model.countdown_seconds > 0 {
            let sender_clone = sender.clone();
            relm4::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(1));
                loop {
                    interval.tick().await;
                    let _ = sender_clone.input(ChildLockscreenMsg::Tick);
                }
            });
        }

        // Apply lockscreen CSS styling
        root.add_css_class("lockscreen-background");

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            ChildLockscreenMsg::UpdateProfile(profile) => {
                self.profile = profile;
            }
            ChildLockscreenMsg::SetReason(reason) => {
                self.reason = reason;
            }
            ChildLockscreenMsg::UpdateCountdown(seconds) => {
                self.countdown_seconds = seconds;
            }
            ChildLockscreenMsg::ShowParentOverride => {
                self.show_parent_override = true;
                self.parent_password.clear();
                self.authentication_failed = false;
            }
            ChildLockscreenMsg::HideParentOverride => {
                self.show_parent_override = false;
                self.parent_password.clear();
                self.authentication_failed = false;
            }
            ChildLockscreenMsg::ParentPasswordChanged(password) => {
                self.parent_password = password;
                self.authentication_failed = false;
            }
            ChildLockscreenMsg::AttemptParentAuth => {
                if self.authentication_attempts >= 3 {
                    self.authentication_failed = true;
                    return;
                }

                let daemon_client = self.daemon_client.clone();
                let password = self.parent_password.clone();
                let sender = sender.clone();

                relm4::spawn(async move {
                    match daemon_client.authenticate_parent(&password).await {
                        Ok(_token) => {
                            let _ = sender.input(ChildLockscreenMsg::AuthenticationResult(true));
                        }
                        Err(_) => {
                            let _ = sender.input(ChildLockscreenMsg::AuthenticationResult(false));
                        }
                    }
                });
            }
            ChildLockscreenMsg::AuthenticationResult(success) => {
                if success {
                    let _ = sender.output(true);
                } else {
                    self.authentication_attempts += 1;
                    self.authentication_failed = true;
                    self.parent_password.clear();
                }
            }
            ChildLockscreenMsg::RequestEmergencyAccess => {
                // Log emergency access request
                println!("Emergency access requested by child");
                // Could notify parents via daemon
            }
            ChildLockscreenMsg::AcceptRestriction => {
                // Child acknowledges the restriction
                // Stay locked but show positive feedback
                println!("Child accepted restriction");
            }
            ChildLockscreenMsg::Tick => {
                if self.countdown_seconds > 0 {
                    self.countdown_seconds -= 1;
                    if self.countdown_seconds == 0 {
                        let _ = sender.output(true);
                    }
                }
            }
        }
    }
}
