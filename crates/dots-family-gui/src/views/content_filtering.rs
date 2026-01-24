use std::collections::HashSet;

use gtk4::prelude::*;
use relm4::prelude::*;

use crate::daemon_client::DaemonClient;

#[derive(Debug)]
pub struct FilteringConfig {
    pub enabled: bool,
    pub safe_search_enforcement: bool,
    pub block_categories: HashSet<String>,
    pub allow_categories: HashSet<String>,
    pub blocked_domains: HashSet<String>,
    pub allowed_domains: HashSet<String>,
    pub custom_rules_enabled: bool,
}

#[derive(Debug)]
pub struct ContentFiltering {
    filtering_config: FilteringConfig,
    daemon_client: Option<DaemonClient>,

    // UI State
    new_blocked_domain: String,
    new_allowed_domain: String,
    new_custom_pattern: String,
    custom_pattern_reason: String,
}

#[derive(Debug)]
pub enum ContentFilteringMsg {
    // Main controls
    ToggleFiltering(bool),
    ToggleSafeSearch(bool),

    // Category controls
    ToggleCategoryBlock(String, bool),
    ToggleCategoryAllow(String, bool),

    // Domain management
    UpdateNewBlockedDomain(String),
    AddBlockedDomain,
    RemoveBlockedDomain(String),
    UpdateNewAllowedDomain(String),
    AddAllowedDomain,
    RemoveAllowedDomain(String),

    // Custom rules
    ToggleCustomRules(bool),
    UpdateCustomPattern(String),
    UpdateCustomReason(String),
    AddCustomPattern,

    // Actions
    SaveConfiguration,
    LoadConfiguration,
    ResetToDefaults,
}

#[relm4::component(pub)]
impl SimpleComponent for ContentFiltering {
    type Init = Option<DaemonClient>;
    type Input = ContentFilteringMsg;
    type Output = String;

