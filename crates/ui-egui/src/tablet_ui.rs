//! Full iPad/Tablet-native UI shell for SolidCraft.
//!
//! Provides a modern, touch-first CAD experience inspired by tablet ergonomics:
//! - Full-bleed edge-to-edge 3D canvas with zero desktop chrome clutter.
//! - Floating island Top Bar with large touch targets for Undo, Redo, Mode Status, Fit View, and Drawer toggles.
//! - Vertical Floating Tool Dock (Shapr3D style) on the left edge with large 48×48pt touch targets,
//!   adapting automatically between 3D Modeling tools and 2D Sketching tools.
//! - Slide-over Items/Feature Drawer that can be quickly toggled without permanently stealing screen width.
//! - Touch Numeric Keypad for fast dimension editing without virtual OS keyboard layout shifts.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::SolveApp;
use crate::ui_kit::{BackingTooltipExt as _, Container, Surface};
use egui::{Align2, Color32, Rect, Sense, pos2, vec2};
use serde_json::json;
use solidcraft_theme::Tokens;

const TOUCH_TARGET: f32 = 44.0;
const DOCK_BTN_SIZE: f32 = 46.0;

/// State for the tablet UI overlay stored across frames.
#[derive(Clone, Debug, Default)]
pub struct TabletUiState {
    pub show_views_menu: bool,
    pub show_items_drawer: bool,
    pub keypad_value: String,
    pub keypad_active: bool,
}

fn state(ctx: &egui::Context) -> TabletUiState {
    ctx.data(|d| d.get_temp(egui::Id::unique("sc_tablet_ui_state"))).unwrap_or_default()
}

fn set_state(ctx: &egui::Context, state: TabletUiState) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::unique("sc_tablet_ui_state"), state));
}

/// Main entry point for the tablet UI overlay.
pub fn show(app: &mut SolveApp, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let screen = ctx.content_rect();
    if screen.width() < 300.0 || screen.height() < 300.0 {
        return;
    }

    let mut st = state(&ctx);

    show_top_bar(app, &ctx, screen, &mut st);
    show_tool_dock(app, &ctx, screen);
    if st.show_items_drawer {
        show_items_drawer(app, &ctx, screen, &mut st);
    }
    if st.keypad_active {
        show_numeric_keypad(app, &ctx, screen, &mut st);
    }

    set_state(&ctx, st);
}

