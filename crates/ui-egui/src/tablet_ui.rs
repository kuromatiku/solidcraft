//! Full iPad/Tablet-native UI shell for SolidCraft.
//!
//! Provides a modern, touch-first CAD experience inspired by tablet ergonomics:
//! - Full-bleed edge-to-edge 3D canvas with zero desktop chrome clutter.
//! - Floating island Top Bar with large touch targets for Project Menu (New, Open, Save, Export),
//!   Undo, Redo, Mode Status, Fit View, and Drawer toggles.
//! - Vertical Floating Tool Dock (Shapr3D style) on the left edge with large 46×46pt touch targets,
//!   featuring expandable tool categories/families (Rectangles, Circles, Arcs, Splines, Modify, Constraints, etc.).
//! - Compact ergonomic Action Bar centered at the bottom (`screen.center().x, screen.bottom() - 36.0`)
//!   for quick [✓ Selesai] and [✕ Batal] thumb/pencil taps.
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
    pub show_project_menu: bool,
    pub show_views_menu: bool,
    pub show_items_drawer: bool,
    pub expanded_dock_group: Option<usize>,
    pub suppress_click_for_group: Option<usize>,
    pub keypad_value: String,
    pub keypad_active: bool,
    pub chosen_sketch_line: Option<String>,
    pub chosen_sketch_rect: Option<String>,
    pub chosen_sketch_circle: Option<String>,
    pub chosen_sketch_curve: Option<String>,
    pub chosen_sketch_modify: Option<String>,
    pub chosen_sketch_constraint: Option<String>,
    pub chosen_solid_create: Option<String>,
    pub chosen_solid_modify: Option<String>,
    pub chosen_solid_pattern: Option<String>,
    pub chosen_inspect: Option<String>,
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
    show_tool_dock(app, &ctx, screen, &mut st);
    show_bottom_action_bar(app, &ctx, screen);
    show_bottom_left_bar(app, &ctx, screen);
    if st.show_project_menu {
        show_project_menu(app, &ctx, screen, &mut st);
    }
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

    // --- Left Pill: Project Menu Trigger, Undo, Redo ---
    let left_pos = pos2(16.0, top_margin);
    container.area(egui::Id::unique("tablet_top_left")).order(egui::Order::Middle).fixed_pos(left_pos).show(ctx, |ui| {
        let frame = container.frame(ctx, t.overlay, crate::frame::Side::Left).inner_margin(egui::Margin::symmetric(8, 4));
        container.show_frame(ui, frame, screen, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = vec2(6.0, 0.0);

                // Project & File Menu Button
                let doc_name = app.session.doc.name.clone();
                let display_name = if doc_name.is_empty() { "SolidCraft".to_string() } else { doc_name };
                let project_btn_text = format!("📄 {} ▾", display_name);
                let project_btn = egui::Button::new(egui::RichText::new(project_btn_text).strong().size(13.5).color(if st.show_project_menu {
                    Color32::WHITE
                } else {
                    t.text
                }))
                .fill(if st.show_project_menu { t.accent } else { Color32::TRANSPARENT })
                .min_size(vec2(100.0, TOUCH_TARGET - 8.0));
                if ui.add(project_btn).backing_on_hover_text("Buka menu proyek (Desain Baru, Buka, Simpan, Ekspor STEP/STL/3MF/DXF/OBJ)").clicked() {
                    st.show_project_menu = !st.show_project_menu;
                    st.expanded_dock_group = None;
                    st.show_views_menu = false;
                }
            });
        });
    });

    // --- Center Pill: Mode Indicator & Quick Context Action ---
    let center_pos = pos2(screen.center().x, top_margin);
    container.area(egui::Id::unique("tablet_top_center")).order(egui::Order::Middle).pivot(Align2::CENTER_TOP).fixed_pos(center_pos).show(
        ctx,
        |ui| {
            let in_sketch = app.session.active_sketch.is_some();
            let frame = container.frame(ctx, t.overlay, crate::frame::Side::Left).inner_margin(egui::Margin::symmetric(12, 6));
            container.show_frame(ui, frame, screen, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = vec2(8.0, 0.0);
                    if in_sketch {
                        if app.tool.as_ref().is_some_and(|tool| tool.cmd == "sketch.trim") {
                            ui.colored_label(Color32::from_rgb(0xf4, 0x3f, 0x5e), "✂️ Trim: Tarik jari melintasi kurva untuk potong");
                            let trim_done_btn = egui::Button::new(egui::RichText::new("✓ Selesai Potong").color(Color32::WHITE).strong())
                                .fill(Color32::from_rgb(0x16, 0xa3, 0x4a))
                                .min_size(vec2(110.0, 32.0));
                            if ui.add(trim_done_btn).backing_on_hover_text("Selesaikan operasi trim").clicked() {
                                crate::tools::finish(app);
                            }
                        } else {
                            ui.colored_label(Color32::from_rgb(0x38, 0xbd, 0xf8), "✏️ Mode Sketch 2D");
                        }
                    } else if app.dialog.is_some() {
                        let is_press_pull = app.dialog.as_ref().is_some_and(|d| d.press_pull);
                        let title = if is_press_pull {
                            "🚀 Press-Pull".to_string()
                        } else {
                            format!("🛠️ {}", app.dialog.as_ref().map(|d| crate::dialogs::report_title(d)).unwrap_or_default())
                        };
                        ui.colored_label(Color32::from_rgb(0x38, 0xbd, 0xf8), title);
                    } else if app
                        .session
                        .selection
                        .first()
                        .is_some_and(|s| matches!(s, solidcraft_engine::Sel::Face { .. } | solidcraft_engine::Sel::Plane { .. }))
                    {
                        ui.colored_label(Color32::from_rgb(0x38, 0xbd, 0xf8), "📐 Face Dipilih");
                        let extrude_btn = egui::Button::new(egui::RichText::new("⬢ Extrude").color(Color32::WHITE).strong())
                            .fill(Color32::from_rgb(0x16, 0xa3, 0x4a))
                            .min_size(vec2(85.0, 32.0));
                        if ui.add(extrude_btn).backing_on_hover_text("Extrude face ini").clicked() {
                            app.start("solid.press_pull");
                        }
                        let sketch_face_btn = egui::Button::new(egui::RichText::new("✏️ Sketch on Face").color(Color32::WHITE).strong())
                            .fill(Color32::from_rgb(0x02, 0x84, 0xc7))
                            .min_size(vec2(115.0, 32.0));
                        if ui.add(sketch_face_btn).backing_on_hover_text("Buka sketch 2D di planar face ini").clicked() {
                            app.start("sketch.create");
                        }
                    } else if app
                        .session
                        .selection
                        .first()
                        .is_some_and(|s| matches!(s, solidcraft_engine::Sel::Profile { .. } | solidcraft_engine::Sel::ExternalSketchCurve { .. }))
                    {
                        ui.colored_label(Color32::from_rgb(0x38, 0xbd, 0xf8), "⭕ Profile Dipilih");
                        let extrude_btn = egui::Button::new(egui::RichText::new("⬢ Extrude").color(Color32::WHITE).strong())
                            .fill(Color32::from_rgb(0x16, 0xa3, 0x4a))
                            .min_size(vec2(90.0, 32.0));
                        if ui.add(extrude_btn).backing_on_hover_text("Extrude profile sketch ini menjadi 3D").clicked() {
                            if let Some(solidcraft_engine::Sel::ExternalSketchCurve { sketch, .. }) = app.session.selection.first() {
                                let n = crate::open_meshing::session_world(&app.session).sketch(*sketch).map_or(0, |sk| sk.profiles.len());
                                if n > 0 {
                                    let items: Vec<serde_json::Value> =
                                        (0..n).map(|i| serde_json::json!({"type": "profile", "sketch": sketch, "index": i})).collect();
                                    let _ = app.run("select.set", serde_json::json!({ "items": items }));
                                }
                            }
                            app.start("solid.extrude");
                        }
                    } else {
                        ui.colored_label(t.text, "📦 3D Part Modeling");
                    }

                    // Quick Radial Wheel Menu access
                    let radial_btn = egui::Button::new(egui::RichText::new("⚡ Menu").color(Color32::WHITE).strong())
                        .fill(Color32::from_rgb(0x33, 0x41, 0x55))
                        .min_size(vec2(64.0, 32.0));
                    if ui.add(radial_btn).backing_on_hover_text("Buka Roda Aksi Radial (⚡)").clicked() {
                        app.preferences.radial_context_menu = true;
                        app.menu.radial = true;
                        crate::context_menu::open_for(app, screen.center(), crate::context_menu::Target::Viewport);
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
                if ui.add(fit_btn).backing_on_hover_text("Pusatkan seluruh geometri ke layar").clicked() {
                    let _ = app.run("view.fit", json!({}));
                }

                // Camera Views Menu
                let views_label = if st.show_views_menu { "View ▲" } else { "View ▼" };
                if ui.button(views_label).backing_on_hover_text("Preset Arah Sudut Kamera").clicked() {
                    st.show_views_menu = !st.show_views_menu;
                    st.show_project_menu = false;
                }

                // Items / Tree Drawer Toggle
                let tree_active = st.show_items_drawer;
                let tree_btn = egui::Button::new(if tree_active { "📑 Items (On)" } else { "📑 Items" })
                    .fill(if tree_active { t.accent } else { t.panel })
                    .min_size(vec2(70.0, bar_height - 12.0));
                if ui.add(tree_btn).backing_on_hover_text("Toggle Panel Features & Bodies").clicked() {
                    st.show_items_drawer = !st.show_items_drawer;
                }

                // Desktop / Tablet mode toggle
                if ui.button("💻 Desktop").backing_on_hover_text("Beralih ke tampilan desktop biasa").clicked() {
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

/// Project & File Popover Menu (New, Open, Save, Export STEP/STL/3MF/DXF/OBJ, Settings)
fn show_project_menu(app: &mut SolveApp, ctx: &egui::Context, screen: Rect, st: &mut TabletUiState) {
    let t = Tokens::get();
    let container = Container::new(ctx, Surface::Palette);
    let menu_pos = pos2(16.0, 12.0 + 48.0 + 4.0);

    container.area(egui::Id::unique("tablet_project_menu_popover")).order(egui::Order::Foreground).fixed_pos(menu_pos).show(ctx, |ui| {
        let frame = container.frame(ctx, t.panel, crate::frame::Side::Left).inner_margin(egui::Margin::same(12));
        container.show_frame(ui, frame, screen, |ui| {
            ui.set_width(260.0);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, 6.0);

                // Header
                ui.horizontal(|ui| {
                    let doc_name = app.session.doc.name.clone();
                    let display_name = if doc_name.is_empty() { "SolidCraft Design" } else { &doc_name };
                    ui.strong(format!("📁 {}", display_name));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("✕").clicked() {
                            st.show_project_menu = false;
                        }
                    });
                });
                ui.separator();

                // Proyek
                ui.colored_label(t.text_dim, "PROYEK");
                if ui.add(egui::Button::new("📄 Desain Baru (New)").min_size(vec2(240.0, 36.0))).clicked() {
                    crate::toolbar::FileAction::New.run(app);
                    st.show_project_menu = false;
                }
                if ui.add(egui::Button::new("📂 Buka Desain (Open)").min_size(vec2(240.0, 36.0))).clicked() {
                    crate::toolbar::FileAction::Open.run(app);
                    st.show_project_menu = false;
                }
                if ui.add(egui::Button::new("💾 Simpan (Save)").min_size(vec2(240.0, 36.0))).clicked() {
                    crate::toolbar::FileAction::Save.run(app);
                    st.show_project_menu = false;
                }
                if ui.add(egui::Button::new("💾 Simpan Sebagai... (Save As)").min_size(vec2(240.0, 36.0))).clicked() {
                    crate::toolbar::FileAction::SaveAs.run(app);
                    st.show_project_menu = false;
                }

                ui.separator();

                // Ekspor File
                ui.colored_label(t.text_dim, "EKSPOR FILE");
                if ui.add(egui::Button::new("📤 Ekspor STEP (.step)").min_size(vec2(240.0, 34.0))).clicked() {
                    crate::toolbar::FileAction::Export("step").run(app);
                    st.show_project_menu = false;
                }
                if ui.add(egui::Button::new("📤 Ekspor Mesh STL (.stl)").min_size(vec2(240.0, 34.0))).clicked() {
                    crate::toolbar::FileAction::ExportMesh("stl").run(app);
                    st.show_project_menu = false;
                }
                if ui.add(egui::Button::new("📤 Ekspor 3MF (.3mf)").min_size(vec2(240.0, 34.0))).clicked() {
                    crate::toolbar::FileAction::ExportMesh("3mf").run(app);
                    st.show_project_menu = false;
                }
                if ui.add(egui::Button::new("📤 Ekspor OBJ (.obj)").min_size(vec2(240.0, 34.0))).clicked() {
                    crate::toolbar::FileAction::Export("obj").run(app);
                    st.show_project_menu = false;
                }
                if ui.add(egui::Button::new("📤 Ekspor 2D DXF (.dxf)").min_size(vec2(240.0, 34.0))).clicked() {
                    crate::toolbar::FileAction::Command("file.export_view_dxf").run(app);
                    st.show_project_menu = false;
                }

                ui.separator();

                // Pengaturan
                ui.colored_label(t.text_dim, "PENGATURAN");
                if ui.add(egui::Button::new("⚙️ Pengaturan (Settings)").min_size(vec2(240.0, 34.0))).clicked() {
                    crate::toolbar::FileAction::Settings.run(app);
                    st.show_project_menu = false;
                }
                if ui.add(egui::Button::new("💻 Beralih ke Desktop UI").min_size(vec2(240.0, 34.0))).clicked() {
                    app.preferences.tablet_ui = false;
                    st.show_project_menu = false;
                }
            });
        });
    });
}

