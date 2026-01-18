use dots_family_common::types::Profile;
use gtk4::prelude::*;
use relm4::prelude::*;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SidebarRow {
    pub profile: Profile,
}

#[derive(Debug)]
pub enum SidebarRowMsg {
    Select,
}

#[relm4::factory(pub)]
impl FactoryComponent for SidebarRow {
    type Init = Profile;
    type Input = SidebarRowMsg;
    type Output = Profile;
    type CommandOutput = ();
    type ParentWidget = gtk4::ListBox;

    #[allow(unused_assignments)] // Used by relm4 macro
    view! {
        root = gtk4::ListBoxRow {
            set_activatable: true,
            connect_activate[sender] => move |_| {
                sender.input(SidebarRowMsg::Select);
            },

            gtk4::Box {
                set_orientation: gtk4::Orientation::Horizontal,
                set_spacing: 12,
                set_margin_all: 8,

                gtk4::Image {
                    set_icon_name: Some("avatar-default-symbolic"),
                    set_pixel_size: 32,
                },

                gtk4::Label {
                    #[watch]
                    set_label: &self.profile.name,
                    set_halign: gtk4::Align::Start,
                    add_css_class: "heading",
                }
            }
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self { profile: init }
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        match msg {
            SidebarRowMsg::Select => {
                sender.output(self.profile.clone()).unwrap();
            }
        }
    }
}
