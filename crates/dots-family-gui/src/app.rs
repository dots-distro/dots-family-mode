use crate::components::sidebar_row::{SidebarRow, SidebarRowMsg};
use crate::state::profile_store::ProfileStore;
use crate::views::child_interface::{ChildInterface, ChildInterfaceMsg};
use crate::views::dashboard::{Dashboard, DashboardMsg};
use crate::views::profile_editor::{ProfileEditor, ProfileEditorMsg};
use crate::views::reports::{Reports, ReportsMsg};
use dots_family_common::types::Profile;
use gtk4::prelude::*;
use libadwaita::prelude::*;
use relm4::factory::FactoryVecDeque;
use relm4::prelude::*;

#[derive(Debug, PartialEq, Eq)]
pub enum AppMode {
    Welcome,
    Dashboard,
    Reports,
    ChildView,
    Edit,
}

pub struct AppModel {
    store: ProfileStore,
    sidebar_rows: FactoryVecDeque<SidebarRow>,
    dashboard: Controller<Dashboard>,
    reports: Controller<Reports>,
    child_interface: Controller<ChildInterface>,
    profile_editor: Controller<ProfileEditor>,
    mode: AppMode,
    is_parent_mode: bool,
}

#[derive(Debug)]
pub enum AppMsg {
    SelectProfile(Profile),
    AddProfile,
    EditProfile,
    SaveProfile(Profile),
    CancelEdit,
    ShowDashboard,
    ShowReports,
    ShowChildView,
    ToggleMode,
}

#[relm4::component(pub)]
impl SimpleComponent for AppModel {
    type Init = ();
    type Input = AppMsg;
    type Output = ();

