use dots_family_common::types::{AgeGroup, Profile};
use gtk4::prelude::*;
use libadwaita::prelude::*;
use relm4::prelude::*;

pub struct ProfileEditor {
    profile: Profile,
    is_new: bool,
}

#[derive(Debug)]
pub enum ProfileEditorMsg {
    Reset(Profile, bool),
    UpdateName(String),
    UpdateAgeGroup(u32),
    UpdateDailyLimit(f64),
    ToggleActive(bool),
    Save,
    Cancel,
}

#[relm4::component(pub)]
impl SimpleComponent for ProfileEditor {
    type Init = (Profile, bool);
    type Input = ProfileEditorMsg;
    type Output = Option<Profile>;

    view! {
        gtk4::Box {
            set_orientation: gtk4::Orientation::Vertical,
            set_spacing: 20,
            set_margin_all: 20,

            libadwaita::PreferencesGroup {
                set_title: "Basic Information",

                #[wrap(Some)]
                set_header_suffix = &gtk4::Box {
                    set_orientation: gtk4::Orientation::Horizontal,
                    set_spacing: 12,
                },

                add = &libadwaita::ActionRow {
                    set_title: "Name",
                    add_suffix = &gtk4::Entry {
                        #[watch]
                        set_text: &model.profile.name,
                        connect_changed[sender] => move |entry| {
                            sender.input(ProfileEditorMsg::UpdateName(entry.text().to_string()));
                        }
                    }
                },

                add = &libadwaita::ComboRow {
                    set_title: "Age Group",
                    set_model: Some(&gtk4::StringList::new(&["5-7 (Early Elementary)", "8-12 (Late Elementary)", "13-17 (High School)"])),
                    #[watch]
                    set_selected: match model.profile.age_group {
                        AgeGroup::EarlyElementary => 0,
                        AgeGroup::LateElementary => 1,
                        AgeGroup::HighSchool => 2,
                    },
                    connect_selected_notify[sender] => move |row| {
                        sender.input(ProfileEditorMsg::UpdateAgeGroup(row.selected()));
                    }
                }
            },

            libadwaita::PreferencesGroup {
                set_title: "Limits",

                add = &libadwaita::ActionRow {
                    set_title: "Daily Screen Time (Minutes)",
                    add_suffix = &gtk4::SpinButton {
                        set_range: (0.0, 1440.0),
                        set_increments: (15.0, 60.0),
                        #[watch]
                        set_value: model.profile.config.screen_time.daily_limit_minutes as f64,
                        connect_value_changed[sender] => move |btn| {
                            sender.input(ProfileEditorMsg::UpdateDailyLimit(btn.value()));
                        }
                    }
                }
            },

            libadwaita::PreferencesGroup {
                set_title: "Status",

                add = &libadwaita::SwitchRow {
                    set_title: "Active",
                    set_subtitle: "Enforce rules for this profile",
                    #[watch]
                    set_active: model.profile.active,
                    connect_active_notify[sender] => move |row| {
                        sender.input(ProfileEditorMsg::ToggleActive(row.is_active()));
                    }
                }
            },

            gtk4::Box {
                set_orientation: gtk4::Orientation::Horizontal,
                set_spacing: 12,
                set_halign: gtk4::Align::End,

                gtk4::Button {
                    set_label: "Cancel",
                    connect_clicked[sender] => move |_| {
                        sender.input(ProfileEditorMsg::Cancel);
                    }
                },

                gtk4::Button {
                    set_label: if model.is_new { "Create Profile" } else { "Save Changes" },
                    add_css_class: "suggested-action",
                    connect_clicked[sender] => move |_| {
                        sender.input(ProfileEditorMsg::Save);
                    }
                }
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = ProfileEditor { profile: init.0, is_new: init.1 };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            ProfileEditorMsg::Reset(profile, is_new) => {
                self.profile = profile;
                self.is_new = is_new;
            }
            ProfileEditorMsg::UpdateName(name) => {
                self.profile.name = name;
            }
            ProfileEditorMsg::UpdateAgeGroup(index) => {
                self.profile.age_group = match index {
                    0 => AgeGroup::EarlyElementary,
                    1 => AgeGroup::LateElementary,
                    _ => AgeGroup::HighSchool,
                };
            }
            ProfileEditorMsg::UpdateDailyLimit(limit) => {
                self.profile.config.screen_time.daily_limit_minutes = limit as u32;
            }
            ProfileEditorMsg::ToggleActive(active) => {
                self.profile.active = active;
            }
            ProfileEditorMsg::Save => {
                sender.output(Some(self.profile.clone())).unwrap();
            }
            ProfileEditorMsg::Cancel => {
                sender.output(None).unwrap();
            }
        }
    }
}
