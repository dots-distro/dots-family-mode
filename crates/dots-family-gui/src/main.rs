use dots_family_gui::AppModel;
use relm4::RelmApp;

fn main() {
    let app = RelmApp::new("org.dots.family_mode");
    app.run::<AppModel>(());
}
