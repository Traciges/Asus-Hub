mod app;
mod components;
mod services;
mod tray;

fn main() {
    let a = relm4::RelmApp::new("de.guido.zenbook-control");
    relm4::adw::StyleManager::default().set_color_scheme(relm4::adw::ColorScheme::PreferDark);
    a.run::<app::AppModel>(());
}
