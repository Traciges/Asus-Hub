use gtk4 as gtk;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;

use crate::services::config::AppConfig;

// ──────────────────────────────────────────────────────────────────────────────
// Ruhezustand der Tastaturhintergrundbeleuchtung
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) enum TimeoutModus {
    #[default]
    Nichts,
    AkkuUndNetz,
    NurAkku,
}

impl From<u32> for TimeoutModus {
    fn from(v: u32) -> Self {
        match v {
            1 => Self::AkkuUndNetz,
            2 => Self::NurAkku,
            _ => Self::Nichts,
        }
    }
}

const TIMEOUT_SEKUNDEN: [u32; 3] = [60, 120, 300];

fn busctl_brightness_cmd(wert: i32, nur_akku: bool) -> String {
    let base = format!(
        "busctl call --system org.freedesktop.UPower \
         /org/freedesktop/UPower/KbdBacklight \
         org.freedesktop.UPower.KbdBacklight SetBrightness i {wert}"
    );
    if nur_akku {
        format!(
            "if [ \"$(cat /sys/class/power_supply/*/online | head -n1)\" = \"0\" ]; \
             then {base}; fi"
        )
    } else {
        base
    }
}

pub struct RuhezustandModel {
    timeout_modus: TimeoutModus,
    check_nichts: gtk::CheckButton,
    check_akku_netz: gtk::CheckButton,
    check_nur_akku: gtk::CheckButton,
    dropdown_akku_netz: gtk::DropDown,
    dropdown_nur_akku: gtk::DropDown,
    swayidle_task: Option<tokio::task::JoinHandle<()>>,
}

#[derive(Debug)]
pub enum RuhezustandMsg {
    ModusWechseln(TimeoutModus),
    AkkuNetzZeitGeaendert(u32),
    NurAkkuZeitGeaendert(u32),
}

#[derive(Debug)]
pub enum RuhezustandCommandOutput {
    Fehler(String),
}

#[relm4::component(pub)]
impl Component for RuhezustandModel {
    type Init = ();
    type Input = RuhezustandMsg;
    type Output = String;
    type CommandOutput = RuhezustandCommandOutput;

