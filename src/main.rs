use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;

struct AppModel;

#[relm4::component]
impl SimpleComponent for AppModel {
    type Init = ();
    type Input = ();
    type Output = ();

    view! {
      adw::ApplicationWindow {
        set_title: Some("Zenbook Control"),
        set_default_size: (1200, 800),

        #[wrap(Some)]
        set_content = &adw::ToolbarView {
          add_top_bar = &adw::HeaderBar {},

                #[wrap(Some)]
                set_content = &gtk::Label {
                    set_label: "UI Design Placeholder",
                    set_halign: gtk::Align::Center,
                    set_valign: gtk::Align::Center,
                    set_vexpand: true,
                },
            }
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = AppModel;
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }
}

fn main() {
    let app = RelmApp::new("de.guido.myasus-linux");
    adw::StyleManager::default().set_color_scheme(adw::ColorScheme::PreferDark);
    app.run::<AppModel>(());
}