/// Floating Island Top Bar
fn show_top_bar(app: &mut SolveApp, ctx: &egui::Context, screen: Rect, st: &mut TabletUiState) {
    let t = Tokens::get();
    let container = Container::new(ctx, Surface::Navigation);
    let top_margin = 12.0;
    let bar_height = 48.0;

    // --- Left Pill: Doc Name, Undo, Redo ---
    let left_pos = pos2(16.0, top_margin);
    container.area(egui::Id::unique("tablet_top_left")).order(egui::Order::Middle).fixed_pos(left_pos).show(ctx, |ui| {
        let frame = container.frame(ctx, t.overlay, crate::frame::Side::Left).inner_margin(egui::Margin::symmetric(8, 4));
        container.show_frame(ui, frame, screen, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = vec2(6.0, 0.0);

                // Document name
                let doc_name = app.session.doc.name.clone();
                let display_name = if doc_name.is_empty() { "SolidCraft".to_string() } else { doc_name };
                ui.add_space(4.0);
                ui.strong(display_name);
                ui.add_space(6.0);

                // Divider
                let (div_rect, _) = ui.allocate_exact_size(vec2(1.0, 24.0), Sense::hover());
                ui.painter().rect_filled(div_rect, 0.0, t.border);
                ui.add_space(4.0);

                // Undo Button
                let can_undo = !app.session.undo.is_empty();
                let (undo_rect, _) = ui.allocate_exact_size(vec2(TOUCH_TARGET, TOUCH_TARGET), Sense::click());
                let undo_resp = ui.interact(undo_rect, ui.scope_id().with("tablet_undo"), if can_undo { Sense::click() } else { Sense::hover() });
                if can_undo && undo_resp.hovered() {
                    ui.painter().rect_filled(undo_rect, t.radius_md, t.hover);
                }
                crate::icons::paint_icon(ui.painter(), undo_rect.shrink(t.space_8), "undo", if can_undo { 1.0 } else { 0.3 });
                if undo_resp.backing_on_hover_text("Undo (↩)").clicked() && can_undo {
                    let _ = app.run("edit.undo", json!({}));
                }

                // Redo Button
                let can_redo = !app.session.redo.is_empty();
                let (redo_rect, _) = ui.allocate_exact_size(vec2(TOUCH_TARGET, TOUCH_TARGET), Sense::click());
                let redo_resp = ui.interact(redo_rect, ui.scope_id().with("tablet_redo"), if can_redo { Sense::click() } else { Sense::hover() });
                if can_redo && redo_resp.hovered() {
                    ui.painter().rect_filled(redo_rect, t.radius_md, t.hover);
                }
                crate::icons::paint_icon(ui.painter(), redo_rect.shrink(t.space_8), "redo", if can_redo { 1.0 } else { 0.3 });
                if redo_resp.backing_on_hover_text("Redo (↪)").clicked() && can_redo {
                    let _ = app.run("edit.redo", json!({}));
                }
            });
        });
    });

    // --- Center Pill: Mode Pill & Sketch Finish / Cancel ---
    let center_pos = pos2(screen.center().x, top_margin);
    container.area(egui::Id::unique("tablet_top_center")).order(egui::Order::Middle).pivot(Align2::CENTER_TOP).fixed_pos(center_pos).show(
        ctx,
        |ui| {
            let in_sketch = app.session.active_sketch.is_some();
            let frame = container.frame(ctx, t.overlay, crate::frame::Side::Left).inner_margin(egui::Margin::symmetric(12, 4));
            container.show_frame(ui, frame, screen, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = vec2(8.0, 0.0);
                    if in_sketch {
                        ui.colored_label(Color32::from_rgb(0x38, 0xbd, 0xf8), "✏️ Sketch Mode");

                        // Finish Sketch Button
                        let finish_btn = egui::Button::new(egui::RichText::new("✓ Finish").color(Color32::WHITE).strong())
                            .fill(Color32::from_rgb(0x16, 0xa3, 0x4a))
                            .min_size(vec2(80.0, 36.0));
                        if ui.add(finish_btn).backing_on_hover_text("Finish Sketch and Save").clicked() {
                            app.start_pinned("sketch.finish", None);
                        }

                        // Cancel Sketch Button
                        let cancel_btn =
                            egui::Button::new(egui::RichText::new("✕ Cancel").color(t.text_dim)).fill(t.panel).min_size(vec2(70.0, 36.0));
                        if ui.add(cancel_btn).backing_on_hover_text("Exit Sketch").clicked() {
                            if !crate::toolbar::cancel_unfinished(app) {
                                app.start_pinned("sketch.cancel", None);
                            }
                        }
                    } else {
                        ui.colored_label(t.text, "📦 3D Part Modeling");
                    }
                });
            });
        },
    );

    // --- Right Pill: Fit, View Presets, Items Drawer, Desktop Toggle ---
    let right_pos = pos2(screen.right() - 16.0, top_margin);
    container.area(egui::Id::unique("tablet_top_right")).order(egui::Order::Middle).pivot(Align2::RIGHT_TOP).fixed_pos(right_pos).show(ctx, |ui| {
        let frame = container.frame(ctx, t.overlay, crate::frame::Side::Right).inner_margin(egui::Margin::symmetric(8, 4));
        container.show_frame(ui, frame, screen, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = vec2(4.0, 0.0);

                // Fit View Button
                let fit_btn = egui::Button::new("⛶ Fit").min_size(vec2(50.0, bar_height - 12.0));
                if ui.add(fit_btn).backing_on_hover_text("Fit all geometry to view").clicked() {
                    let _ = app.run("view.fit", json!({}));
                }

                // Camera Views Menu
                let views_label = if st.show_views_menu { "View ▲" } else { "View ▼" };
                if ui.button(views_label).backing_on_hover_text("Camera Presets").clicked() {
                    st.show_views_menu = !st.show_views_menu;
                }

                // Items / Tree Drawer Toggle
                let tree_active = st.show_items_drawer;
                let tree_btn = egui::Button::new(if tree_active { "📑 Items (On)" } else { "📑 Items" })
                    .fill(if tree_active { t.accent } else { t.panel })
                    .min_size(vec2(70.0, bar_height - 12.0));
                if ui.add(tree_btn).backing_on_hover_text("Toggle Features & Bodies Tree").clicked() {
                    st.show_items_drawer = !st.show_items_drawer;
                }

                // Desktop / Tablet mode toggle
                if ui.button("💻 Desktop").backing_on_hover_text("Switch to Desktop UI layout").clicked() {
                    app.preferences.tablet_ui = false;
                }
            });
        });
    });

    // --- Floating Views Dropdown Menu ---
    if st.show_views_menu {
        container
            .area(egui::Id::unique("tablet_views_dropdown"))
            .order(egui::Order::Foreground)
            .pivot(Align2::RIGHT_TOP)
            .fixed_pos(pos2(screen.right() - 80.0, top_margin + bar_height + 4.0))
            .show(ctx, |ui| {
                let frame = container.frame(ctx, t.panel, crate::frame::Side::Right).inner_margin(egui::Margin::same(8));
                container.show_frame(ui, frame, screen, |ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing = vec2(0.0, 4.0);
                        let presets = [
                            ("Isometric", "view.isometric"),
                            ("Top", "view.top"),
                            ("Front", "view.front"),
                            ("Right", "view.right"),
                            ("Bottom", "view.bottom"),
                            ("Left", "view.left"),
                        ];
                        for (label, cmd) in presets {
                            if ui.add(egui::Button::new(label).min_size(vec2(110.0, 32.0))).clicked() {
                                let _ = app.run(cmd, json!({}));
                                st.show_views_menu = false;
                            }
                        }
                    });
                });
            });
    }
}

