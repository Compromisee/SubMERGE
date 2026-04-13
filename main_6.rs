pub mod animations;
pub mod components;
pub mod icons;
pub mod theme;

use crate::app::{AppMode, AppState, LogType, SubMergeApp};
use egui::{
    Align, CentralPanel, Color32, Context, Layout, Pos2, Rect, RichText, ScrollArea,
    Sense, Stroke, TextEdit, Ui, Vec2,
};

pub fn render_ui(app: &mut SubMergeApp, ctx: &Context) {
    CentralPanel::default()
        .frame(egui::Frame::none().fill(theme::BG_PRIMARY))
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(12.0, 12.0);

            let available_width = ui.available_width();
            let content_width = (available_width - 48.0).min(650.0);
            let margin = (available_width - content_width) / 2.0;

            ui.horizontal(|ui| {
                ui.add_space(margin);
                ui.vertical(|ui| {
                    ui.set_width(content_width);
                    ui.add_space(24.0);

                    render_header(app, ui);
                    ui.add_space(20.0);

                    render_mode_toggle(app, ui);
                    ui.add_space(16.0);

                    render_drop_zone(app, ui, ctx);
                    ui.add_space(16.0);

                    if app.episode_info.is_some() {
                        render_episode_info(app, ui);
                        ui.add_space(16.0);
                    }

                    if app.state != AppState::Idle {
                        render_show_input(app, ui);
                        ui.add_space(16.0);
                    }

                    if !app.subtitles.is_empty() {
                        render_subtitle_list(app, ui);
                        ui.add_space(16.0);
                    }

                    render_action_buttons(app, ui);
                    ui.add_space(16.0);

                    render_log_panel(app, ui);
                    ui.add_space(24.0);
                });
                ui.add_space(margin);
            });
        });
}

fn render_header(app: &SubMergeApp, ui: &mut Ui) {
    ui.horizontal(|ui| {
        // Animated gradient title
        let time = app.animation.time as f32;
        let gradient_offset = (time * 0.5).sin() * 0.5 + 0.5;

        let title_rect = ui.available_rect_before_wrap();
        let painter = ui.painter();

        let text = "SubMerge";
        let font_id = egui::FontId::new(36.0, egui::FontFamily::Name("JetBrains".into()));

        let galley = painter.layout_no_wrap(
            text.to_string(),
            font_id,
            Color32::WHITE,
        );

        let text_pos = title_rect.min + Vec2::new(0.0, 0.0);

        // Draw gradient text character by character
        for (i, c) in text.chars().enumerate() {
            let char_offset = i as f32 / text.len() as f32;
            let t = (char_offset + gradient_offset) % 1.0;
            let color = theme::gradient_color(t);

            let char_font = egui::FontId::new(36.0, egui::FontFamily::Name("JetBrains".into()));
            let char_galley =
                painter.layout_no_wrap(c.to_string(), char_font.clone(), color);

            let x_offset: f32 = text.chars().take(i).map(|_| 22.0).sum();
            painter.galley(text_pos + Vec2::new(x_offset, 0.0), char_galley, color);
        }

        ui.add_space(galley.size().x + 16.0);

        // Animated status indicator
        let status_color = match app.state {
            AppState::Idle => theme::TEXT_DIM,
            AppState::FileSelected => theme::ACCENT_BLUE,
            AppState::Searching | AppState::Downloading | AppState::Merging => {
                let pulse = ((time * 3.0).sin() * 0.5 + 0.5) as u8;
                Color32::from_rgb(100 + pulse * 50, 100 + pulse * 50, 255)
            }
            AppState::SubtitlesFound => theme::ACCENT_GREEN,
            AppState::Complete => theme::ACCENT_GREEN,
            AppState::Error(_) => theme::ACCENT_RED,
        };

        let indicator_rect = Rect::from_min_size(
            ui.cursor().min + Vec2::new(0.0, 14.0),
            Vec2::new(8.0, 8.0),
        );
        painter.circle_filled(indicator_rect.center(), 4.0, status_color);

        ui.add_space(16.0);
        ui.label(
            RichText::new(state_label(&app.state))
                .color(theme::TEXT_SECONDARY)
                .size(14.0),
        );
    });
}