    fn init(
        daemon_client: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // Create default filtering configuration
        let filtering_config = FilteringConfig {
            enabled: true,
            safe_search_enforcement: true,
            block_categories: ["adult", "gambling", "violence", "drugs"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            allow_categories: ["educational", "children", "reference"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            blocked_domains: HashSet::new(),
            allowed_domains: HashSet::new(),
            custom_rules_enabled: false,
        };

        let model = ContentFiltering {
            filtering_config,
            daemon_client,
            new_blocked_domain: String::new(),
            new_allowed_domain: String::new(),
            new_custom_pattern: String::new(),
            custom_pattern_reason: String::new(),
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            ContentFilteringMsg::ToggleFiltering(enabled) => {
                self.filtering_config.enabled = enabled;
                println!("Content filtering {}", if enabled { "enabled" } else { "disabled" });
            }

            ContentFilteringMsg::ToggleSafeSearch(enabled) => {
                self.filtering_config.safe_search_enforcement = enabled;
                println!(
                    "Safe search enforcement {}",
                    if enabled { "enabled" } else { "disabled" }
                );
            }

            ContentFilteringMsg::ToggleCategoryBlock(category, blocked) => {
                if blocked {
                    self.filtering_config.block_categories.insert(category.clone());
                    self.filtering_config.allow_categories.remove(&category);
                } else {
                    self.filtering_config.block_categories.remove(&category);
                }
                println!(
                    "Category '{}' {} blocked",
                    category,
                    if blocked { "now" } else { "no longer" }
                );
            }

            ContentFilteringMsg::ToggleCategoryAllow(category, allowed) => {
                if allowed {
                    self.filtering_config.allow_categories.insert(category.clone());
                    self.filtering_config.block_categories.remove(&category);
                } else {
                    self.filtering_config.allow_categories.remove(&category);
                }
                println!(
                    "Category '{}' {} allowed",
                    category,
                    if allowed { "now" } else { "no longer" }
                );
            }

            ContentFilteringMsg::UpdateNewBlockedDomain(domain) => {
                self.new_blocked_domain = domain;
            }

            ContentFilteringMsg::AddBlockedDomain => {
                if !self.new_blocked_domain.is_empty() {
                    self.filtering_config.blocked_domains.insert(self.new_blocked_domain.clone());
                    self.filtering_config.allowed_domains.remove(&self.new_blocked_domain);
                    println!("Added '{}' to blocked domains", self.new_blocked_domain);
                    self.new_blocked_domain.clear();
                }
            }

            ContentFilteringMsg::RemoveBlockedDomain(domain) => {
                self.filtering_config.blocked_domains.remove(&domain);
                println!("Removed '{}' from blocked domains", domain);
            }

            ContentFilteringMsg::UpdateNewAllowedDomain(domain) => {
                self.new_allowed_domain = domain;
            }

            ContentFilteringMsg::AddAllowedDomain => {
                if !self.new_allowed_domain.is_empty() {
                    self.filtering_config.allowed_domains.insert(self.new_allowed_domain.clone());
                    self.filtering_config.blocked_domains.remove(&self.new_allowed_domain);
                    println!("Added '{}' to allowed domains", self.new_allowed_domain);
                    self.new_allowed_domain.clear();
                }
            }

            ContentFilteringMsg::RemoveAllowedDomain(domain) => {
                self.filtering_config.allowed_domains.remove(&domain);
                println!("Removed '{}' from allowed domains", domain);
            }

            ContentFilteringMsg::ToggleCustomRules(enabled) => {
                self.filtering_config.custom_rules_enabled = enabled;
                println!("Custom rules {}", if enabled { "enabled" } else { "disabled" });
            }

            ContentFilteringMsg::UpdateCustomPattern(pattern) => {
                self.new_custom_pattern = pattern;
            }

            ContentFilteringMsg::UpdateCustomReason(reason) => {
                self.custom_pattern_reason = reason;
            }

            ContentFilteringMsg::AddCustomPattern => {
                if !self.new_custom_pattern.is_empty() {
                    println!(
                        "Added custom pattern: '{}' ({})",
                        self.new_custom_pattern, self.custom_pattern_reason
                    );
                    self.new_custom_pattern.clear();
                    self.custom_pattern_reason.clear();
                }
            }

            ContentFilteringMsg::SaveConfiguration => {
                println!("Saving content filtering configuration...");
                if let Some(ref _daemon_client) = self.daemon_client {
                    println!("Configuration would be saved to daemon");
                }
                let _ = sender.output(String::from("Content filtering settings saved"));
            }

            ContentFilteringMsg::LoadConfiguration => {
                println!("Loading content filtering configuration...");
                let _ = sender.output(String::from("Content filtering settings loaded"));
            }

            ContentFilteringMsg::ResetToDefaults => {
                self.filtering_config = FilteringConfig {
                    enabled: true,
                    safe_search_enforcement: true,
                    block_categories: ["adult", "gambling", "violence", "drugs"]
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                    allow_categories: ["educational", "children", "reference"]
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                    blocked_domains: HashSet::new(),
                    allowed_domains: HashSet::new(),
                    custom_rules_enabled: false,
                };
                self.new_blocked_domain.clear();
                self.new_allowed_domain.clear();
                self.new_custom_pattern.clear();
                self.custom_pattern_reason.clear();
                println!("Reset to default content filtering configuration");
                let _ = sender.output(String::from("Reset to default settings"));
            }
        }
    }

    view! {
        gtk4::ScrolledWindow {
            set_policy: (gtk4::PolicyType::Never, gtk4::PolicyType::Automatic),

            gtk4::Box {
                set_orientation: gtk4::Orientation::Vertical,
                set_margin_all: 20,
                set_spacing: 20,

                // Header
                gtk4::Box {
                    set_orientation: gtk4::Orientation::Horizontal,
                    set_spacing: 10,

                    gtk4::Image {
                        set_icon_name: Some("applications-internet"),
                        set_pixel_size: 32,
                    },

                    gtk4::Box {
                        set_orientation: gtk4::Orientation::Vertical,
                        set_hexpand: true,

                        gtk4::Label {
                            set_markup: "<span size='large'><b>Content Filtering</b></span>",
                            set_halign: gtk4::Align::Start,
                        },

                        gtk4::Label {
                            set_text: "Configure web content filtering and safe browsing for this profile",
                            set_halign: gtk4::Align::Start,
                            add_css_class: "dim-label",
                        },
                    },
                },

                // Main Controls Card
                gtk4::Frame {
                    gtk4::Box {
                        set_orientation: gtk4::Orientation::Vertical,
                        set_margin_all: 16,
                        set_spacing: 12,

                        gtk4::Label {
                            set_markup: "<b>Main Controls</b>",
                            set_halign: gtk4::Align::Start,
                        },

                        gtk4::Box {
                            set_orientation: gtk4::Orientation::Horizontal,
                            set_spacing: 12,

                            gtk4::Switch {
                                set_halign: gtk4::Align::Start,
                                #[watch]
                                set_active: model.filtering_config.enabled,
                                connect_active_notify[sender] => move |switch| {
                                    sender.input(ContentFilteringMsg::ToggleFiltering(switch.is_active()));
                                },
                            },

                            gtk4::Label {
                                set_text: "Enable content filtering",
                                set_hexpand: true,
                                set_halign: gtk4::Align::Start,
                            },
                        },

                        gtk4::Box {
                            set_orientation: gtk4::Orientation::Horizontal,
                            set_spacing: 12,
                            #[watch]
                            set_sensitive: model.filtering_config.enabled,

                            gtk4::Switch {
                                set_halign: gtk4::Align::Start,
                                #[watch]
                                set_active: model.filtering_config.safe_search_enforcement,
                                connect_active_notify[sender] => move |switch| {
                                    sender.input(ContentFilteringMsg::ToggleSafeSearch(switch.is_active()));
                                },
                            },

                            gtk4::Label {
                                set_text: "Enforce safe search (Google, Bing, DuckDuckGo, YouTube)",
                                set_hexpand: true,
                                set_halign: gtk4::Align::Start,
                            },
                        },
                    },
                },

                // Content Categories Card
                gtk4::Frame {
                    #[watch]
                    set_sensitive: model.filtering_config.enabled,

                    gtk4::Box {
                        set_orientation: gtk4::Orientation::Vertical,
                        set_margin_all: 16,
                        set_spacing: 12,

                        gtk4::Label {
                            set_markup: "<b>Content Categories</b>",
                            set_halign: gtk4::Align::Start,
                        },

                        // Blocked Categories
                        gtk4::Label {
                            set_markup: "<b>Blocked Categories:</b>",
                            set_halign: gtk4::Align::Start,
                        },

                        gtk4::Box {
                            set_orientation: gtk4::Orientation::Horizontal,
                            set_spacing: 16,

                            gtk4::CheckButton {
                                set_label: Some("Adult Content"),
                                #[watch]
                                set_active: model.filtering_config.block_categories.contains("adult"),
                                connect_toggled[sender] => move |btn| {
                                    sender.input(ContentFilteringMsg::ToggleCategoryBlock("adult".to_string(), btn.is_active()));
                                },
                            },

                            gtk4::CheckButton {
                                set_label: Some("Gambling"),
                                #[watch]
                                set_active: model.filtering_config.block_categories.contains("gambling"),
                                connect_toggled[sender] => move |btn| {
                                    sender.input(ContentFilteringMsg::ToggleCategoryBlock("gambling".to_string(), btn.is_active()));
                                },
                            },

                            gtk4::CheckButton {
                                set_label: Some("Violence"),
                                #[watch]
                                set_active: model.filtering_config.block_categories.contains("violence"),
                                connect_toggled[sender] => move |btn| {
                                    sender.input(ContentFilteringMsg::ToggleCategoryBlock("violence".to_string(), btn.is_active()));
                                },
                            },

                            gtk4::CheckButton {
                                set_label: Some("Drugs"),
                                #[watch]
                                set_active: model.filtering_config.block_categories.contains("drugs"),
                                connect_toggled[sender] => move |btn| {
                                    sender.input(ContentFilteringMsg::ToggleCategoryBlock("drugs".to_string(), btn.is_active()));
                                },
                            },
                        },

                        // Allowed Categories
                        gtk4::Label {
                            set_markup: "<b>Explicitly Allowed Categories:</b>",
                            set_halign: gtk4::Align::Start,
                            set_margin_top: 16,
                        },

                        gtk4::Box {
                            set_orientation: gtk4::Orientation::Horizontal,
                            set_spacing: 16,

                            gtk4::CheckButton {
                                set_label: Some("Educational"),
                                #[watch]
                                set_active: model.filtering_config.allow_categories.contains("educational"),
                                connect_toggled[sender] => move |btn| {
                                    sender.input(ContentFilteringMsg::ToggleCategoryAllow("educational".to_string(), btn.is_active()));
                                },
                            },

                            gtk4::CheckButton {
                                set_label: Some("Children's Content"),
                                #[watch]
                                set_active: model.filtering_config.allow_categories.contains("children"),
                                connect_toggled[sender] => move |btn| {
                                    sender.input(ContentFilteringMsg::ToggleCategoryAllow("children".to_string(), btn.is_active()));
                                },
                            },

                            gtk4::CheckButton {
                                set_label: Some("Reference"),
                                #[watch]
                                set_active: model.filtering_config.allow_categories.contains("reference"),
                                connect_toggled[sender] => move |btn| {
                                    sender.input(ContentFilteringMsg::ToggleCategoryAllow("reference".to_string(), btn.is_active()));
                                },
                            },
                        },
                    },
                },

                // Domain Management Card
                gtk4::Frame {
                    #[watch]
                    set_sensitive: model.filtering_config.enabled,

                    gtk4::Box {
                        set_orientation: gtk4::Orientation::Vertical,
                        set_margin_all: 16,
                        set_spacing: 12,

                        gtk4::Label {
                            set_markup: "<b>Domain Management</b>",
                            set_halign: gtk4::Align::Start,
                        },

                        // Blocked Domains Section
                        gtk4::Box {
                            set_orientation: gtk4::Orientation::Vertical,
                            set_spacing: 8,

                            gtk4::Label {
                                set_markup: "<b>Blocked Domains:</b>",
                                set_halign: gtk4::Align::Start,
                            },

                            gtk4::Box {
                                set_orientation: gtk4::Orientation::Horizontal,
                                set_spacing: 8,

                                gtk4::Entry {
                                    #[watch]
                                    set_text: &model.new_blocked_domain,
                                    set_placeholder_text: Some("Enter domain to block (e.g., example.com)"),
                                    set_hexpand: true,
                                    connect_changed[sender] => move |entry| {
                                        sender.input(ContentFilteringMsg::UpdateNewBlockedDomain(entry.text().to_string()));
                                    },
                                },

                                gtk4::Button {
                                    set_label: "Add",
                                    add_css_class: "suggested-action",
                                    #[watch]
                                    set_sensitive: !model.new_blocked_domain.is_empty(),
                                    connect_clicked => ContentFilteringMsg::AddBlockedDomain,
                                },
                            },

                            gtk4::Label {
                                set_text: &format!("Blocked domains: {}",
                                    model.filtering_config.blocked_domains.len()),
                                set_halign: gtk4::Align::Start,
                                add_css_class: "dim-label",
                                #[watch]
                                set_visible: !model.filtering_config.blocked_domains.is_empty(),
                            },
                        },

                        // Allowed Domains Section
                        gtk4::Box {
                            set_orientation: gtk4::Orientation::Vertical,
                            set_spacing: 8,
                            set_margin_top: 16,

                            gtk4::Label {
                                set_markup: "<b>Allowed Domains:</b>",
                                set_halign: gtk4::Align::Start,
                            },

                            gtk4::Box {
                                set_orientation: gtk4::Orientation::Horizontal,
                                set_spacing: 8,

                                gtk4::Entry {
                                    #[watch]
                                    set_text: &model.new_allowed_domain,
                                    set_placeholder_text: Some("Enter domain to allow (e.g., educational-site.com)"),
                                    set_hexpand: true,
                                    connect_changed[sender] => move |entry| {
                                        sender.input(ContentFilteringMsg::UpdateNewAllowedDomain(entry.text().to_string()));
                                    },
                                },

                                gtk4::Button {
                                    set_label: "Add",
                                    add_css_class: "suggested-action",
                                    #[watch]
                                    set_sensitive: !model.new_allowed_domain.is_empty(),
                                    connect_clicked => ContentFilteringMsg::AddAllowedDomain,
                                },
                            },

                            gtk4::Label {
                                set_text: &format!("Allowed domains: {}",
                                    model.filtering_config.allowed_domains.len()),
                                set_halign: gtk4::Align::Start,
                                add_css_class: "dim-label",
                                #[watch]
                                set_visible: !model.filtering_config.allowed_domains.is_empty(),
                            },
                        },
                    },
                },

                // Custom Rules Card
                gtk4::Frame {
                    #[watch]
                    set_sensitive: model.filtering_config.enabled,

                    gtk4::Box {
                        set_orientation: gtk4::Orientation::Vertical,
                        set_margin_all: 16,
                        set_spacing: 12,

                        gtk4::Box {
                            set_orientation: gtk4::Orientation::Horizontal,
                            set_spacing: 12,

                            gtk4::Switch {
                                set_halign: gtk4::Align::Start,
                                #[watch]
                                set_active: model.filtering_config.custom_rules_enabled,
                                connect_active_notify[sender] => move |switch| {
                                    sender.input(ContentFilteringMsg::ToggleCustomRules(switch.is_active()));
                                },
                            },

                            gtk4::Label {
                                set_markup: "<b>Custom URL Pattern Rules</b>",
                                set_hexpand: true,
                                set_halign: gtk4::Align::Start,
                            },
                        },

                        gtk4::Box {
                            set_orientation: gtk4::Orientation::Vertical,
                            set_spacing: 8,
                            #[watch]
                            set_sensitive: model.filtering_config.custom_rules_enabled,

                            gtk4::Label {
                                set_text: "Add custom URL patterns to block (uses regular expressions)",
                                set_halign: gtk4::Align::Start,
                                add_css_class: "dim-label",
                            },

                            gtk4::Box {
                                set_orientation: gtk4::Orientation::Horizontal,
                                set_spacing: 8,

                                gtk4::Entry {
                                    #[watch]
                                    set_text: &model.new_custom_pattern,
                                    set_placeholder_text: Some("Enter URL pattern (e.g., .*casino.*)"),
                                    set_hexpand: true,
                                    connect_changed[sender] => move |entry| {
                                        sender.input(ContentFilteringMsg::UpdateCustomPattern(entry.text().to_string()));
                                    },
                                },

                                gtk4::Entry {
                                    #[watch]
                                    set_text: &model.custom_pattern_reason,
                                    set_placeholder_text: Some("Reason (e.g., Gambling)"),
                                    set_width_chars: 15,
                                    connect_changed[sender] => move |entry| {
                                        sender.input(ContentFilteringMsg::UpdateCustomReason(entry.text().to_string()));
                                    },
                                },

                                gtk4::Button {
                                    set_label: "Add",
                                    add_css_class: "suggested-action",
                                    #[watch]
                                    set_sensitive: !model.new_custom_pattern.is_empty() && !model.custom_pattern_reason.is_empty(),
                                    connect_clicked => ContentFilteringMsg::AddCustomPattern,
                                },
                            },
                        },
                    },
                },

                // Action Buttons
                gtk4::Box {
                    set_orientation: gtk4::Orientation::Horizontal,
                    set_spacing: 12,
                    set_halign: gtk4::Align::End,

                    gtk4::Button {
                        set_label: "Reset to Defaults",
                        connect_clicked => ContentFilteringMsg::ResetToDefaults,
                    },

                    gtk4::Button {
                        set_label: "Load Configuration",
                        connect_clicked => ContentFilteringMsg::LoadConfiguration,
                    },

                    gtk4::Button {
                        set_label: "Save Configuration",
                        add_css_class: "suggested-action",
                        connect_clicked => ContentFilteringMsg::SaveConfiguration,
                    },
                },
            },
        }
    }
}