    view! {
        adw::PreferencesGroup {
            set_title: &t!("sleep_group_title"),
            set_description: Some(&t!("sleep_group_desc")),

            add = &adw::ActionRow {
                set_title: &t!("sleep_mode_never_title"),
                set_subtitle: &t!("sleep_mode_never_subtitle"),
                add_prefix = &model.check_nichts.clone(),
                set_activatable_widget: Some(&model.check_nichts),
            },

            add = &adw::ActionRow {
                set_title: &t!("sleep_mode_always_title"),
                add_prefix = &model.check_akku_netz.clone(),
                set_activatable_widget: Some(&model.check_akku_netz),
                add_suffix = &model.dropdown_akku_netz.clone() -> gtk::DropDown {
                    set_valign: gtk::Align::Center,
                    #[watch]
                    set_sensitive: model.timeout_modus == TimeoutModus::AkkuUndNetz,
                },
            },

            add = &adw::ActionRow {
                set_title: &t!("sleep_mode_battery_title"),
                add_prefix = &model.check_nur_akku.clone(),
                set_activatable_widget: Some(&model.check_nur_akku),
                add_suffix = &model.dropdown_nur_akku.clone() -> gtk::DropDown {
                    set_valign: gtk::Align::Center,
                    #[watch]
                    set_sensitive: model.timeout_modus == TimeoutModus::NurAkku,
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let config = AppConfig::load();
        let modus = TimeoutModus::from(config.kbd_timeout_modus);

        let check_nichts = gtk::CheckButton::new();
        let check_akku_netz = gtk::CheckButton::new();
        let check_nur_akku = gtk::CheckButton::new();
        check_akku_netz.set_group(Some(&check_nichts));
        check_nur_akku.set_group(Some(&check_nichts));

        match modus {
            TimeoutModus::Nichts => check_nichts.set_active(true),
            TimeoutModus::AkkuUndNetz => check_akku_netz.set_active(true),
            TimeoutModus::NurAkku => check_nur_akku.set_active(true),
        }

        let t1 = t!("sleep_timeout_1min");
        let t2 = t!("sleep_timeout_2min");
        let t5 = t!("sleep_timeout_5min");
        let zeitoptionen = gtk::StringList::new(&[&t1, &t2, &t5]);
        let dropdown_akku_netz =
            gtk::DropDown::new(Some(zeitoptionen.clone()), gtk::Expression::NONE);
        let dropdown_nur_akku = gtk::DropDown::new(Some(zeitoptionen), gtk::Expression::NONE);
        dropdown_akku_netz.set_selected(config.kbd_timeout_akku_netz_index);
        dropdown_nur_akku.set_selected(config.kbd_timeout_nur_akku_index);

        for (btn, modus_val) in [
            (&check_nichts, TimeoutModus::Nichts),
            (&check_akku_netz, TimeoutModus::AkkuUndNetz),
            (&check_nur_akku, TimeoutModus::NurAkku),
        ] {
            let sender = sender.clone();
            btn.connect_toggled(move |b| {
                if b.is_active() {
                    sender.input(RuhezustandMsg::ModusWechseln(modus_val));
                }
            });
        }

        {
            let sender = sender.clone();
            dropdown_akku_netz.connect_selected_notify(move |dd| {
                sender.input(RuhezustandMsg::AkkuNetzZeitGeaendert(dd.selected()));
            });
        }
        {
            let sender = sender.clone();
            dropdown_nur_akku.connect_selected_notify(move |dd| {
                sender.input(RuhezustandMsg::NurAkkuZeitGeaendert(dd.selected()));
            });
        }

        let mut model = RuhezustandModel {
            timeout_modus: modus,
            check_nichts,
            check_akku_netz,
            check_nur_akku,
            dropdown_akku_netz,
            dropdown_nur_akku,
            swayidle_task: None,
        };

        let widgets = view_output!();
        model.timeout_schreiben(modus, &sender);
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: RuhezustandMsg, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            RuhezustandMsg::ModusWechseln(modus) => {
                self.timeout_modus = modus;
                AppConfig::update(|c| c.kbd_timeout_modus = modus as u32);
                self.timeout_schreiben(modus, &sender);
            }
            RuhezustandMsg::AkkuNetzZeitGeaendert(index) => {
                AppConfig::update(|c| c.kbd_timeout_akku_netz_index = index);
                if self.timeout_modus == TimeoutModus::AkkuUndNetz {
                    self.timeout_schreiben(TimeoutModus::AkkuUndNetz, &sender);
                }
            }
            RuhezustandMsg::NurAkkuZeitGeaendert(index) => {
                AppConfig::update(|c| c.kbd_timeout_nur_akku_index = index);
                if self.timeout_modus == TimeoutModus::NurAkku {
                    self.timeout_schreiben(TimeoutModus::NurAkku, &sender);
                }
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: RuhezustandCommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            RuhezustandCommandOutput::Fehler(e) => {
                let _ = sender.output(e);
            }
        }
    }
}

impl RuhezustandModel {
    fn timeout_schreiben(
        &mut self,
        modus: TimeoutModus,
        sender: &ComponentSender<RuhezustandModel>,
    ) {
        if let Some(task) = self.swayidle_task.take() {
            task.abort();
        }

        if modus == TimeoutModus::Nichts {
            return;
        }

        let sekunden = match modus {
            TimeoutModus::Nichts => unreachable!(),
            TimeoutModus::AkkuUndNetz => {
                let idx = self.dropdown_akku_netz.selected() as usize;
                *TIMEOUT_SEKUNDEN.get(idx).unwrap_or(&60)
            }
            TimeoutModus::NurAkku => {
                let idx = self.dropdown_nur_akku.selected() as usize;
                *TIMEOUT_SEKUNDEN.get(idx).unwrap_or(&60)
            }
        };

        let nur_akku = modus == TimeoutModus::NurAkku;
        let timeout_cmd = busctl_brightness_cmd(0, nur_akku);
        let resume_cmd = busctl_brightness_cmd(3, nur_akku);
        let sekunden_str = sekunden.to_string();

        let cmd_sender = sender.command_sender().clone();
        let handle = tokio::spawn(async move {
            let mut child = match tokio::process::Command::new("swayidle")
                .kill_on_drop(true)
                .args([
                    "-w",
                    "timeout",
                    &sekunden_str,
                    &timeout_cmd,
                    "resume",
                    &resume_cmd,
                ])
                .spawn()
            {
                Ok(c) => c,
                Err(e) => {
                    cmd_sender.emit(RuhezustandCommandOutput::Fehler(
                        t!("error_swayidle_start", error = e.to_string()).to_string(),
                    ));
                    return;
                }
            };
            if let Err(e) = child.wait().await {
                cmd_sender.emit(RuhezustandCommandOutput::Fehler(
                    t!("error_swayidle_wait", error = e.to_string()).to_string(),
                ));
            }
        });

        self.swayidle_task = Some(handle);
    }
}