fn state_label(state: &AppState) -> &'static str {
    match state {
        AppState::Idle => "Ready",
        AppState::FileSelected => "File Selected",
        AppState::Searching => "Searching...",
        AppState::SubtitlesFound => "Subtitles Found",
        AppState::Downloading => "Downloading...",
        AppState::Merging => "Merging...",
        AppState::Complete => "Complete",
        AppState::Error(_) => "Error",
    }
}

fn render_mode_toggle(app: &mut SubMergeApp, ui: &mut Ui) {
    let time = app.animation.time as f32;

    ui.horizontal(|ui| {
        // Mode toggle with gradient outline
        let toggle_rect = ui.available_rect_before_wrap();
        let toggle_width = 200.0;
        let toggle_height = 36.0;

        let rect = Rect::from_min_size(
            toggle_rect.min,
            Vec2::new(toggle_width, toggle_height),
        );

        let response = ui.allocate_rect(rect, Sense::click());
        let painter = ui.painter();

        // Animated gradient border
        let gradient_progress = (time * 2.0) % 1.0;
        components::draw_gradient_border(painter, rect, 2.0, gradient_progress);

        // Background
        painter.rect_filled(
            rect.shrink(2.0),
            egui::Rounding::same(8.0),
            theme::BG_SECONDARY,
        );

        // Sliding indicator
        let target_x = if app.mode == AppMode::DryRun {
            rect.left() + 4.0
        } else {
            rect.left() + toggle_width / 2.0
        };

        let indicator_rect = Rect::from_min_size(
            Pos2::new(target_x, rect.top() + 4.0),
            Vec2::new(toggle_width / 2.0 - 6.0, toggle_height - 8.0),
        );

        let indicator_color = if app.mode == AppMode::DryRun {
            theme::ACCENT_YELLOW
        } else {
            theme::ACCENT_GREEN
        };

        painter.rect_filled(
            indicator_rect,
            egui::Rounding::same(6.0),
            indicator_color.gamma_multiply(0.3),
        );

        // Labels
        let dry_run_color = if app.mode == AppMode::DryRun {
            Color32::WHITE
        } else {
            theme::TEXT_DIM
        };
        let real_color = if app.mode == AppMode::Real {
            Color32::WHITE
        } else {
            theme::TEXT_DIM
        };

        painter.text(
            rect.left_center() + Vec2::new(toggle_width / 4.0, 0.0),
            egui::Align2::CENTER_CENTER,
            "DRY RUN",
            egui::FontId::new(12.0, egui::FontFamily::Name("JetBrains".into())),
            dry_run_color,
        );

        painter.text(
            rect.right_center() - Vec2::new(toggle_width / 4.0, 0.0),
            egui::Align2::CENTER_CENTER,
            "REAL",
            egui::FontId::new(12.0, egui::FontFamily::Name("JetBrains".into())),
            real_color,
        );

        if response.clicked() {
            app.mode = match app.mode {
                AppMode::DryRun => AppMode::Real,
                AppMode::Real => AppMode::DryRun,
            };
        }

        // Icons
        ui.add_space(toggle_width + 12.0);

        // Dry run icon
        icons::draw_icon(
            painter,
            icons::IconType::Eye,
            Pos2::new(rect.right() + 20.0, rect.center().y),
            12.0,
            if app.mode == AppMode::DryRun {
                theme::ACCENT_YELLOW
            } else {
                theme::TEXT_DIM
            },
        );

        // Real mode icon
        icons::draw_icon(
            painter,
            icons::IconType::Bolt,
            Pos2::new(rect.right() + 44.0, rect.center().y),
            12.0,
            if app.mode == AppMode::Real {
                theme::ACCENT_GREEN
            } else {
                theme::TEXT_DIM
            },
        );
    });
}

