//! The Scriptographer Console panel, as the original Scriptographer's console was: a multi-line field on top where
//! Return runs the line it ends as PaperScript, in the console's own project on the active document
//! (each line one undo step), and under it everything scripts printed, errors in the error colour.
//! The divider between them sets the field's height.

use egui::{CursorIcon, Sense, Ui, vec2};
use serde_json::json;

use super::{pstate, set_pstate};
use crate::theme::Tokens;
use crate::{VectorcraftApp, widgets};

pub const ID: &str = "scriptographerConsole";

pub(crate) const INPUT: &str = "scriptographer-console-input";

/// The field's height in lines, which the divider under it sets.
const INPUT_LINES: &str = "scriptographer-console-input-lines";

pub fn show(app: &mut VectorcraftApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    // A multi-line field, as Scriptographer's: Return goes to a new line as usual, and runs the line
    // it ends. The text stays, to run lines again.
    let mut input: String = pstate(ui.ctx(), INPUT);
    let mut lines: usize = pstate::<Option<usize>>(ui.ctx(), INPUT_LINES).unwrap_or(2);
    let entered = ui.input(|i| i.key_pressed(egui::Key::Enter) && i.modifiers.is_none());
    let line_height = ui.text_style_height(&egui::TextStyle::Monospace);
    // Longer text than the field shows scrolls in it:
    let field = egui::ScrollArea::vertical()
        .id_salt("scriptographer-console-input-scroll")
        .max_height(lines as f32 * line_height + 2.0 * ui.spacing().button_padding.y)
        .show(ui, |ui| {
            egui::TextEdit::multiline(&mut input)
                .id(egui::Id::unique(INPUT))
                .font(egui::TextStyle::Monospace)
                .desired_rows(lines)
                .desired_width(f32::INFINITY)
                .show(ui)
        })
        .inner;
    if entered
        && field.response.has_focus()
        && let Some(cursor) = field.cursor_range.map(|c| c.primary.index.0)
    {
        let line = ended_line(&input, cursor);
        if !line.is_empty()
            && let Err(e) = app.run("ui.scriptographerEvaluate", json!({ "code": line }))
        {
            app.status(e);
        }
    }
    set_pstate(ui.ctx(), INPUT, input);
    // The log keeps at least a few lines:
    let most = ((ui.available_height() - 4.0 * line_height) / line_height).max(1.0) as usize;
    resize_handle(ui, &mut lines, line_height, most);
    set_pstate(ui.ctx(), INPUT_LINES, Some(lines));
    // Laid out from the bottom: the bar stays at the bottom, the log takes the height above it.
    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
        super::scriptographer::bottom_bar(ui, |ui| {
            // Scriptographer's console cleared with its refresh arrows.
            if widgets::icon_button(ui, "refresh-cw", tl!("Clear Console"), false, super::scriptographer::BUTTON).clicked() {
                app.scripts.clear_log();
            }
        });
        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
            egui::ScrollArea::vertical().id_salt("scriptographer-console-log").auto_shrink([false, false]).stick_to_bottom(true).show(ui, |ui| {
                // One read-only text, so lines can be selected together and copied.
                let mut text = String::new();
                let mut errors = Vec::new();
                for line in app.scripts.log() {
                    let start = text.len();
                    text.push_str(&line.text);
                    text.push('\n');
                    if line.error {
                        errors.push(start..text.len());
                    }
                }
                let font = egui::TextStyle::Monospace.resolve(ui.style());
                let mut layouter = |ui: &Ui, text: &dyn egui::TextBuffer, width: f32| {
                    let text = text.as_str();
                    let mut job = egui::text::LayoutJob::default();
                    job.wrap.max_width = width;
                    let mut at = 0;
                    for range in errors.iter().cloned().chain(std::iter::once(text.len()..text.len())) {
                        let format = |color| egui::TextFormat::simple(font.clone(), color);
                        job.append(text.get(at..range.start).unwrap_or_default(), 0.0, format(t.text));
                        job.append(text.get(range.clone()).unwrap_or_default(), 0.0, format(t.error));
                        at = range.end;
                    }
                    ui.fonts_mut(|f| f.layout_job(job))
                };
                ui.add(egui::TextEdit::multiline(&mut text.as_str()).desired_width(f32::INFINITY).frame(egui::Frame::NONE).layouter(&mut layouter));
            });
        });
    });
}

