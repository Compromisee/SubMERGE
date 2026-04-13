use crate::ui::theme;
use egui::{Color32, Painter, Pos2, Rect, Rounding, Stroke, Vec2};

pub fn draw_gradient_border(painter: &Painter, rect: Rect, width: f32, progress: f32) {
    let perimeter = (rect.width() + rect.height()) * 2.0;
    let segments = 60;

    for i in 0..segments {
        let t1 = i as f32 / segments as f32;
        let t2 = (i + 1) as f32 / segments as f32;

        let pos1 = rect_point_at_t(rect, t1);
        let pos2 = rect_point_at_t(rect, t2);

        let color_t = (t1 + progress) % 1.0;
        let color = theme::gradient_color(color_t);

        painter.line_segment([pos1, pos2], Stroke::new(width, color));
    }
}

fn rect_point_at_t(rect: Rect, t: f32) -> Pos2 {
    let perimeter = (rect.width() + rect.height()) * 2.0;
    let distance = t * perimeter;

    let top_width = rect.width();
    let right_height = rect.height();
    let bottom_width = rect.width();

    if distance < top_width {
        // Top edge
        Pos2::new(rect.left() + distance, rect.top())
    } else if distance < top_width + right_height {
        // Right edge
        let d = distance - top_width;
        Pos2::new(rect.right(), rect.top() + d)
    } else if distance < top_width + right_height + bottom_width {
        // Bottom edge
        let d = distance - top_width - right_height;
        Pos2::new(rect.right() - d, rect.bottom())
    } else {
        // Left edge
        let d = distance - top_width - right_height - bottom_width;
        Pos2::new(rect.left(), rect.bottom() - d)
    }
}

pub fn draw_dashed_rect(
    painter: &Painter,
    rect: Rect,
    width: f32,
    color: Color32,
    offset: f32,
) {
    let dash_length = 10.0;
    let gap_length = 6.0;
    let total = dash_length + gap_length;

    let corners = [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
    ];

    for i in 0..4 {
        let start = corners[i];
        let end = corners[(i + 1) % 4];
        let edge_vec = end - start;
        let edge_length = edge_vec.length();
        let edge_dir = edge_vec / edge_length;

        let mut pos = offset % total;

        while pos < edge_length {
            let dash_start = start + edge_dir * pos;
            let dash_end_pos = (pos + dash_length).min(edge_length);
            let dash_end = start + edge_dir * dash_end_pos;

            painter.line_segment([dash_start, dash_end], Stroke::new(width, color));

            pos += total;
        }
    }
}

pub fn draw_rounded_gradient_rect(
    painter: &Painter,
    rect: Rect,
    rounding: Rounding,
    start_color: Color32,
    end_color: Color32,
    vertical: bool,
) {
    let steps = 20;

    for i in 0..steps {
        let t1 = i as f32 / steps as f32;
        let t2 = (i + 1) as f32 / steps as f32;

        let color = lerp_color(start_color, end_color, (t1 + t2) / 2.0);

        let slice = if vertical {
            Rect::from_min_max(
                Pos2::new(rect.left(), rect.top() + rect.height() * t1),
                Pos2::new(rect.right(), rect.top() + rect.height() * t2),
            )
        } else {
            Rect::from_min_max(
                Pos2::new(rect.left() + rect.width() * t1, rect.top()),
                Pos2::new(rect.left() + rect.width() * t2, rect.bottom()),
            )
        };

        let slice_rounding = if i == 0 {
            Rounding {
                nw: rounding.nw,
                ne: if vertical { 0.0 } else { rounding.ne },
                sw: if vertical { 0.0 } else { rounding.sw },
                se: 0.0,
            }
        } else if i == steps - 1 {
            Rounding {
                nw: 0.0,
                ne: if vertical { rounding.ne } else { 0.0 },
                sw: if vertical { rounding.sw } else { 0.0 },
                se: rounding.se,
            }
        } else {
            Rounding::ZERO
        };

        painter.rect_filled(slice, slice_rounding, color);
    }
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgba_unmultiplied(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
        (a.a() as f32 + (b.a() as f32 - a.a() as f32) * t) as u8,
    )
}