fn render_drop_zone(app: &mut SubMergeApp, ui: &mut Ui, ctx: &Context) {
    let time = app.animation.time as f32;
    let zone_height = if app.file_path.is_some() { 80.0 } else { 140.0 };

    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), zone_height),
        Sense::click(),
    );

    let painter = ui.painter();
    let is_hovered = response.hovered();

    // Animated dashed border
    let dash_offset = (time * 50.0) % 20.0;
    let border_color = if is_hovered {
        theme::ACCENT_BLUE
    } else {
        theme::BORDER
    };

    components::draw_dashed_rect(painter, rect, 2.0, border_color, dash_offset);

    // Background
    let bg_color = if is_hovered {
        theme::BG_TERTIARY
    } else {
        theme::BG_SECONDARY
    };
    painter.rect_filled(rect.shrink(2.0), egui::Rounding::same(12.0), bg_color);

    if let Some(ref path) = app.file_path {
        // Show selected file
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown");

        icons::draw_icon(
            painter,
            icons::IconType::File,
            rect.left_center() + Vec2::new(24.0, 0.0),
            20.0,
            theme::ACCENT_BLUE,
        );

        painter.text(
            rect.left_center() + Vec2::new(52.0, 0.0),
            egui::Align2::LEFT_CENTER,
            filename,
            egui::FontId::new(14.0, egui::FontFamily::Name("JetBrains".into())),
            theme::TEXT_PRIMARY,
        );

        // Clear button
        let clear_rect = Rect::from_center_size(
            rect.right_center() - Vec2::new(24.0, 0.0),
            Vec2::new(20.0, 20.0),
        );

        if ui
            .put(
                clear_rect,
                egui::Button::new(
                    RichText::new("×")
                        .size(16.0)
                        .color(theme::TEXT_DIM),
                )
                .frame(false),
            )
            .clicked()
        {
            app.reset();
        }
    } else {
        // Drop zone prompt
        let icon_y = rect.center().y - 16.0;
        let text_y = rect.center().y + 16.0;

        // Animated upload icon
        let bounce = (time * 2.0).sin() * 4.0;
        icons::draw_icon(
            painter,
            icons::IconType::Upload,
            Pos2::new(rect.center().x, icon_y + bounce),
            28.0,
            if is_hovered {
                theme::ACCENT_BLUE
            } else {
                theme::TEXT_DIM
            },
        );

        painter.text(
            Pos2::new(rect.center().x, text_y),
            egui::Align2::CENTER_CENTER,
            "Drop MKV file here or click to browse",
            egui::FontId::new(14.0, egui::FontFamily::Name("JetBrains".into())),
            theme::TEXT_SECONDARY,
        );
    }

    // Handle file drop
    ctx.input(|i| {
        if !i.raw.dropped_files.is_empty() {
            if let Some(path) = i.raw.dropped_files[0].path.clone() {
                if path.extension().map_or(false, |ext| ext == "mkv") {
                    app.select_file(path);
                }
            }
        }
    });

    // Handle click to browse
    if response.clicked() {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("MKV Video", &["mkv"])
            .pick_file()
        {
            app.select_file(path);
        }
    }
}