struct DockToolVariant {
    cmd: &'static str,
    label: &'static str,
    icon: &'static str,
}

struct DockGroup {
    id: usize,
    title: &'static str,
    variants: &'static [DockToolVariant],
}

const SKETCH_GROUPS: &[DockGroup] = &[
    DockGroup {
        id: 0,
        title: "Lines & Construction",
        variants: &[
            DockToolVariant { cmd: "sketch.line", label: "2-Point Line", icon: "line" },
            DockToolVariant { cmd: "sketch.line.midpoint", label: "Midpoint Line", icon: "line" },
            DockToolVariant { cmd: "sketch.construction_line", label: "Construction Line", icon: "line" },
        ],
    },
    DockGroup {
        id: 1,
        title: "Rectangles",
        variants: &[
            DockToolVariant { cmd: "sketch.rectangle.two_point", label: "Corner Rectangle", icon: "rect" },
            DockToolVariant { cmd: "sketch.rectangle.center", label: "Center Rectangle", icon: "rect" },
            DockToolVariant { cmd: "sketch.rectangle.three_point", label: "3-Point Rectangle", icon: "rect" },
            DockToolVariant { cmd: "sketch.rectangle.rounded", label: "Rounded Rectangle", icon: "rect" },
        ],
    },
    DockGroup {
        id: 2,
        title: "Circles & Arcs",
        variants: &[
            DockToolVariant { cmd: "sketch.circle.center", label: "Center Circle", icon: "circle" },
            DockToolVariant { cmd: "sketch.circle.two_point", label: "2-Point Circle", icon: "circle" },
            DockToolVariant { cmd: "sketch.circle.three_point", label: "3-Point Circle", icon: "circle" },
            DockToolVariant { cmd: "sketch.arc.three_point", label: "3-Point Arc", icon: "arc" },
            DockToolVariant { cmd: "sketch.arc.center_point", label: "Center Point Arc", icon: "arc" },
            DockToolVariant { cmd: "sketch.arc.tangent", label: "Tangent Arc", icon: "arc" },
        ],
    },
    DockGroup {
        id: 3,
        title: "Curves & Shapes",
        variants: &[
            DockToolVariant { cmd: "sketch.spline.fit_point", label: "Fit Point Spline", icon: "spline" },
            DockToolVariant { cmd: "sketch.spline.control_point", label: "Control Point Spline", icon: "spline" },
            DockToolVariant { cmd: "sketch.ellipse", label: "Ellipse", icon: "circle" },
            DockToolVariant { cmd: "sketch.polygon.inscribed", label: "Inscribed Polygon", icon: "rect" },
            DockToolVariant { cmd: "sketch.slot.center_to_center", label: "Center Slot", icon: "rect" },
            DockToolVariant { cmd: "sketch.point", label: "Point", icon: "point" },
        ],
    },
    DockGroup { id: 4, title: "Dimension", variants: &[DockToolVariant { cmd: "sketch.dimension", label: "Dimension", icon: "dimension" }] },
    DockGroup {
        id: 5,
        title: "Modify & Edit",
        variants: &[
            DockToolVariant { cmd: "sketch.trim", label: "Trim", icon: "trim" },
            DockToolVariant { cmd: "sketch.extend", label: "Extend", icon: "trim" },
            DockToolVariant { cmd: "sketch.offset", label: "Offset", icon: "offset" },
            DockToolVariant { cmd: "sketch.fillet", label: "Sketch Fillet", icon: "fillet" },
            DockToolVariant { cmd: "sketch.chamfer.equal_distance", label: "Sketch Chamfer", icon: "chamfer" },
            DockToolVariant { cmd: "sketch.mirror", label: "Mirror", icon: "mirror" },
        ],
    },
    DockGroup {
        id: 6,
        title: "Constraints",
        variants: &[
            DockToolVariant { cmd: "sketch.constraint.horizontal_vertical", label: "Horiz / Vert", icon: "constraint" },
            DockToolVariant { cmd: "sketch.constraint.coincident", label: "Coincident", icon: "constraint" },
            DockToolVariant { cmd: "sketch.constraint.tangent", label: "Tangent", icon: "constraint" },
            DockToolVariant { cmd: "sketch.constraint.equal", label: "Equal", icon: "constraint" },
            DockToolVariant { cmd: "sketch.constraint.perpendicular", label: "Perpendicular", icon: "constraint" },
            DockToolVariant { cmd: "sketch.constraint.parallel", label: "Parallel", icon: "constraint" },
            DockToolVariant { cmd: "sketch.constraint.concentric", label: "Concentric", icon: "constraint" },
            DockToolVariant { cmd: "sketch.constraint.midpoint", label: "Midpoint", icon: "constraint" },
            DockToolVariant { cmd: "sketch.constraint.fix", label: "Fix / Lock", icon: "constraint" },
        ],
    },
    DockGroup {
        id: 7,
        title: "Normal/Construction Toggle",
        variants: &[DockToolVariant { cmd: "sketch.construction", label: "Toggle Normal/Construction", icon: "construction" }],
    },
];