/// Floating Vertical Tool Dock on the Left Side (Shapr3D Style)
fn show_tool_dock(app: &mut SolveApp, ctx: &egui::Context, screen: Rect) {
    let t = Tokens::get();
    let container = Container::new(ctx, Surface::Navigation);
    let dock_pos = pos2(16.0, 72.0);

    let in_sketch = app.session.active_sketch.is_some();

    struct DockTool {
        cmd: &'static str,
        icon: &'static str,
        label: &'static str,
    }

    let tools: &[DockTool] = if in_sketch {
        &[
            DockTool { cmd: "sketch.line", icon: "line", label: "Line" },
            DockTool { cmd: "sketch.rectangle.two_point", icon: "rect", label: "Rectangle" },
            DockTool { cmd: "sketch.circle", icon: "circle", label: "Circle" },
            DockTool { cmd: "sketch.arc", icon: "arc", label: "Arc" },
            DockTool { cmd: "sketch.spline", icon: "spline", label: "Spline" },
            DockTool { cmd: "sketch.dim", icon: "dimension", label: "Dimension" },
            DockTool { cmd: "sketch.trim", icon: "trim", label: "Trim" },
            DockTool { cmd: "sketch.offset", icon: "offset", label: "Offset" },
        ]
    } else {
        &[
            DockTool { cmd: "sketch.new", icon: "sketch", label: "New Sketch" },
            DockTool { cmd: "solid.extrude", icon: "extrude", label: "Extrude" },
            DockTool { cmd: "solid.revolve", icon: "revolve", label: "Revolve" },
            DockTool { cmd: "solid.fillet", icon: "fillet", label: "Fillet" },
            DockTool { cmd: "solid.chamfer", icon: "chamfer", label: "Chamfer" },
            DockTool { cmd: "solid.hole", icon: "hole", label: "Hole" },
            DockTool { cmd: "solid.shell", icon: "shell", label: "Shell" },
            DockTool { cmd: "inspect.measure", icon: "measure", label: "Measure" },
        ]
    };

    container.area(egui::Id::unique("tablet_side_dock")).order(egui::Order::Middle).fixed_pos(dock_pos).show(ctx, |ui| {
        let frame = container.frame(ctx, t.overlay, crate::frame::Side::Left).inner_margin(egui::Margin::same(6));
        container.show_frame(ui, frame, screen, |ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, 6.0);

                for tool in tools {
                    let is_active = app.tool.as_ref().is_some_and(|active| active.cmd == tool.cmd);
                    let (rect, _) = ui.allocate_exact_size(vec2(DOCK_BTN_SIZE, DOCK_BTN_SIZE), Sense::click());
                    let resp = ui.interact(rect, ui.scope_id().with(tool.cmd), Sense::click());

                    let bg_color = if is_active {
                        t.accent
                    } else if resp.hovered() {
                        t.hover
                    } else {
                        Color32::TRANSPARENT
                    };

                    if bg_color != Color32::TRANSPARENT {
                        ui.painter().rect_filled(rect, t.radius_md, bg_color);
                    }

                    // Paint tool icon
                    crate::icons::paint_icon(ui.painter(), rect.shrink(t.space_8), tool.icon, 1.0);

                    if resp.backing_on_hover_text(tool.label).clicked() {
                        if is_active {
                            crate::tools::finish(app);
                        } else {
                            app.start(tool.cmd);
                        }
                    }
                }
            });
        });
    });
}