fn render_episode_info(app: &SubMergeApp, ui: &mut Ui) {
    if let Some(ref info) = app.episode_info {
        let (rect, _) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), 60.0),
            Sense::hover(),
        );

        let painter = ui.painter();

        painter.rect_filled(
            rect,
            egui::Rounding::same(12.0),
            theme::BG_SECONDARY,
        );

        // Season badge
        let season_rect = Rect::from_min_size(
            rect.min + Vec2::new(16.0, 14.0),
            Vec2::new(60.0, 32.0),
        );
        painter.rect_filled(
            season_rect,
            egui::Rounding::same(6.0),
            theme::ACCENT_PURPLE.gamma_multiply(0.3),
        );
        painter.text(
            season_rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("S{:02}", info.season),
            egui::FontId::new(16.0, egui::FontFamily::Name("JetBrains".into())),
            theme::ACCENT_PURPLE,
        );

        // Episode badge
        let episode_rect = Rect::from_min_size(
            rect.min + Vec2::new(84.0, 14.0),
            Vec2::new(60.0, 32.0),
        );
        painter.rect_filled(
            episode_rect,
            egui::Rounding::same(6.0),
            theme::ACCENT_CYAN.gamma_multiply(0.3),
        );
        painter.text(
            episode_rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("E{:02}", info.episode),
            egui::FontId::new(16.0, egui::FontFamily::Name("JetBrains".into())),
            theme::ACCENT_CYAN,
        );

        // Detected label
        painter.text(
            rect.right_center() - Vec2::new(16.0, 0.0),
            egui::Align2::RIGHT_CENTER,
            "Detected from filename",
            egui::FontId::new(11.0, egui::FontFamily::Name("JetBrains".into())),
            theme::TEXT_DIM,
        );
    }
}

fn render_show_input(app: &mut SubMergeApp, ui: &mut Ui) {
    let time = app.animation.time as f32;

    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Show Name")
                .color(theme::TEXT_SECONDARY)
                .size(12.0),
        );
    });

    ui.add_space(4.0);

    let input_rect = ui.available_rect_before_wrap();
    let rect = Rect::from_min_size(input_rect.min, Vec2::new(ui.available_width(), 44.0));

    let painter = ui.painter();

    // Gradient border when focused
    let gradient_progress = (time * 2.0) % 1.0;
    components::draw_gradient_border(painter, rect, 1.5, gradient_progress);

    painter.rect_filled(
        rect.shrink(1.5),
        egui::Rounding::same(8.0),
        theme::BG_TERTIARY,
    );

    let text_rect = rect.shrink2(Vec2::new(12.0, 8.0));
    let response = ui.put(
        text_rect,
        TextEdit::singleline(&mut app.show_name)
            .font(egui::FontId::new(
                14.0,
                egui::FontFamily::Name("JetBrains".into()),
            ))
            .text_color(theme::TEXT_PRIMARY)
            .frame(false)
            .hint_text(
                RichText::new("Enter show name...")
                    .color(theme::TEXT_DIM),
            ),
    );

    ui.add_space(rect.height() - text_rect.height());

    // Language selector
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Language:")
                .color(theme::TEXT_SECONDARY)
                .size(12.0),
        );

        let languages = [("en", "English"), ("es", "Spanish"), ("fr", "French"), ("de", "German")];

        for (code, name) in languages {
            let selected = app.language == code;
            let btn_color = if selected {
                theme::ACCENT_BLUE
            } else {
                theme::BG_TERTIARY
            };

            if ui
                .add(
                    egui::Button::new(
                        RichText::new(name)
                            .color(if selected {
                                Color32::WHITE
                            } else {
                                theme::TEXT_SECONDARY
                            })
                            .size(11.0),
                    )
                    .fill(btn_color)
                    .rounding(egui::Rounding::same(4.0)),
                )
                .clicked()
            {
                app.language = code.to_string();
            }
        }
    });
}