const MODEL_GROUPS: &[DockGroup] = &[
    DockGroup { id: 10, title: "Sketch", variants: &[DockToolVariant { cmd: "sketch.create", label: "New Sketch", icon: "sketch" }] },
    DockGroup {
        id: 11,
        title: "3D Create",
        variants: &[
            DockToolVariant { cmd: "solid.extrude", label: "Extrude", icon: "extrude" },
            DockToolVariant { cmd: "solid.revolve", label: "Revolve", icon: "revolve" },
            DockToolVariant { cmd: "solid.sweep", label: "Sweep", icon: "sweep" },
            DockToolVariant { cmd: "solid.loft", label: "Loft", icon: "loft" },
            DockToolVariant { cmd: "solid.hole", label: "Hole", icon: "hole" },
            DockToolVariant { cmd: "solid.thread", label: "Thread", icon: "thread" },
        ],
    },
    DockGroup {
        id: 12,
        title: "3D Modify",
        variants: &[
            DockToolVariant { cmd: "solid.fillet", label: "Fillet", icon: "fillet" },
            DockToolVariant { cmd: "solid.chamfer", label: "Chamfer", icon: "chamfer" },
            DockToolVariant { cmd: "solid.shell", label: "Shell", icon: "shell" },
            DockToolVariant { cmd: "solid.draft", label: "Draft", icon: "draft" },
            DockToolVariant { cmd: "solid.split_body", label: "Split Body", icon: "split" },
            DockToolVariant { cmd: "solid.combine", label: "Combine / Boolean", icon: "combine" },
        ],
    },
    DockGroup {
        id: 13,
        title: "Patterns & Arrange",
        variants: &[
            DockToolVariant { cmd: "solid.mirror", label: "Mirror", icon: "mirror" },
            DockToolVariant { cmd: "solid.pattern.rectangular", label: "Rectangular Pattern", icon: "pattern_rectangular" },
            DockToolVariant { cmd: "solid.pattern.circular", label: "Circular Pattern", icon: "pattern_circular" },
        ],
    },
    DockGroup {
        id: 14,
        title: "Construct Datum",
        variants: &[DockToolVariant { cmd: "construct.plane.offset", label: "Offset Plane", icon: "plane_offset" }],
    },
    DockGroup {
        id: 15,
        title: "Inspect",
        variants: &[
            DockToolVariant { cmd: "inspect.measure", label: "Measure", icon: "measure" },
            DockToolVariant { cmd: "inspect.section", label: "Section Analysis", icon: "section" },
        ],
    },
];