/// Slide-Over Items / Feature Tree Drawer on the Right Edge
fn show_items_drawer(app: &mut SolveApp, ctx: &egui::Context, screen: Rect, st: &mut TabletUiState) {
    let t = Tokens::get();
    let container = Container::new(ctx, Surface::Panel);
    let drawer_width = 320.0_f32.min(screen.width() * 0.4);
    let drawer_rect = Rect::from_min_size(pos2(screen.right() - drawer_width - 16.0, 72.0), vec2(drawer_width, screen.height() - 88.0));

    container.area(egui::Id::unique("tablet_items_drawer")).order(egui::Order::Middle).fixed_pos(drawer_rect.min).show(ctx, |ui| {
        let frame = container.frame(ctx, t.panel, crate::frame::Side::Right).inner_margin(egui::Margin::same(10));
        container.show_frame(ui, frame, screen, |ui| {
            ui.set_width(drawer_rect.width() - t.space_16);
            ui.set_height(drawer_rect.height() - t.space_16);

            ui.horizontal(|ui| {
                ui.strong("📑 Items & Features");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✕ Close").backing_on_hover_text("Close Items Drawer").clicked() {
                        st.show_items_drawer = false;
                    }
                });
            });
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                crate::browser::panel_contents(app, ui);
            });
        });
    });
}

/// Floating On-Screen Numeric Keypad for Dimension Input
fn show_numeric_keypad(app: &mut SolveApp, ctx: &egui::Context, screen: Rect, st: &mut TabletUiState) {
    let t = Tokens::get();
    let container = Container::new(ctx, Surface::Palette);
    let pad_pos = pos2(screen.right() - 260.0, screen.bottom() - 320.0);

    container.area(egui::Id::unique("tablet_numeric_keypad")).order(egui::Order::Foreground).fixed_pos(pad_pos).show(ctx, |ui| {
        let frame = container.frame(ctx, t.panel, crate::frame::Side::Right).inner_margin(egui::Margin::same(8));
        container.show_frame(ui, frame, screen, |ui| {
            ui.set_width(240.0);
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&st.keypad_value).monospace().size(18.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("✕").clicked() {
                            st.keypad_active = false;
                            st.keypad_value.clear();
                        }
                    });
                });
                ui.separator();

                let rows = [["7", "8", "9"], ["4", "5", "6"], ["1", "2", "3"], ["±", "0", "."]];

                for row in rows {
                    ui.horizontal(|ui| {
                        for key in row {
                            let btn = egui::Button::new(egui::RichText::new(key).size(16.0)).min_size(vec2(72.0, 44.0));
                            if ui.add(btn).clicked() {
                                if key == "±" {
                                    if st.keypad_value.starts_with('-') {
                                        st.keypad_value.remove(0);
                                    } else {
                                        st.keypad_value.insert(0, '-');
                                    }
                                } else {
                                    st.keypad_value.push_str(key);
                                }
                            }
                        }
                    });
                }

                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new("⌫ Back").min_size(vec2(108.0, 44.0))).clicked() {
                        st.keypad_value.pop();
                    }
                    let enter_btn = egui::Button::new(egui::RichText::new("✓ OK").color(Color32::WHITE).strong())
                        .fill(Color32::from_rgb(0x16, 0xa3, 0x4a))
                        .min_size(vec2(108.0, 44.0));
                    if ui.add(enter_btn).clicked() {
                        if let Ok(val) = st.keypad_value.parse::<f64>() {
                            let _ = app.run("sketch.dim.set", json!({ "value": val }));
                        }
                        st.keypad_active = false;
                        st.keypad_value.clear();
                    }
                });
            });
        });
    });
}