fn render_subtitle_list(app: &mut SubMergeApp, ui: &mut Ui) {
    ui.horizontal(|ui| {
        icons::draw_icon(
            ui.painter(),
            icons::IconType::Subtitles,
            ui.cursor().min + Vec2::new(8.0, 8.0),
            14.0,
            theme::ACCENT_GREEN,
        );
        ui.add_space(28.0);
        ui.label(
            RichText::new("Available Subtitles")
                .color(theme::TEXT_PRIMARY)
                .size(14.0),
        );
    });

    ui.add_space(8.0);

    ScrollArea::vertical()
        .max_height(150.0)
        .show(ui, |ui| {
            for (i, sub) in app.subtitles.iter().enumerate() {
                let is_selected = app.selected_subtitle == Some(i);

                let (rect, response) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), 50.0),
                    Sense::click(),
                );

                let painter = ui.painter();

                let bg_color = if is_selected {
                    theme::ACCENT_BLUE.gamma_multiply(0.2)
                } else if response.hovered() {
                    theme::BG_TERTIARY
                } else {
                    theme::BG_SECONDARY
                };

                painter.rect_filled(rect, egui::Rounding::same(8.0), bg_color);

                if is_selected {
                    painter.rect_stroke(
                        rect,
                        egui::Rounding::same(8.0),
                        Stroke::new(1.5, theme::ACCENT_BLUE),
                    );
                }

                // Selection indicator
                if is_selected {
                    icons::draw_icon(
                        painter,
                        icons::IconType::Check,
                        rect.left_center() + Vec2::new(20.0, 0.0),
                        12.0,
                        theme::ACCENT_GREEN,
                    );
                }

                // Subtitle name
                painter.text(
                    rect.left_center() + Vec2::new(if is_selected { 40.0 } else { 16.0 }, -8.0),
                    egui::Align2::LEFT_CENTER,
                    &sub.name,
                    egui::FontId::new(12.0, egui::FontFamily::Name("JetBrains".into())),
                    theme::TEXT_PRIMARY,
                );

                // Language badge
                painter.text(
                    rect.left_center() + Vec2::new(if is_selected { 40.0 } else { 16.0 }, 10.0),
                    egui::Align2::LEFT_CENTER,
                    sub.language.to_uppercase(),
                    egui::FontId::new(10.0, egui::FontFamily::Name("JetBrains".into())),
                    theme::TEXT_DIM,
                );

                if response.clicked() {
                    app.selected_subtitle = Some(i);
                }

                ui.add_space(4.0);
            }
        });
}