fn get_chosen_cmd<'a>(st: &'a TabletUiState, group: &DockGroup) -> &'static str {
    let saved = match group.id {
        0 => st.chosen_sketch_line.as_deref(),
        1 => st.chosen_sketch_rect.as_deref(),
        2 => st.chosen_sketch_circle.as_deref(),
        3 => st.chosen_sketch_curve.as_deref(),
        5 => st.chosen_sketch_modify.as_deref(),
        6 => st.chosen_sketch_constraint.as_deref(),
        11 => st.chosen_solid_create.as_deref(),
        12 => st.chosen_solid_modify.as_deref(),
        13 => st.chosen_solid_pattern.as_deref(),
        15 => st.chosen_inspect.as_deref(),
        _ => None,
    };
    saved
        .and_then(|cmd| group.variants.iter().find(|v| v.cmd == cmd).map(|v| v.cmd))
        .unwrap_or_else(|| group.variants.first().map(|v| v.cmd).unwrap_or(""))
}

fn set_chosen_cmd(st: &mut TabletUiState, group_id: usize, cmd: &str) {
    let val = Some(cmd.to_string());
    match group_id {
        0 => st.chosen_sketch_line = val,
        1 => st.chosen_sketch_rect = val,
        2 => st.chosen_sketch_circle = val,
        3 => st.chosen_sketch_curve = val,
        5 => st.chosen_sketch_modify = val,
        6 => st.chosen_sketch_constraint = val,
        11 => st.chosen_solid_create = val,
        12 => st.chosen_solid_modify = val,
        13 => st.chosen_solid_pattern = val,
        15 => st.chosen_inspect = val,
        _ => {}
    }
}

