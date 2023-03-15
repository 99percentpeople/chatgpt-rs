mod model_table;
mod parameter_control;
use std::{
    sync::{atomic, Arc},
    time::Duration,
};

use crate::api::chat::{ChatGPT, Role};
use eframe::egui;

use self::{model_table::ModelTable, parameter_control::ParameterControl};
use egui_notify::Toasts;
pub struct ChatApp {
    chat: ChatGPT,
    text: String,
    model_table: model_table::ModelTable,
    parameter_control: parameter_control::ParameterControl,
    is_ready: Arc<atomic::AtomicBool>,
    toasts: Toasts,
}
impl ChatApp {
    pub fn new_with_chat(cc: &eframe::CreationContext, chat: ChatGPT) -> Self {
        setup_fonts(&cc.egui_ctx);
        let mut model_table = ModelTable::default();
        {
            let chat = chat.clone();
            model_table.on_select_model(move |model| {
                let mut chat = chat.clone();
                tokio::spawn(async move {
                    chat.set_model(model).await;
                });
            });
        }
        let mut parameter_control = ParameterControl::default();
        {
            let chat = chat.clone();
            parameter_control.on_max_tokens_changed(move |max_tokens| {
                let mut chat = chat.clone();
                tokio::spawn(async move {
                    chat.set_max_tokens(max_tokens).await;
                });
            });
        }
        {
            let chat = chat.clone();
            parameter_control.on_temperature_changed(move |temperature| {
                let mut chat = chat.clone();
                tokio::spawn(async move {
                    chat.set_temperature(temperature).await;
                });
            });
        }
        {
            let chat = chat.clone();
            parameter_control.on_top_p_changed(move |top_p| {
                let mut chat = chat.clone();
                tokio::spawn(async move {
                    chat.set_top_p(top_p).await;
                });
            });
        }
        {
            let chat = chat.clone();
            parameter_control.on_presence_penalty_changed(move |presence_penalty| {
                let mut chat = chat.clone();
                tokio::spawn(async move {
                    chat.set_presence_penalty(presence_penalty).await;
                });
            });
        }
        {
            let chat = chat.clone();
            parameter_control.on_frequency_penalty_changed(move |frequency_penalty| {
                let mut chat = chat.clone();
                tokio::spawn(async move {
                    chat.set_frequency_penalty(frequency_penalty).await;
                });
            });
        }
        Self {
            chat,
            text: String::new(),
            toasts: Toasts::default(),
            model_table,
            is_ready: Arc::new(atomic::AtomicBool::new(true)),
            parameter_control,
        }
    }
}
impl eframe::App for ChatApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let (chat, generate) = tokio::task::block_in_place(|| {
            (
                self.chat.chat.blocking_read().clone(),
                self.chat.pending_generate.blocking_read().clone(),
            )
        });

        let is_ready = self.is_ready.load(atomic::Ordering::Relaxed);

        egui::SidePanel::left("left_panel").show(ctx, |ui| {
            self.model_table.ui(ui);
        });
        egui::SidePanel::right("right_panel").show(ctx, |ui| {
            self.parameter_control.ui(ui);
        });
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.heading(chat.model);
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::RIGHT), |ui| {
                ui.add_enabled_ui(is_ready, |ui| {
                    ui.add(egui::TextEdit::multiline(&mut self.text).desired_width(f32::INFINITY));
                    ui.add_space(5.);
                    ui.horizontal(|ui| {
                        if ui
                            .add_sized(egui::vec2(50., 40.), egui::Button::new("Send"))
                            .clicked()
                        {
                            let input_text = self.text.trim().to_string();
                            if !input_text.is_empty() {
                                let mut chat = self.chat.clone();
                                let is_ready = self.is_ready.clone();
                                tokio::spawn(async move {
                                    is_ready.store(false, atomic::Ordering::Relaxed);
                                    if let Err(e) = chat.question(input_text).await {
                                        println!("Error sending message: {}", e);
                                    }
                                    is_ready.store(true, atomic::Ordering::Relaxed);
                                });
                                self.text.clear();
                            }
                        }
                        if ui
                            .add_sized(egui::vec2(50., 40.), egui::Button::new("Clear"))
                            .clicked()
                        {
                            let mut chat = self.chat.clone();
                            tokio::spawn(async move {
                                chat.clear_message().await;
                            });
                        }
                    });
                });
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.vertical(|ui| {
                    for message in chat.messages {
                        message_container(
                            ui,
                            |ui| {
                                let content = message.content.to_string();
                                ui.add(
                                    egui::Label::new(egui::RichText::new(&content))
                                        .sense(egui::Sense::click()),
                                )
                                .clicked()
                                .then(|| {
                                    ui.output_mut(|o| o.copied_text = content);
                                    self.toasts
                                        .success("Copied")
                                        .set_closable(false)
                                        .set_duration(Some(Duration::from_secs(1)));
                                });
                            },
                            &message.role,
                        );
                    }
                    if let Some(generate) = generate.as_ref() {
                        {
                            message_container(
                                ui,
                                |ui| {
                                    if let Some(content) = generate.content.as_ref() {
                                        ui.label(content)
                                    } else {
                                        ui.spinner()
                                    };
                                },
                                &Role::Assistant,
                            );
                        }

                        ctx.request_repaint();
                    }
                });
            });
        });
        self.toasts.show(ctx);
    }
}

fn setup_fonts(ctx: &egui::Context) {
    // Start with the default fonts (we will be adding to them rather than replacing them).
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "msyhl".to_owned(),
        egui::FontData::from_static(include_bytes!("c:\\windows\\fonts\\msyhl.ttc")),
    );
    fonts.font_data.insert(
        "seguiemj".to_owned(),
        egui::FontData::from_static(include_bytes!("c:\\windows\\fonts\\seguiemj.ttf")),
    );
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "msyhl".to_owned());
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(1, "seguiemj".to_owned());
    ctx.set_fonts(fonts);
}

pub fn message_container<R>(
    ui: &mut egui::Ui,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
    role: &Role,
) {
    ui.horizontal(|ui| {
        ui.with_layout(
            match role {
                Role::User => egui::Layout::right_to_left(egui::Align::Min).with_main_wrap(true),
                Role::Assistant => {
                    egui::Layout::left_to_right(egui::Align::Min).with_main_wrap(true)
                }
                Role::System => egui::Layout::centered_and_justified(egui::Direction::TopDown)
                    .with_main_wrap(true),
            },
            |ui| {
                ui.group(|ui| {
                    add_contents(ui);
                });
            },
        )
    });
}
