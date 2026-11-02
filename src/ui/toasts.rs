//! Toasts in the page's bottom-right corner: short-lived confirmations
//! ("Found 2 new projects") that fade out, and errors that stay until clicked
//! away. Hovering a toast keeps it; long text wraps.

use eframe::egui::{self, Align2, CornerRadius, FontId, Rect, Role, Sense, Stroke, vec2};

use super::theme::colors::{BORDER, CARD_BG, DELETE_RED_SOFT, TEXT, TEXT_WEAK};

/// Seconds a confirmation stays fully visible, then fades over [`FADE`].
const HOLD: f64 = 1.8;
const FADE: f64 = 0.5;
/// Most toasts shown at once (the oldest go first).
const MAX_VISIBLE: usize = 4;
/// Widest a toast grows before its text wraps (points).
const MAX_WIDTH: f32 = 360.0;

/// What a toast says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToastKind {
    /// A confirmation; fades by itself.
    Info,
    /// Something failed; stays until clicked.
    Error,
}

#[derive(Clone)]
struct Toast {
    text: String,
    kind: ToastKind,
    shown_at: f64,
}

/// Queue `messages` and `errors`, then draw the live toasts inside `area`.
pub fn render(ctx: &egui::Context, area: Rect, messages: Vec<String>, errors: Vec<String>) {
    let id = egui::Id::unique("toasts");
    let now = ctx.input(|i| i.time);
    // Take the list instead of cloning every toast's text; put it back at
    // the end of the frame.
    let mut toasts = Vec::new();
    ctx.data_mut(|d| {
        let stored = d.get_temp_mut_or_default::<Vec<Toast>>(id);
        toasts = std::mem::take(stored);
    });
    let incoming = messages
        .into_iter()
        .map(|text| (text, ToastKind::Info))
        .chain(errors.into_iter().map(|text| (text, ToastKind::Error)));
    for (text, kind) in incoming {
        // The same message again refreshes the toast instead of stacking.
        toasts.retain(|t| t.text != text);
        toasts.push(Toast {
            text,
            kind,
            shown_at: now,
        });
    }
    toasts.retain(|t| t.kind == ToastKind::Error || now - t.shown_at < HOLD + FADE);
    if toasts.len() > MAX_VISIBLE {
        toasts.drain(..toasts.len() - MAX_VISIBLE);
    }
    if toasts.is_empty() {
        ctx.data_mut(|d| d.remove::<Vec<Toast>>(id));
        return;
    }

    let mut bottom = area.bottom() - 14.0;
    let mut dismissed = Vec::new();
    for (ix, toast) in toasts.iter_mut().enumerate().rev() {
        let age = now - toast.shown_at;
        let alpha = if toast.kind == ToastKind::Error || age < HOLD {
            1.0
        } else {
            (1.0 - (age - HOLD) / FADE).clamp(0.0, 1.0) as f32
        };
        let galley = ctx.fonts_mut(|fonts| {
            fonts.layout(
                toast.text.clone(),
                FontId::proportional(13.0),
                TEXT.gamma_multiply(alpha),
                MAX_WIDTH,
            )
        });
        let error = toast.kind == ToastKind::Error;
        let extra = if error { 22.0 } else { 0.0 };
        let size = galley.size() + vec2(28.0 + extra, 16.0);
        let rect = Rect::from_min_size(
            egui::pos2(area.right() - 14.0 - size.x, bottom - size.y),
            size,
        );
        let response = egui::Area::new(id.with(("toast", ix)))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.min)
            .show(ctx, |ui| {
                let response = ui.allocate_rect(rect, Sense::click());
                let painter = ui.painter();
                painter.rect(
                    rect,
                    CornerRadius::same(8),
                    CARD_BG.gamma_multiply(0.96 * alpha),
                    Stroke::new(
                        1.0,
                        if error { DELETE_RED_SOFT } else { BORDER }.gamma_multiply(alpha),
                    ),
                    egui::StrokeKind::Inside,
                );
                let text_min = Align2::LEFT_CENTER
                    .anchor_size(rect.left_center() + vec2(14.0, 0.0), galley.size())
                    .min;
                painter.galley(text_min, galley, TEXT);
                if error {
                    painter.text(
                        rect.right_center() - vec2(14.0, 0.0),
                        Align2::CENTER_CENTER,
                        "×",
                        FontId::proportional(15.0),
                        if response.hovered() { TEXT } else { TEXT_WEAK },
                    );
                }
                response
            })
            .inner;
        // Hovering holds a confirmation; clicking any toast dismisses it.
        if response.hovered() && !error {
            toast.shown_at = now;
        }
        if response.clicked() {
            dismissed.push(ix);
        }
        let response = if error {
            response.on_hover_text("Click to dismiss")
        } else {
            response
        };
        response.widget_info(|| egui::WidgetInfo::labeled(Role::Label, true, &toast.text));
        bottom = rect.top() - 8.0;
    }
    for ix in dismissed {
        toasts.remove(ix);
    }
    // Idle until the oldest confirmation starts fading, then animate.
    let fade_in = toasts
        .iter()
        .filter(|t| t.kind == ToastKind::Info)
        .map(|t| HOLD - (now - t.shown_at))
        .fold(f64::INFINITY, f64::min);
    ctx.data_mut(|d| d.insert_temp(id, toasts));
    if fade_in.is_finite() {
        if fade_in > 0.0 {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(fade_in));
        } else {
            ctx.request_repaint();
        }
    }
}