/// Floating Vertical Tool Dock on the Left Side with Expandable Tool Categories
fn show_tool_dock(app: &mut SolveApp, ctx: &egui::Context, screen: Rect, st: &mut TabletUiState) {
    let t = Tokens::get();
    let container = Container::new(ctx, Surface::Navigation);
    let dock_pos = pos2(16.0, 72.0);

    let in_sketch = app.session.active_sketch.is_some();
    let groups: &[DockGroup] = if in_sketch { SKETCH_GROUPS } else { MODEL_GROUPS };

    let mut opened_flyout_pos = None;
    let mut opened_flyout_group: Option<&'static DockGroup> = None;

    container.area(egui::Id::unique("tablet_side_dock")).order(egui::Order::Middle).fixed_pos(dock_pos).show(ctx, |ui| {
        let frame = container.frame(ctx, t.overlay, crate::frame::Side::Left).inner_margin(egui::Margin::same(6));
        container.show_frame(ui, frame, screen, |ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, 6.0);

                for group in groups {
                    let active_cmd = get_chosen_cmd(st, group);
                    let active_variant = group.variants.iter().find(|v| v.cmd == active_cmd).unwrap_or(&group.variants[0]);

                    let is_running = app.tool.as_ref().is_some_and(|active| group.variants.iter().any(|v| v.cmd == active.cmd))
                        || app.dialog.as_ref().and_then(|d| d.command()).is_some_and(|cmd| group.variants.iter().any(|v| v.cmd == cmd));

                    let is_flyout_open = st.expanded_dock_group == Some(group.id);

                    let (rect, _) = ui.allocate_exact_size(vec2(DOCK_BTN_SIZE, DOCK_BTN_SIZE), Sense::click());
                    let resp = ui.interact(rect, ui.scope_id().with(format!("dock_btn_{}", group.id)), Sense::click());

                    let bg_color = if is_running || is_flyout_open {
                        t.accent
                    } else if resp.hovered() {
                        t.hover
                    } else {
                        Color32::TRANSPARENT
                    };

                    if bg_color != Color32::TRANSPARENT {
                        ui.painter().rect_filled(rect, t.radius_md, bg_color);
                    }

                    // Paint icon
                    let icon_name = solidcraft_engine::find_command(active_variant.cmd).map_or(active_variant.icon, |s| s.icon);
                    crate::icons::paint_icon(ui.painter(), rect.shrink(t.space_8), icon_name, 1.0);

                    // If group has multiple variants, paint a small corner chevron indicator
                    if group.variants.len() > 1 {
                        let tri_c = rect.right_bottom() - vec2(5.0, 4.0);
                        ui.painter().text(
                            tri_c,
                            Align2::RIGHT_BOTTOM,
                            "▾",
                            egui::FontId::proportional(11.0),
                            if is_running || is_flyout_open { Color32::WHITE } else { t.text_dim },
                        );
                    }

                    if is_flyout_open {
                        opened_flyout_pos = Some(pos2(dock_pos.x + DOCK_BTN_SIZE + 12.0, rect.min.y));
                        opened_flyout_group = Some(group);
                    }

                    // Hold detection: holding for >= 0.35s opens the variant flyout
                    let (is_held, is_down_here) = ctx.input(|i| {
                        if i.pointer.primary_down() {
                            if let (Some(origin), Some(start)) = (i.pointer.press_origin(), i.pointer.press_start_time()) {
                                if rect.contains(origin) {
                                    return (i.time - start >= 0.35, true);
                                }
                            }
                        }
                        (false, false)
                    });

                    if is_down_here {
                        ctx.request_repaint_after(std::time::Duration::from_millis(40));
                    }

                    if is_held && group.variants.len() > 1 {
                        st.expanded_dock_group = Some(group.id);
                        st.suppress_click_for_group = Some(group.id);
                    }

                    let is_clicked = resp.clicked();
                    let is_secondary = resp.secondary_clicked();
                    let _ = resp.backing_on_hover_text(format!("{} (Tahan untuk pilih varian)", active_variant.label));

                    // Click interaction:
                    // If click was triggered by releasing after hold, consume and suppress it.
                    // Otherwise, single tap ALWAYS starts/toggles the active tool directly!
                    if is_clicked {
                        let was_suppressed = st.suppress_click_for_group.take() == Some(group.id);
                        if !was_suppressed {
                            st.expanded_dock_group = None;
                            if is_running {
                                crate::tools::finish(app);
                            } else {
                                app.start(active_variant.cmd);
                            }
                        }
                    } else if !ctx.input(|i| i.pointer.primary_down()) && st.suppress_click_for_group == Some(group.id) {
                        st.suppress_click_for_group = None;
                    }

                    // Secondary click (desktop right click) toggles flyout immediately
                    if is_secondary && group.variants.len() > 1 {
                        st.expanded_dock_group = if st.expanded_dock_group == Some(group.id) { None } else { Some(group.id) };
                    }
                }

                // Divider
                let (div_rect, _) = ui.allocate_exact_size(vec2(DOCK_BTN_SIZE, 1.0), Sense::hover());
                ui.painter().rect_filled(div_rect, 0.0, t.border);

                // Quick Radial Wheel button
                let (radial_rect, _) = ui.allocate_exact_size(vec2(DOCK_BTN_SIZE, DOCK_BTN_SIZE), Sense::click());
                let radial_resp = ui.interact(radial_rect, ui.scope_id().with("tablet_dock_radial"), Sense::click());
                let radial_bg = if app.menu.radial {
                    t.accent
                } else if radial_resp.hovered() {
                    t.hover
                } else {
                    Color32::from_rgb(0x33, 0x41, 0x55)
                };
                ui.painter().rect_filled(radial_rect, t.radius_md, radial_bg);
                ui.painter().text(radial_rect.center(), Align2::CENTER_CENTER, "⚡", egui::FontId::proportional(20.0), Color32::WHITE);
                if radial_resp.backing_on_hover_text("Aksi Radial Cepat (⚡)").clicked() {
                    app.menu.radial = true;
                    crate::context_menu::open_for(app, screen.center(), crate::context_menu::Target::Viewport);
                }
            });
        });
    });

    // Render flyout popover for expanded group
    if let (Some(fly_pos), Some(group)) = (opened_flyout_pos, opened_flyout_group) {
        let current_chosen = get_chosen_cmd(st, group);
        let group_id = group.id;

        container.area(egui::Id::unique("tablet_dock_flyout")).order(egui::Order::Foreground).fixed_pos(fly_pos).show(ctx, |ui| {
            let frame = container.frame(ctx, t.panel, crate::frame::Side::Left).inner_margin(egui::Margin::same(10));
            container.show_frame(ui, frame, screen, |ui| {
                ui.set_width(210.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing = vec2(0.0, 4.0);
                    ui.horizontal(|ui| {
                        ui.strong(group.title);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("✕").clicked() {
                                st.expanded_dock_group = None;
                            }
                        });
                    });
                    ui.separator();

                    for v in group.variants {
                        let is_active = app.tool.as_ref().is_some_and(|active| active.cmd == v.cmd)
                            || app.dialog.as_ref().and_then(|d| d.command()).is_some_and(|cmd| cmd == v.cmd);
                        let is_chosen = current_chosen == v.cmd;

                        let (btn_rect, _) = ui.allocate_exact_size(vec2(210.0, 36.0), Sense::click());
                        let resp = ui.interact(btn_rect, ui.scope_id().with(format!("flyout_item_{}", v.cmd)), Sense::click());

                        let bg_color = if is_active {
                            t.accent
                        } else if is_chosen {
                            t.hover
                        } else if resp.hovered() {
                            t.hover
                        } else {
                            Color32::TRANSPARENT
                        };

                        if bg_color != Color32::TRANSPARENT {
                            ui.painter().rect_filled(btn_rect, t.radius_md, bg_color);
                        }

                        // Icon on left
                        let icon_rect = Rect::from_min_size(btn_rect.min + vec2(6.0, 6.0), vec2(24.0, 24.0));
                        let icon_name = solidcraft_engine::find_command(v.cmd).map_or(v.icon, |s| s.icon);
                        crate::icons::paint_icon(ui.painter(), icon_rect, icon_name, 1.0);

                        // Label next to icon
                        let text_pos = pos2(btn_rect.min.x + 38.0, btn_rect.center().y);
                        ui.painter().text(
                            text_pos,
                            Align2::LEFT_CENTER,
                            v.label,
                            egui::FontId::proportional(14.0),
                            if is_active { Color32::WHITE } else { t.text },
                        );

                        if resp.clicked() {
                            set_chosen_cmd(st, group_id, v.cmd);
                            app.start(v.cmd);
                            st.expanded_dock_group = None;
                        }
                    }
                });
            });
        });
    }

    // Dismiss open flyout when tapping outside both the dock and flyout
    if st.expanded_dock_group.is_some() {
        let dock_rect = Rect::from_min_size(dock_pos, vec2(DOCK_BTN_SIZE + 16.0, groups.len() as f32 * (DOCK_BTN_SIZE + 6.0) + 70.0));
        let flyout_rect = opened_flyout_pos
            .zip(opened_flyout_group)
            .map(|(pos, grp)| Rect::from_min_size(pos, vec2(230.0, grp.variants.len() as f32 * 42.0 + 50.0)));
        if let Some(press_pos) = ctx.input(|i| if i.pointer.primary_pressed() { i.pointer.press_origin() } else { None }) {
            let in_dock = dock_rect.contains(press_pos);
            let in_flyout = flyout_rect.is_some_and(|r| r.contains(press_pos));
            if !in_dock && !in_flyout {
                st.expanded_dock_group = None;
            }
        }
    }
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
                    if ui.button("✕ Close").backing_on_hover_text("Tutup Panel Items").clicked() {
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

/// Floating Action Bar at the Bottom Center (Sleek, Compact, Centered)
fn show_bottom_action_bar(app: &mut SolveApp, ctx: &egui::Context, screen: Rect) {
    let in_sketch = app.session.active_sketch.is_some();
    let has_dialog = app.dialog.is_some();
    if !in_sketch && !has_dialog {
        return;
    }

    let t = Tokens::get();
    let container = Container::new(ctx, Surface::Navigation);
    // Positioned at horizontal center, elevated slightly above bottom edge
    let bar_pos = pos2(screen.center().x, screen.bottom() - 36.0);

    let is_press_pull = app.dialog.as_ref().is_some_and(|d| d.press_pull);
    let is_fillet = app.dialog.as_ref().is_some_and(|d| matches!(d.kind, crate::dialogs::Kind::Fillet { chamfer: false, .. }));
    let is_chamfer = app.dialog.as_ref().is_some_and(|d| matches!(d.kind, crate::dialogs::Kind::Fillet { chamfer: true, .. }));

    let mut action_switch = None;
    let mut action_sketch_face = false;
    let mut action_done = false;
    let mut action_cancel = false;
    let mut action_sketch_finish = false;
    let mut action_sketch_cancel = false;

    container.area(egui::Id::unique("tablet_bottom_action_bar")).order(egui::Order::Middle).pivot(Align2::CENTER_BOTTOM).fixed_pos(bar_pos).show(
        ctx,
        |ui| {
            let frame = container.frame(ctx, t.overlay, crate::frame::Side::Left).inner_margin(egui::Margin::symmetric(10, 6));
            container.show_frame(ui, frame, screen, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = vec2(8.0, 0.0);

                    if in_sketch {
                        // Cancel Sketch Button: compact, sleek
                        let cancel_btn = egui::Button::new(egui::RichText::new("✕ Batal").color(Color32::from_rgb(0xf8, 0x71, 0x71)).size(14.0))
                            .fill(t.panel)
                            .min_size(vec2(76.0, 34.0));
                        if ui.add(cancel_btn).backing_on_hover_text("Batal dan keluar dari sketch").clicked() {
                            action_sketch_cancel = true;
                        }

                        // Finish Sketch Button: compact, sleek
                        let finish_btn = egui::Button::new(egui::RichText::new("✓ Selesai").color(Color32::WHITE).strong().size(14.0))
                            .fill(Color32::from_rgb(0x16, 0xa3, 0x4a))
                            .min_size(vec2(90.0, 34.0));
                        if ui.add(finish_btn).backing_on_hover_text("Simpan dan selesaikan sketch").clicked() {
                            action_sketch_finish = true;
                        }
                    } else if has_dialog {
                        // Fillet / Chamfer 1-tap mode switch pill
                        if is_fillet {
                            let switch_btn = egui::Button::new(egui::RichText::new("⏢ Chamfer").color(Color32::WHITE).strong().size(13.0))
                                .fill(Color32::from_rgb(0x02, 0x84, 0xc7))
                                .min_size(vec2(86.0, 34.0));
                            if ui.add(switch_btn).backing_on_hover_text("Switch ke Chamfer (Bevel)").clicked() {
                                action_switch = Some("solid.chamfer");
                            }
                        } else if is_chamfer {
                            let switch_btn = egui::Button::new(egui::RichText::new("⌒ Fillet").color(Color32::WHITE).strong().size(13.0))
                                .fill(Color32::from_rgb(0x02, 0x84, 0xc7))
                                .min_size(vec2(80.0, 34.0));
                            if ui.add(switch_btn).backing_on_hover_text("Switch ke Fillet (Round)").clicked() {
                                action_switch = Some("solid.fillet");
                            }
                        }

                        if is_press_pull {
                            let sketch_btn = egui::Button::new(egui::RichText::new("✏️ Sketch on Face").color(Color32::WHITE).strong().size(13.0))
                                .fill(Color32::from_rgb(0x02, 0x84, 0xc7))
                                .min_size(vec2(115.0, 34.0));
                            if ui.add(sketch_btn).backing_on_hover_text("Buka sketch di planar face ini").clicked() {
                                action_sketch_face = true;
                            }
                        }

                        // Cancel / Discard
                        let cancel_btn = egui::Button::new(egui::RichText::new("✕ Batal").color(Color32::from_rgb(0xf8, 0x71, 0x71)).size(14.0))
                            .fill(t.panel)
                            .min_size(vec2(76.0, 34.0));
                        if ui.add(cancel_btn).backing_on_hover_text("Batalkan modifikasi").clicked() {
                            action_cancel = true;
                        }

                        // Commit / Apply
                        let done_btn = egui::Button::new(egui::RichText::new("✓ Selesai").color(Color32::WHITE).strong().size(14.0))
                            .fill(Color32::from_rgb(0x16, 0xa3, 0x4a))
                            .min_size(vec2(86.0, 34.0));
                        if ui.add(done_btn).backing_on_hover_text("Apply dan simpan perubahan").clicked() {
                            action_done = true;
                        }
                    }
                });
            });
        },
    );

    if let Some(target_cmd) = action_switch {
        let sel = app.session.selection.clone();
        crate::dialogs::cancel(app);
        app.session.selection = sel;
        app.start(target_cmd);
    } else if action_sketch_face {
        crate::dialogs::cancel(app);
        app.start("sketch.create");
    } else if action_done {
        crate::dialogs::confirm(app);
    } else if action_cancel {
        crate::dialogs::cancel(app);
    } else if action_sketch_finish {
        let sid = app.session.active_sketch;
        app.start_pinned("sketch.finish", None);
        if let Some(id) = sid {
            let n = crate::open_meshing::session_world(&app.session).sketch(id).map_or(0, |s| s.profiles.len());
            if n > 0 {
                let items: Vec<serde_json::Value> = (0..n).map(|i| serde_json::json!({"type": "profile", "sketch": id, "index": i})).collect();
                let _ = app.run("select.set", serde_json::json!({ "items": items }));
            }
        }
    } else if action_sketch_cancel {
        if !crate::toolbar::cancel_unfinished(app) {
            app.start_pinned("sketch.cancel", None);
        }
    }
}

