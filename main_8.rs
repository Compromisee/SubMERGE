use egui::{Color32, Painter, Pos2, Stroke, Vec2};

#[derive(Clone, Copy)]
pub enum IconType {
    File,
    Upload,
    Search,
    Check,
    Merge,
    Eye,
    Bolt,
    Subtitles,
    Terminal,
}

pub fn draw_icon(painter: &Painter, icon: IconType, center: Pos2, size: f32, color: Color32) {
    let stroke = Stroke::new(1.5, color);
    let half = size / 2.0;

    match icon {
        IconType::File => {
            // File icon
            let points = [
                center + Vec2::new(-half * 0.6, -half),
                center + Vec2::new(half * 0.3, -half),
                center + Vec2::new(half * 0.6, -half * 0.7),
                center + Vec2::new(half * 0.6, half),
                center + Vec2::new(-half * 0.6, half),
            ];
            painter.line_segment([points[0], points[1]], stroke);
            painter.line_segment([points[1], points[2]], stroke);
            painter.line_segment([points[2], points[3]], stroke);
            painter.line_segment([points[3], points[4]], stroke);
            painter.line_segment([points[4], points[0]], stroke);
            // Fold corner
            painter.line_segment(
                [points[1], points[1] + Vec2::new(0.0, half * 0.3)],
                stroke,
            );
            painter.line_segment(
                [points[1] + Vec2::new(0.0, half * 0.3), points[2]],
                stroke,
            );
        }
        IconType::Upload => {
            // Upload arrow
            let arrow_top = center + Vec2::new(0.0, -half * 0.8);
            let arrow_bottom = center + Vec2::new(0.0, half * 0.3);
            painter.line_segment([arrow_bottom, arrow_top], stroke);
            painter.line_segment(
                [arrow_top, arrow_top + Vec2::new(-half * 0.4, half * 0.4)],
                stroke,
            );
            painter.line_segment(
                [arrow_top, arrow_top + Vec2::new(half * 0.4, half * 0.4)],
                stroke,
            );
            // Base line
            painter.line_segment(
                [
                    center + Vec2::new(-half * 0.7, half * 0.8),
                    center + Vec2::new(half * 0.7, half * 0.8),
                ],
                stroke,
            );
        }
        IconType::Search => {
            // Magnifying glass
            let circle_center = center + Vec2::new(-half * 0.15, -half * 0.15);
            let radius = half * 0.55;
            painter.circle_stroke(circle_center, radius, stroke);
            // Handle
            let handle_start = circle_center + Vec2::new(radius * 0.7, radius * 0.7);
            let handle_end = center + Vec2::new(half * 0.7, half * 0.7);
            painter.line_segment([handle_start, handle_end], Stroke::new(2.0, color));
        }
        IconType::Check => {
            // Checkmark
            let points = [
                center + Vec2::new(-half * 0.6, 0.0),
                center + Vec2::new(-half * 0.15, half * 0.45),
                center + Vec2::new(half * 0.6, -half * 0.45),
            ];
            painter.line_segment([points[0], points[1]], Stroke::new(2.0, color));
            painter.line_segment([points[1], points[2]], Stroke::new(2.0, color));
        }
        IconType::Merge => {
            // Merge arrows
            painter.line_segment(
                [
                    center + Vec2::new(-half * 0.7, -half * 0.5),
                    center + Vec2::new(0.0, 0.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    center + Vec2::new(-half * 0.7, half * 0.5),
                    center + Vec2::new(0.0, 0.0),
                ],
                stroke,
            );
            painter.line_segment(
                [center, center + Vec2::new(half * 0.7, 0.0)],
                stroke,
            );
            // Arrow head
            painter.line_segment(
                [
                    center + Vec2::new(half * 0.7, 0.0),
                    center + Vec2::new(half * 0.35, -half * 0.25),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    center + Vec2::new(half * 0.7, 0.0),
                    center + Vec2::new(half * 0.35, half * 0.25),
                ],
                stroke,
            );
        }
        IconType::Eye => {
            // Eye icon
            let eye_width = half * 1.4;
            let eye_height = half * 0.7;

            // Eye shape (two arcs)
            for i in 0..12 {
                let t1 = i as f32 / 12.0;
                let t2 = (i + 1) as f32 / 12.0;

                let x1 = (t1 * std::f32::consts::PI).cos() * eye_width;
                let y1 = (t1 * std::f32::consts::PI).sin() * eye_height;
                let x2 = (t2 * std::f32::consts::PI).cos() * eye_width;
                let y2 = (t2 * std::f32::consts::PI).sin() * eye_height;

                painter.line_segment(
                    [
                        center + Vec2::new(x1, y1 * 0.5),
                        center + Vec2::new(x2, y2 * 0.5),
                    ],
                    stroke,
                );
                painter.line_segment(
                    [
                        center + Vec2::new(x1, -y1 * 0.5),
                        center + Vec2::new(x2, -y2 * 0.5),
                    ],
                    stroke,
                );
            }

            // Pupil
            painter.circle_filled(center, half * 0.25, color);
        }
        IconType::Bolt => {
            // Lightning bolt
            let points = [
                center + Vec2::new(half * 0.1, -half),
                center + Vec2::new(-half * 0.3, half * 0.1),
                center + Vec2::new(half * 0.1, half * 0.1),
                center + Vec2::new(-half * 0.1, half),
                center + Vec2::new(half * 0.3, -half * 0.1),
                center + Vec2::new(-half * 0.1, -half * 0.1),
            ];
            for i in 0..points.len() {
                painter.line_segment([points[i], points[(i + 1) % points.len()]], stroke);
            }
        }
        IconType::Subtitles => {
            // Subtitle icon (CC badge)
            let rect_half_w = half * 0.9;
            let rect_half_h = half * 0.6;

            // Outer rectangle
            painter.rect_stroke(
                egui::Rect::from_center_size(center, Vec2::new(rect_half_w * 2.0, rect_half_h * 2.0)),
                egui::Rounding::same(3.0),
                stroke,
            );

            // Text lines
            painter.line_segment(
                [
                    center + Vec2::new(-rect_half_w * 0.6, -rect_half_h * 0.3),
                    center + Vec2::new(rect_half_w * 0.6, -rect_half_h * 0.3),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    center + Vec2::new(-rect_half_w * 0.6, rect_half_h * 0.3),
                    center + Vec2::new(rect_half_w * 0.3, rect_half_h * 0.3),
                ],
                stroke,
            );
        }
        IconType::Terminal => {
            // Terminal/console icon
            painter.rect_stroke(
                egui::Rect::from_center_size(center, Vec2::new(half * 1.8, half * 1.4)),
                egui::Rounding::same(2.0),
                stroke,
            );
            // Prompt
            painter.line_segment(
                [
                    center + Vec2::new(-half * 0.5, 0.0),
                    center + Vec2::new(-half * 0.2, -half * 0.25),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    center + Vec2::new(-half * 0.5, 0.0),
                    center + Vec2::new(-half * 0.2, half * 0.25),
                ],
                stroke,
            );
            // Cursor line
            painter.line_segment(
                [
                    center + Vec2::new(half * 0.1, half * 0.15),
                    center + Vec2::new(half * 0.5, half * 0.15),
                ],
                stroke,
            );
        }
    }
}
