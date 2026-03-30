mod backend;
mod components;
mod services;

use components::battery::BatteryModel;
use components::display::FarbskalaModel;
use components::display::OledCareModel;
use components::display::ZielmodusModel;
use components::fan::FanModel;
use components::input::FnKeyModel;
use components::input::GesturenModel;
use components::keyboard::AutoBeleuchtungModel;
use components::keyboard::RuhezustandModel;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;

struct AppModel {
    battery: Controller<BatteryModel>,
    fan: Controller<FanModel>,
    oled_care: Controller<OledCareModel>,
    farbskala: Controller<FarbskalaModel>,
    zielmodus: Controller<ZielmodusModel>,
    fn_key: Controller<FnKeyModel>,
    gesten: Controller<GesturenModel>,
    auto_beleuchtung: Controller<AutoBeleuchtungModel>,
    ruhezustand: Controller<RuhezustandModel>,
}

#[relm4::component]
impl SimpleComponent for AppModel {
    type Init = ();
    type Input = ();
    type Output = ();

    view! {
        adw::ApplicationWindow {
            set_title: Some("Zenbook Control Center"),
            set_default_size: (1200, 800),

            #[wrap(Some)]
            set_content = &adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {},

                #[wrap(Some)]
                set_content = &adw::PreferencesPage {
                    #[local_ref]
                    add = battery_widget -> adw::PreferencesGroup {},
                    #[local_ref]
                    add = fan_widget -> adw::PreferencesGroup {},
                    #[local_ref]
                    add = oled_care_widget -> adw::PreferencesGroup {},
                    #[local_ref]
                    add = farbskala_widget -> adw::PreferencesGroup {},
                    #[local_ref]
                    add = zielmodus_widget -> adw::PreferencesGroup {},
                    #[local_ref]
                    add = fn_key_widget -> adw::PreferencesGroup {},
                    #[local_ref]
                    add = gesten_widget -> adw::PreferencesGroup {},
                    #[local_ref]
                    add = auto_beleuchtung_widget -> adw::PreferencesGroup {},
                    #[local_ref]
                    add = ruhezustand_widget -> adw::PreferencesGroup {},
                },
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let battery = BatteryModel::builder().launch(()).detach();
        let fan = FanModel::builder().launch(()).detach();
        let oled_care = OledCareModel::builder().launch(()).detach();
        let farbskala = FarbskalaModel::builder().launch(()).detach();
        let zielmodus = ZielmodusModel::builder().launch(()).detach();
        let fn_key = FnKeyModel::builder().launch(()).detach();
        let gesten = GesturenModel::builder().launch(()).detach();
        let auto_beleuchtung = AutoBeleuchtungModel::builder().launch(()).detach();
        let ruhezustand = RuhezustandModel::builder().launch(()).detach();

        let model = AppModel {
            battery,
            fan,
            oled_care,
            farbskala,
            zielmodus,
            fn_key,
            gesten,
            auto_beleuchtung,
            ruhezustand,
        };
        let battery_widget = model.battery.widget();
        let fan_widget = model.fan.widget();
        let oled_care_widget = model.oled_care.widget();
        let farbskala_widget = model.farbskala.widget();
        let zielmodus_widget = model.zielmodus.widget();
        let fn_key_widget = model.fn_key.widget();
        let gesten_widget = model.gesten.widget();
        let auto_beleuchtung_widget = model.auto_beleuchtung.widget();
        let ruhezustand_widget = model.ruhezustand.widget();
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }
}

fn main() {
    let app = RelmApp::new("de.guido.zenbook-control");
    adw::StyleManager::default().set_color_scheme(adw::ColorScheme::PreferDark);
    app.run::<AppModel>(());
}