    view! {
        libadwaita::ApplicationWindow {
            set_title: Some("DOTS Family Mode"),
            set_default_size: (1000, 700),

            #[wrap(Some)]
            set_content = &libadwaita::NavigationSplitView {
                set_collapsed: false,

                #[wrap(Some)]
                set_sidebar = &libadwaita::NavigationPage {
                    set_title: "Navigation",
                    #[wrap(Some)]
                    set_child = &libadwaita::ToolbarView {
                        add_top_bar = &libadwaita::HeaderBar {
                            pack_start = &gtk4::Button {
                                #[watch]
                                set_visible: model.is_parent_mode,
                                set_icon_name: "list-add-symbolic",
                                connect_clicked => AppMsg::AddProfile,
                            },

                            pack_end = &gtk4::Button {
                                #[watch]
                                set_label: if model.is_parent_mode { "Child Mode" } else { "Parent Mode" },
                                set_icon_name: "system-users-symbolic",
                                connect_clicked => AppMsg::ToggleMode,
                            }
                        },

                        #[wrap(Some)]
                        set_content = &gtk4::Box {
                            set_orientation: gtk4::Orientation::Vertical,
                            set_spacing: 10,
                            set_margin_all: 10,

                            gtk4::Label {
                                #[watch]
                                set_label: if model.is_parent_mode { "Profiles" } else { "Quick Access" },
                                add_css_class: "title-2",
                                set_halign: gtk4::Align::Start,
                            },

                            gtk4::Box {
                                set_orientation: gtk4::Orientation::Vertical,
                                set_spacing: 8,
                                #[watch]
                                set_visible: model.is_parent_mode,

                                #[local_ref]
                                sidebar_list -> gtk4::ListBox {
                                    set_selection_mode: gtk4::SelectionMode::Single,
                                    set_css_classes: &["navigation-sidebar"],
                                }
                            },

                            gtk4::Box {
                                set_orientation: gtk4::Orientation::Vertical,
                                set_spacing: 8,
                                #[watch]
                                set_visible: model.is_parent_mode,

                                gtk4::Separator {},

                                gtk4::Label {
                                    set_label: "Views",
                                    add_css_class: "title-3",
                                    set_halign: gtk4::Align::Start,
                                },

                                gtk4::Button {
                                    set_label: "Dashboard",
                                    set_icon_name: "view-grid-symbolic",
                                    connect_clicked => AppMsg::ShowDashboard,
                                },

                                gtk4::Button {
                                    set_label: "Reports",
                                    set_icon_name: "document-open-symbolic",
                                    connect_clicked => AppMsg::ShowReports,
                                }
                            }
                        }
                    }
                },

                #[wrap(Some)]
                set_content = &libadwaita::NavigationPage {
                    #[watch]
                    set_title: match (&model.mode, model.is_parent_mode) {
                        (AppMode::Dashboard, true) => "Dashboard",
                        (AppMode::Reports, true) => "Reports",
                        (AppMode::ChildView, false) => "My Screen Time",
                        (AppMode::Edit, _) => "Edit Profile",
                        _ => "DOTS Family Mode",
                    },

                    #[wrap(Some)]
                    set_child = &libadwaita::ToolbarView {
                        add_top_bar = &libadwaita::HeaderBar {
                            #[watch]
                            set_show_end_title_buttons: true,

                            pack_end = &gtk4::Button {
                                #[watch]
                                set_visible: model.is_parent_mode && model.mode == AppMode::Dashboard,
                                set_icon_name: "document-edit-symbolic",
                                connect_clicked => AppMsg::EditProfile,
                            }
                        },

                        #[wrap(Some)]
                        set_content = &gtk4::Stack {
                            #[watch]
                            set_visible_child_name: match (&model.mode, model.is_parent_mode) {
                                (AppMode::Welcome, _) => "welcome",
                                (AppMode::Dashboard, true) => "dashboard",
                                (AppMode::Reports, true) => "reports",
                                (AppMode::ChildView, false) => "child",
                                (AppMode::Edit, _) => "edit",
                                (_, false) => "child",
                                _ => "welcome",
                            },

                            add_named[Some("welcome")] = &libadwaita::StatusPage {
                                set_title: "Welcome to DOTS Family Mode",
                                set_description: Some("Select a profile to manage or create a new one"),
                                set_icon_name: Some("avatar-default-symbolic"),
                            },

                            add_named[Some("dashboard")] = model.dashboard.widget(),
                            add_named[Some("reports")] = model.reports.widget(),
                            add_named[Some("child")] = model.child_interface.widget(),
                            add_named[Some("edit")] = model.profile_editor.widget(),
                        }
                    }
                }
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let store = ProfileStore::new();

        let sidebar_list = gtk4::ListBox::default();
        let mut sidebar_rows = FactoryVecDeque::builder()
            .launch(sidebar_list.clone())
            .forward(sender.input_sender(), |profile| AppMsg::SelectProfile(profile));

        {
            let mut guard = sidebar_rows.guard();
            for profile in &store.profiles {
                guard.push_back(profile.clone());
            }
        }

        let default_profile = store.profiles.first().cloned().unwrap_or_else(Profile::default);

        let dashboard = Dashboard::builder().launch(default_profile.clone()).detach();

        let reports = Reports::builder().launch(default_profile.clone()).detach();

        let child_interface = ChildInterface::builder().launch(default_profile.clone()).detach();

        let profile_editor = ProfileEditor::builder().launch((Profile::default(), true)).forward(
            sender.input_sender(),
            |output| match output {
                Some(profile) => AppMsg::SaveProfile(profile),
                None => AppMsg::CancelEdit,
            },
        );

        let model = AppModel {
            store,
            sidebar_rows,
            dashboard,
            reports,
            child_interface,
            profile_editor,
            mode: AppMode::Welcome,
            is_parent_mode: true,
        };

        let sidebar_list = model.sidebar_rows.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            AppMsg::SelectProfile(profile) => {
                self.store.selected_profile_id = Some(profile.id);
                self.dashboard.emit(DashboardMsg::UpdateProfile(profile.clone()));
                self.reports.emit(ReportsMsg::UpdateProfile(profile.clone()));
                self.child_interface.emit(ChildInterfaceMsg::UpdateProfile(profile));
                self.mode =
                    if self.is_parent_mode { AppMode::Dashboard } else { AppMode::ChildView };
            }
            AppMsg::AddProfile => {
                let new_profile = Profile::default();
                self.profile_editor
                    .sender()
                    .send(ProfileEditorMsg::Reset(new_profile, true))
                    .expect("Failed to init editor");
                self.mode = AppMode::Edit;
            }
            AppMsg::EditProfile => {
                if let Some(profile) = self.store.get_selected_profile() {
                    self.profile_editor
                        .sender()
                        .send(ProfileEditorMsg::Reset(profile.clone(), false))
                        .expect("Failed to init editor");
                    self.mode = AppMode::Edit;
                }
            }
            AppMsg::SaveProfile(profile) => {
                self.store.update_profile(profile.clone());

                let mut guard = self.sidebar_rows.guard();
                guard.clear();
                for p in &self.store.profiles {
                    guard.push_back(p.clone());
                }

                self.dashboard.emit(DashboardMsg::UpdateProfile(profile.clone()));
                self.reports.emit(ReportsMsg::UpdateProfile(profile.clone()));
                self.child_interface.emit(ChildInterfaceMsg::UpdateProfile(profile.clone()));
                self.store.selected_profile_id = Some(profile.id);

                self.mode =
                    if self.is_parent_mode { AppMode::Dashboard } else { AppMode::ChildView };
            }
            AppMsg::CancelEdit => {
                if self.store.selected_profile_id.is_some() {
                    self.mode =
                        if self.is_parent_mode { AppMode::Dashboard } else { AppMode::ChildView };
                } else {
                    self.mode = AppMode::Welcome;
                }
            }
            AppMsg::ShowDashboard => {
                self.mode = AppMode::Dashboard;
            }
            AppMsg::ShowReports => {
                self.mode = AppMode::Reports;
            }
            AppMsg::ShowChildView => {
                self.mode = AppMode::ChildView;
            }
            AppMsg::ToggleMode => {
                self.is_parent_mode = !self.is_parent_mode;
                self.mode = if self.is_parent_mode {
                    if self.store.selected_profile_id.is_some() {
                        AppMode::Dashboard
                    } else {
                        AppMode::Welcome
                    }
                } else {
                    AppMode::ChildView
                };
            }
        }
    }
}