/// Floating Bottom-Left Bar: Undo, Redo, and Quick Radial Menu (Ergonomic left thumb reach)
fn show_bottom_left_bar(app: &mut SolveApp, ctx: &egui::Context, screen: Rect) {
    let t = Tokens::get();
    let container = Container::new(ctx, Surface::Navigation);
    let bar_pos = pos2(16.0, screen.bottom() - 36.0);

    let can_undo = !app.session.undo.is_empty();
    let can_redo = !app.session.redo.is_empty();

    let mut do_undo = false;
    let mut do_redo = false;
    let mut do_quick_menu = false;

    container.area(egui::Id::unique("tablet_bottom_left")).order(egui::Order::Middle).pivot(Align2::LEFT_BOTTOM).fixed_pos(bar_pos).show(ctx, |ui| {
        let frame = container.frame(ctx, t.overlay, crate::frame::Side::Left).inner_margin(egui::Margin::symmetric(8, 4));
        container.show_frame(ui, frame, screen, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = vec2(6.0, 0.0);

                // Undo Button
                let (undo_rect, _) = ui.allocate_exact_size(vec2(TOUCH_TARGET, 34.0), Sense::click());
                let undo_resp = ui.interact(undo_rect, ui.scope_id().with("tablet_bl_undo"), if can_undo { Sense::click() } else { Sense::hover() });
                if can_undo && undo_resp.hovered() {
                    ui.painter().rect_filled(undo_rect, t.radius_md, t.hover);
                }
                crate::icons::paint_icon(ui.painter(), undo_rect.shrink(t.space_8), "undo", if can_undo { 1.0 } else { 0.3 });
                if undo_resp.backing_on_hover_text("Undo (↩) / Tap 2 Jari").clicked() && can_undo {
                    do_undo = true;
                }

                // Redo Button
                let (redo_rect, _) = ui.allocate_exact_size(vec2(TOUCH_TARGET, 34.0), Sense::click());
                let redo_resp = ui.interact(redo_rect, ui.scope_id().with("tablet_bl_redo"), if can_redo { Sense::click() } else { Sense::hover() });
                if can_redo && redo_resp.hovered() {
                    ui.painter().rect_filled(redo_rect, t.radius_md, t.hover);
                }
                crate::icons::paint_icon(ui.painter(), redo_rect.shrink(t.space_8), "redo", if can_redo { 1.0 } else { 0.3 });
                if redo_resp.backing_on_hover_text("Redo (↪) / Tap 3 Jari").clicked() && can_redo {
                    do_redo = true;
                }

                // Divider
                let (div_rect, _) = ui.allocate_exact_size(vec2(1.0, 20.0), Sense::hover());
                ui.painter().rect_filled(div_rect, 0.0, t.border);

                // Quick Radial Menu trigger (alternative for Pencil Pro squeeze / long-press)
                let (quick_rect, _) = ui.allocate_exact_size(vec2(34.0, 34.0), Sense::click());
                let quick_resp = ui.interact(quick_rect, ui.scope_id().with("tablet_bl_quick_menu"), Sense::click());
                if quick_resp.hovered() {
                    ui.painter().rect_filled(quick_rect, t.radius_md, t.hover);
                }
                ui.painter().text(
                    quick_rect.center(),
                    Align2::CENTER_CENTER,
                    "⚡",
                    egui::FontId::proportional(15.0),
                    Color32::from_rgb(0xfb, 0xbf, 0x24),
                );
                if quick_resp.backing_on_hover_text("Menu Cepat / Radial Marking Menu (Pengganti Pencil Squeeze)").clicked() {
                    do_quick_menu = true;
                }
            });
        });
    });

    if do_undo {
        let _ = app.run("edit.undo", json!({}));
    }
    if do_redo {
        let _ = app.run("edit.redo", json!({}));
    }
    if do_quick_menu {
        app.menu.radial = true;
        let menu_pos = pos2(screen.center().x, screen.center().y);
        crate::context_menu::open(app, menu_pos, crate::tool_modifiers::Held::default());
    }
}