fn render_action_buttons(app: &mut SubMergeApp, ui: &mut Ui) {
    let time = app.animation.time as f32;

    ui.horizontal(|ui| {
        let button_width = (ui.available_width() - 12.0) / 2.0;

        // Search button
        let search_enabled = app.state == AppState::FileSelected && !app.show_name.is_empty();
        let search_rect = Rect::from_min_size(
            ui.cursor().min,
            Vec2::new(button_width, 48.0),
        );

        let search_response = ui.allocate_rect(search_rect, Sense::click());
        let painter = ui.painter();

        let search_color = if search_enabled {
            theme::ACCENT_BLUE
        } else {
            theme::BG_TERTIARY
        };

        // Button glow when processing
        if app.state == AppState::Searching {
            let glow_alpha = ((time * 4.0).sin() * 0.3 + 0.5) as f32;
            painter.rect_filled(
                search_rect.expand(4.0),
                egui::Rounding::same(14.0),
                search_color.gamma_multiply(glow_alpha * 0.5),
            );
        }

        painter.rect_filled(
            search_rect,
            egui::Rounding::same(10.0),
            search_color,
        );

        icons::draw_icon(
            painter,
            icons::IconType::Search,
            search_rect.center() - Vec2::new(40.0, 0.0),
            14.0,
            if search_enabled {
                Color32::WHITE
            } else {
                theme::TEXT_DIM
            },
        );

        painter.text(
            search_rect.center() + Vec2::new(8.0, 0.0),
            egui::Align2::LEFT_CENTER,
            if app.state == AppState::Searching {
                "Searching..."
            } else {
                "Search"
            },
            egui::FontId::new(14.0, egui::FontFamily::Name("JetBrains".into())),
            if search_enabled {
                Color32::WHITE
            } else {
                theme::TEXT_DIM
            },
        );

        if search_response.clicked() && search_enabled {
            app.search_subtitles();
        }

        ui.add_space(button_width + 12.0);

        // Merge button
        let merge_enabled = app.selected_subtitle.is_some()
            && matches!(
                app.state,
                AppState::SubtitlesFound | AppState::Complete
            );
        let merge_rect = Rect::from_min_size(
            ui.cursor().min,
            Vec2::new(button_width, 48.0),
        );

        let merge_response = ui.allocate_rect(merge_rect, Sense::click());

        let merge_color = if merge_enabled {
            if app.mode == AppMode::DryRun {
                theme::ACCENT_YELLOW
            } else {
                theme::ACCENT_GREEN
            }
        } else {
            theme::BG_TERTIARY
        };

        // Button glow when processing
        if matches!(app.state, AppState::Downloading | AppState::Merging) {
            let glow_alpha = ((time * 4.0).sin() * 0.3 + 0.5) as f32;
            painter.rect_filled(
                merge_rect.expand(4.0),
                egui::Rounding::same(14.0),
                merge_color.gamma_multiply(glow_alpha * 0.5),
            );
        }

        painter.rect_filled(
            merge_rect,
            egui::Rounding::same(10.0),
            merge_color,
        );

        let merge_icon = if app.mode == AppMode::DryRun {
            icons::IconType::Eye
        } else {
            icons::IconType::Merge
        };

        icons::draw_icon(
            painter,
            merge_icon,
            merge_rect.center() - Vec2::new(50.0, 0.0),
            14.0,
            if merge_enabled {
                Color32::WHITE
            } else {
                theme::TEXT_DIM
            },
        );

        let merge_text = match app.state {
            AppState::Downloading => "Downloading...",
            AppState::Merging => "Merging...",
            _ => {
                if app.mode == AppMode::DryRun {
                    "Dry Run"
                } else {
                    "Merge"
                }
            }
        };

        painter.text(
            merge_rect.center() + Vec2::new(-8.0, 0.0),
            egui::Align2::LEFT_CENTER,
            merge_text,
            egui::FontId::new(14.0, egui::FontFamily::Name("JetBrains".into())),
            if merge_enabled {
                Color32::WHITE
            } else {
                theme::TEXT_DIM
            },
        );

        if merge_response.clicked() && merge_enabled {
            app.download_and_merge();
        }
    });

    ui.add_space(48.0); // Reserve space for buttons
}

fn render_log_panel(app: &SubMergeApp, ui: &mut Ui) {
    ui.horizontal(|ui| {
        icons::draw_icon(
            ui.painter(),
            icons::IconType::Terminal,
            ui.cursor().min + Vec2::new(8.0, 8.0),
            14.0,
            theme::TEXT_DIM,
        );
        ui.add_space(28.0);
        ui.label(
            RichText::new("Log")
                .color(theme::TEXT_SECONDARY)
                .size(12.0),
        );
    });

    ui.add_space(8.0);

    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), 160.0),
        Sense::hover(),
    );

    let painter = ui.painter();
    painter.rect_filled(rect, egui::Rounding::same(8.0), theme::BG_SECONDARY);

    let inner_rect = rect.shrink(12.0);

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .show_viewport(ui, |ui, viewport| {
            ui.set_min_size(inner_rect.size());

            for entry in &app.logs {
                let color = match entry.log_type {
                    LogType::Info => theme::TEXT_SECONDARY,
                    LogType::Success => theme::ACCENT_GREEN,
                    LogType::Warning => theme::ACCENT_YELLOW,
                    LogType::Error => theme::ACCENT_RED,
                };

                let prefix = match entry.log_type {
                    LogType::Info => "→",
                    LogType::Success => "✓",
                    LogType::Warning => "!",
                    LogType::Error => "✗",
                };

                ui.horizontal(|ui| {
                    ui.label(RichText::new(prefix).color(color).size(11.0));
                    ui.label(
                        RichText::new(&entry.message)
                            .color(color)
                            .size(11.0)
                            .family(egui::FontFamily::Name("JetBrains".into())),
                    );
                });
            }
        });
}
