use crate::app::AppMsg;
use gtk4::prelude::ApplicationExt;
use ksni::menu::StandardItem;
use ksni::{Icon, MenuItem, Tray};
use relm4::Sender;

pub struct ZenbookTray {
    pub app_sender: Sender<AppMsg>,
}

impl Tray for ZenbookTray {
    fn id(&self) -> String {
        "ZenbookControl".into()
    }

    fn title(&self) -> String {
        "Zenbook Control".into()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        // 16x16 solid blue square, ARGB32 big-endian: [A, R, G, B]
        let pixel: [u8; 4] = [0xFF, 0x00, 0x78, 0xD7];
        let data: Vec<u8> = pixel.iter().cloned().cycle().take(16 * 16 * 4).collect();
        vec![Icon {
            width: 16,
            height: 16,
            data,
        }]
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let sender_show = self.app_sender.clone();
        vec![
            MenuItem::Standard(StandardItem {
                label: "Anzeigen".into(),
                activate: Box::new(move |_| {
                    sender_show.emit(AppMsg::ShowWindow);
                }),
                ..Default::default()
            }),
            MenuItem::Separator,
            MenuItem::Standard(StandardItem {
                label: "Beenden".into(),
                activate: Box::new(|_| {
                    relm4::main_application().quit();
                }),
                ..Default::default()
            }),
        ]
    }
}
