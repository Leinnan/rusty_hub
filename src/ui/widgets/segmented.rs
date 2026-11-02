//! macOS-style segmented control.

use eframe::egui::{
    Align2, CornerRadius, FontId, Pos2, Rect, Response, Role, Sense, Stroke, StrokeKind, Ui, Vec2,
    WidgetInfo,
};

use crate::ui::theme::colors::{BORDER, CARD_BG, CARD_BG_HOVER, SELECTION_BG, TEXT, TEXT_WEAK};

/// Segment height (px), matching a regular-size `NSSegmentedControl`.
const SEGMENT_HEIGHT: f32 = 24.0;

/// Horizontal text padding inside each segment (px).
const SEGMENT_PAD_X: f32 = 12.0;

/// Minimum segment width (px).
const SEGMENT_MIN_WIDTH: f32 = 44.0;

/// Outer corner radius of the control (px).
const SEGMENT_ROUNDING: u8 = 6;

/// A row of segments inside one rounded container. `selected` is the index
/// of the highlighted segment. Returns one `Response` per segment.
pub fn segmented_control(
    ui: &mut Ui,
    id_salt: &str,
    labels: &[&str],
    selected: Option<usize>,
) -> Vec<Response> {
    let font = FontId::proportional(12.0);
    let widths: Vec<f32> = labels
        .iter()
        .map(|label| {
            let width = ui
                .fonts_mut(|f| f.layout_no_wrap((*label).to_owned(), font.clone(), TEXT))
                .size()
                .x;
            (width + SEGMENT_PAD_X * 2.0).max(SEGMENT_MIN_WIDTH).ceil()
        })
        .collect();
    let total_width: f32 = widths.iter().sum();
    let (outer, _) = ui.allocate_exact_size(Vec2::new(total_width, SEGMENT_HEIGHT), Sense::hover());

    let painter = ui.painter().clone();
    let rounding = CornerRadius::same(SEGMENT_ROUNDING);
    painter.rect(
        outer,
        rounding,
        CARD_BG,
        Stroke::new(1.0, BORDER),
        StrokeKind::Inside,
    );

    let base_id = ui.scope_id().with(id_salt);
    let mut responses = Vec::with_capacity(labels.len());
    let mut x = outer.min.x;
    for (ix, (label, width)) in labels.iter().zip(&widths).enumerate() {
        let rect =
            Rect::from_min_size(Pos2::new(x, outer.min.y), Vec2::new(*width, SEGMENT_HEIGHT));
        let response = ui.interact(rect, base_id.with(ix), Sense::click());
        let is_selected = selected == Some(ix);
        response.widget_info(|| WidgetInfo::selected(Role::Button, true, is_selected, *label));
        if is_selected {
            painter.rect_filled(
                rect.shrink(2.0),
                CornerRadius::same(SEGMENT_ROUNDING - 2),
                SELECTION_BG,
            );
        } else if response.hovered() {
            painter.rect_filled(
                rect.shrink(2.0),
                CornerRadius::same(SEGMENT_ROUNDING - 2),
                CARD_BG_HOVER,
            );
        }
        // Hairline separator between two unselected neighbours.
        let next_selected = selected == Some(ix + 1);
        if ix + 1 < labels.len() && !is_selected && !next_selected {
            painter.line_segment(
                [
                    Pos2::new(rect.max.x, rect.min.y + 5.0),
                    Pos2::new(rect.max.x, rect.max.y - 5.0),
                ],
                Stroke::new(1.0, BORDER),
            );
        }
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            *label,
            font.clone(),
            if is_selected { TEXT } else { TEXT_WEAK },
        );
        responses.push(response);
        x += width;
    }
    responses
}