/// The divider under the field, which sets its height in `lines`, from one to `most`: dragged, or
/// a line at a time with ↑ and ↓ once Tab gave it the keyboard focus, which draws it thicker.
fn resize_handle(ui: &mut Ui, lines: &mut usize, line_height: f32, most: usize) {
    let t = Tokens::get(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 13.0), Sense::click_and_drag());
    let response = response.on_hover_cursor(CursorIcon::ResizeVertical);
    // The lines the field had as the drag started, and as many more as the pointer moved down:
    if response.drag_started()
        && let Some(pointer) = ui.input(|i| i.pointer.press_origin())
    {
        ui.data_mut(|d| d.insert_temp(response.id, (*lines, pointer.y)));
    }
    if response.dragged()
        && let (Some(pointer), Some((start, y))) = (response.interact_pointer_pos(), ui.data(|d| d.get_temp::<(usize, f32)>(response.id)))
    {
        *lines = start.saturating_add_signed(((pointer.y - y) / line_height).round() as isize);
    }
    if response.has_focus() {
        ui.input_mut(|i| {
            if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp) {
                *lines = lines.saturating_sub(1);
            }
            if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown) {
                *lines += 1;
            }
        });
    }
    *lines = (*lines).clamp(1, most.max(1));
    let active = response.hovered() || response.dragged() || response.has_focus();
    let line = rect.shrink2(vec2(0.0, if response.has_focus() { 5.5 } else { 6.0 }));
    ui.painter().rect_filled(line, 0.0, if active { t.accent } else { t.divider });
    let value = *lines as f64;
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Splitter, true, tl!("Console Input Height")));
    ui.ctx().accesskit_node_builder(response.id, |node| node.set_numeric_value(value));
}

/// The line that a newline before the cursor at `cursor` (a character index) ended, trimmed.
fn ended_line(text: &str, cursor: usize) -> String {
    let before: String = text.chars().take(cursor).collect();
    let before = before.strip_suffix('\n').unwrap_or(&before);
    before.rsplit('\n').next().unwrap_or_default().trim().to_owned()
}

#[cfg(test)]
#[test]
fn return_runs_the_line_it_ends() {
    assert_eq!(ended_line("a = 1\nb = 2\n", 12), "b = 2");
    assert_eq!(ended_line("a = 1\n\nb = 2", 6), "a = 1", "a line in the middle, its newline just typed");
    assert_eq!(ended_line("\n", 1), "", "an empty line runs nothing");
}

/// The divider sets the field's height: dragged down by three lines it's three lines higher, and
/// once Tab focused it, ↑ makes it a line lower. It stays between one line and the most the log leaves.
#[cfg(test)]
#[test]
fn the_divider_sets_the_fields_height() {
    let ctx = egui::Context::default();
    let mut lines = 2;
    let handle = std::cell::Cell::new(egui::Pos2::ZERO);
    let frame = |events: Vec<egui::Event>, lines: &mut usize, most: usize| {
        let input =
            egui::RawInput { events, screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, vec2(400.0, 400.0))), ..Default::default() };
        let mut output = ctx.run_ui(input, |ui| {
            handle.set(ui.available_rect_before_wrap().min + vec2(200.0, 6.5));
            resize_handle(ui, lines, 10.0, most);
        });
        // Nothing draws here:
        output.textures_delta.clear();
    };
    frame(vec![], &mut lines, 10);
    let at = handle.get();
    let button = |pos, pressed| egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
    let below = at + vec2(0.0, 30.0);
    frame(vec![egui::Event::PointerMoved(at), button(at, true)], &mut lines, 10);
    frame(vec![egui::Event::PointerMoved(at + vec2(0.0, 10.0))], &mut lines, 10);
    frame(vec![egui::Event::PointerMoved(below)], &mut lines, 10);
    frame(vec![button(below, false)], &mut lines, 10);
    assert_eq!(lines, 5, "dragged three lines down");
    let key = |key| egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() };
    frame(vec![key(egui::Key::Tab)], &mut lines, 10);
    frame(vec![key(egui::Key::ArrowUp)], &mut lines, 10);
    assert_eq!(lines, 4, "a line lower with ↑");
    frame(vec![], &mut lines, 3);
    assert_eq!(lines, 3, "no higher than the log leaves");
}
