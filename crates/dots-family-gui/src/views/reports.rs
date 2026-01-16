use dots_family_common::types::Profile;
use gtk4::prelude::*;
use relm4::prelude::*;

pub struct Reports {
    profile: Profile,
    report_data: Vec<ReportEntry>,
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
                                            set_label: "2 hours 15 minutes screen time",
                                            add_css_class: "body",
                                            set_halign: gtk4::Align::Start,
                                        }
                                    },

                                    gtk4::Label {
                                        set_label: "85%",
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
                                            set_label: "14 hours 30 minutes total",
                                            add_css_class: "body",
                                            set_halign: gtk4::Align::Start,
                                        }
                                    },

                                    gtk4::Label {
                                        set_label: "78%",
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
                                            set_label: "Educational apps (40%)",
                                            add_css_class: "body",
                                            set_halign: gtk4::Align::Start,
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
                                            set_label: "2 blocked attempts this week",
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
                }
            }
        }
    }

    fn init(
        init: Self::Init,
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let report_data = vec![
            ReportEntry {
                date: "2025-01-16".to_string(),
                screen_time: 135,
                top_activity: "Educational Apps".to_string(),
                violations: 1,
            },
            ReportEntry {
                date: "2025-01-15".to_string(),
                screen_time: 98,
                top_activity: "Creative Apps".to_string(),
                violations: 0,
            },
        ];

        let model = Reports { profile: init, report_data };

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            ReportsMsg::UpdateProfile(profile) => {
                self.profile = profile;
            }
            ReportsMsg::RefreshReports => {}
            ReportsMsg::ExportReports => {}
            ReportsMsg::ViewDaily => {}
            ReportsMsg::ViewWeekly => {}
            ReportsMsg::ViewMonthly => {}
        }
    }
}
