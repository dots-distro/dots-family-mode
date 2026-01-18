use dots_family_common::types::Profile;
use gtk4::prelude::*;
use relm4::prelude::*;

pub struct Overview {
    profile: Profile,
}

#[derive(Debug)]
pub enum OverviewMsg {
    UpdateProfile(Profile),
}

#[relm4::component(pub)]
impl SimpleComponent for Overview {
    type Init = Profile;
    type Input = OverviewMsg;
    type Output = ();

    view! {
        gtk4::Box {
            set_orientation: gtk4::Orientation::Vertical,
            set_spacing: 20,
            set_margin_all: 20,

            // Header
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
                }
            },

            // Stats cards
            gtk4::Box {
                set_orientation: gtk4::Orientation::Horizontal,
                set_spacing: 12,
                set_homogeneous: true,

                gtk4::Frame {
                    #[wrap(Some)]
                    set_child = &gtk4::Box {
                        set_orientation: gtk4::Orientation::Vertical,
                        set_spacing: 6,
                        set_margin_all: 12,

                        gtk4::Label {
                            set_label: "Screen Time",
                            add_css_class: "heading",
                        },

                        gtk4::Label {
                            #[watch]
                            set_label: &format!("{}m / {}m",
                                45, // Placeholder for actual usage
                                model.profile.config.screen_time.daily_limit_minutes
                            ),
                            add_css_class: "title-2",
                        }
                    }
                },

                gtk4::Frame {
                    #[wrap(Some)]
                    set_child = &gtk4::Box {
                        set_orientation: gtk4::Orientation::Vertical,
                        set_spacing: 6,
                        set_margin_all: 12,

                        gtk4::Label {
                            set_label: "Status",
                            add_css_class: "heading",
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

    fn init(
        init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Overview { profile: init };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            OverviewMsg::UpdateProfile(p) => {
                self.profile = p;
            }
        }
    }
}
