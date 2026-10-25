//! A touch-first chrome for the Android WebView build.
//!
//! The desktop workspace stays as it is: the canvas, dialogs, panels and commands are the same
//! upstream code. This module only replaces the small desktop rails around the canvas with a
//! phone-sized top bar, a bottom tool strip, and full-width panel/menu sheets. Every control here
//! is at least ~44 points (the usual Android touch target), and the desktop menu bar, toolbar,
//! status bar and dock are not drawn, so the picture gets the screen.
//!
//! The mode is enabled by the web shell (`?mobile`, which the Android activity appends to its
//! start URL). A desktop browser never sees it.

use std::sync::atomic::{AtomicBool, Ordering};

use egui::{Align2, Color32, FontId, Rect, RichText, Sense, Stroke, StrokeKind, Vec2, pos2, vec2};
use serde_json::json;

use crate::dock::Group;
use crate::state::Tool;
use crate::theme::{self, Tokens};
use crate::{PhotocraftApp, icons, menus, panels, tool_memory};

/// Set by the web shell before the app starts. A process-wide switch is enough: a page has one
/// PhotoCraft app, and the flag is never changed while it runs.
static ENABLED: AtomicBool = AtomicBool::new(false);

pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Open the touch-friendly New Document sheet. The start screen calls this instead of the
/// desktop's two-column New Document dialog, which is wider than a phone.
pub fn request_new_document(ctx: &egui::Context) {
    set_new(ctx, true);
    set_menu(ctx, false);
    set_tools(ctx, false);
    set_sheet(ctx, None);
}

const TOP_H: f32 = 54.0;
const BOTTOM_H: f32 = 70.0;
const TOUCH: f32 = 46.0;
const MENU_ID: &str = "photocraft-mobile-menu";
const TOOLS_ID: &str = "photocraft-mobile-tools";
const SHEET_ID: &str = "photocraft-mobile-sheet";
const NEW_ID: &str = "photocraft-mobile-new";

#[derive(Clone)]
struct NewDraft {
    name: String,
    width: String,
    height: String,
    background: String,
}

impl Default for NewDraft {
    fn default() -> Self {
        Self {
            name: "Untitled-1".into(),
            width: "1920".into(),
            height: "1080".into(),
            background: "white".into(),
        }
    }
}

/// The tools on the strip. More tools live in the tool sheet.
const PRIMARY_TOOLS: &[Tool] = &[
    Tool::Move,
    Tool::Brush,
    Tool::Eraser,
    Tool::Hand,
    Tool::Zoom,
    Tool::Type,
    Tool::Crop,
    Tool::RectMarquee,
    Tool::Lasso,
];

/// The panel groups in the order a phone needs them: Layers and Color first, the rest behind.
const SHEET_GROUPS: &[Group] = &[
    Group::Layers,
    Group::Color,
    Group::History,
    Group::Properties,
    Group::Brushes,
    Group::Navigator,
    Group::Character,
];

/// The tool sheet's groups. Every one of `Tool::ALL` appears exactly once.
const TOOL_GROUPS: &[(&str, &[Tool])] = &[
    (
        "Move & Select",
        &[
            Tool::Move,
            Tool::Artboard,
            Tool::RectMarquee,
            Tool::EllipseMarquee,
            Tool::SingleRowMarquee,
            Tool::SingleColumnMarquee,
            Tool::Lasso,
            Tool::PolygonLasso,
            Tool::MagneticLasso,
            Tool::MagicWand,
            Tool::QuickSelection,
            Tool::ObjectSelection,
            Tool::Crop,
            Tool::PerspectiveCrop,
            Tool::Slice,
            Tool::SliceSelect,
            Tool::Frame,
        ],
    ),
    (
        "Paint",
        &[
            Tool::Brush,
            Tool::Pencil,
            Tool::MixerBrush,
            Tool::ColorReplacement,
            Tool::Eraser,
            Tool::BackgroundEraser,
            Tool::MagicEraser,
            Tool::Gradient,
            Tool::PaintBucket,
        ],
    ),
    (
        "Retouch",
        &[
            Tool::Remove,
            Tool::SpotHealing,
            Tool::Healing,
            Tool::Patch,
            Tool::ContentAwareMove,
            Tool::RedEye,
            Tool::CloneStamp,
            Tool::PatternStamp,
            Tool::HistoryBrush,
            Tool::Blur,
            Tool::Sharpen,
            Tool::Smudge,
            Tool::Dodge,
            Tool::Burn,
            Tool::Sponge,
        ],
    ),
    (
        "Type & Vector",
        &[
            Tool::Pen,
            Tool::FreeformPen,
            Tool::CurvaturePen,
            Tool::AddAnchorPoint,
            Tool::DeleteAnchorPoint,
            Tool::ConvertPoint,
            Tool::PathSelection,
            Tool::DirectSelection,
            Tool::Type,
            Tool::VerticalType,
            Tool::HorizontalTypeMask,
            Tool::VerticalTypeMask,
            Tool::Rectangle,
            Tool::EllipseShape,
            Tool::Triangle,
            Tool::Polygon,
            Tool::Line,
            Tool::CustomShape,
        ],
    ),
    (
        "View & Measure",
        &[
            Tool::Hand,
            Tool::RotateView,
            Tool::Zoom,
            Tool::Eyedropper,
            Tool::ColorSampler,
            Tool::Ruler,
            Tool::Note,
            Tool::Count,
        ],
    ),
];

struct MenuSection {
    title: &'static str,
    items: &'static [(&'static str, &'static str)],
}

/// Curated commands, with the same ids the desktop menus use. Keeping the list short makes the
/// sheet useful on a phone; Edit › Search and the full desktop workspace remain available.
const MENU_SECTIONS: &[MenuSection] = &[
    MenuSection {
        title: "File",
        items: &[
            ("New…", "file.new"),
            ("Open…", "file.open"),
            ("Save", "file.save"),
            ("Save As…", "file.saveAs"),
            ("Export As…", "file.export.exportAs"),
            ("Quick Export as PNG", "file.export.quickExportAsPng"),
        ],
    },
    MenuSection {
        title: "Edit",
        items: &[
            ("Undo", "edit.undo"),
            ("Redo", "edit.redo"),
            ("Cut", "edit.cut"),
            ("Copy", "edit.copy"),
            ("Paste", "edit.paste"),
            ("Clear", "edit.clear"),
            ("Free Transform", "edit.freeTransform"),
        ],
    },
    MenuSection {
        title: "Select",
        items: &[
            ("All", "select.all"),
            ("Deselect", "select.deselect"),
            ("Inverse", "select.inverse"),
            ("Select and Mask…", "select.selectAndMask"),
        ],
    },
    MenuSection {
        title: "Image",
        items: &[
            ("Image Size…", "image.imageSize"),
            ("Canvas Size…", "image.canvasSize"),
            ("Crop", "image.crop"),
            ("Auto Tone", "image.autoTone"),
            ("Auto Contrast", "image.autoContrast"),
            ("Rotate 90° Clockwise", "image.imageRotation.90cw"),
            (
                "Flip Canvas Horizontal",
                "image.imageRotation.flipCanvasHorizontal",
            ),
        ],
    },
    MenuSection {
        title: "Layer",
        items: &[
            ("New Layer…", "layer.new.layer"),
            ("Duplicate Layer…", "layer.duplicate"),
            ("Delete Layer", "layer.delete"),
            ("Rasterize Layer", "layer.rasterize.layer"),
            ("Merge Layers", "layer.mergeLayers"),
            ("Flatten Image", "layer.flattenImage"),
        ],
    },
    MenuSection {
        title: "View",
        items: &[
            ("Fit on Screen", "view.fitOnScreen"),
            ("100%", "view.actualPixels"),
            ("Zoom In", "view.zoomIn"),
            ("Zoom Out", "view.zoomOut"),
            ("Rulers", "view.rulers"),
            ("Grid", "view.show.grid"),
            ("Guides", "view.show.guides"),
        ],
    },
];

pub fn chrome(app: &mut PhotocraftApp, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let t = Tokens::get(&ctx);

    egui::Panel::top("mobile_top")
        .exact_size(TOP_H)
        .frame(
            egui::Frame::NONE
                .fill(t.chrome)
                .inner_margin(egui::Margin::symmetric(8, 4)),
        )
        .show(ui, |ui| top_bar(app, ui, &t));

    // The options bar is still the upstream one: it scrolls sideways and keeps every tool's
    // settings, including brush size and opacity.
    panels::options_bar(app, ui);

    egui::Panel::bottom("mobile_tools")
        .exact_size(BOTTOM_H)
        .frame(
            egui::Frame::NONE
                .fill(t.chrome)
                .inner_margin(egui::Margin::symmetric(4, 4)),
        )
        .show(ui, |ui| bottom_tools(app, ui, &t));

    brush_quick(app, ui, &t);

    if sheet_group(&ctx).is_some() {
        panel_sheet(app, ui, &t);
    }

    if new_open(&ctx) {
        new_sheet(app, &ctx, &t);
    }
    if menu_open(&ctx) {
        menu_sheet(app, &ctx, &t);
    }
    if tools_open(&ctx) {
        tools_sheet(app, &ctx, &t);
    }

    // Escape closes our sheets before it reaches the editor's own handlers. The modal sheets also
    // report `should_close()` for a backdrop tap.
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        if menu_open(&ctx) {
            set_menu(&ctx, false);
        }
        if tools_open(&ctx) {
            set_tools(&ctx, false);
        }
        if sheet_group(&ctx).is_some() {
            set_sheet(&ctx, None);
        }
        if new_open(&ctx) {
            set_new(&ctx, false);
        }
    }
}

fn top_bar(app: &mut PhotocraftApp, ui: &mut egui::Ui, t: &Tokens) {
    ui.spacing_mut().item_spacing = vec2(4.0, 0.0);
    ui.horizontal_centered(|ui| {
        if hamburger(ui, TOUCH, t).clicked() {
            set_menu(ui.ctx(), true);
            set_tools(ui.ctx(), false);
            set_sheet(ui.ctx(), None);
        }

        // Undo and Redo are the two actions a touch editor needs constantly. Their states are the
        // same as the desktop menu's, so a disabled button is honest.
        let undo = menus::is_enabled(app, "edit.undo");
        let redo = menus::is_enabled(app, "edit.redo");
        let panels_open = sheet_group(ui.ctx()).is_some();
        let title_w = (ui.available_width() - 32.0 - TOUCH * 3.0 - 24.0).max(48.0);
        let title = document_title(app);
        let title_resp = ui.add_sized(
            [title_w, TOUCH],
            egui::Label::new(RichText::new(&title).size(14.5).color(t.text).strong()).truncate(),
        );
        title_resp.on_hover_text(title);
        if color_swatch(ui, app.session.tools.foreground, 32.0, t).clicked() {
            open_panel(app, ui.ctx(), Group::Color);
        }
        let undo_resp = top_icon(ui, "undo-2", TOUCH, undo, t, tl!("Undo"));
        if undo && undo_resp.clicked() {
            run(app, ui.ctx(), "edit.undo");
        }
        let redo_resp = top_icon(ui, "redo-2", TOUCH, redo, t, tl!("Redo"));
        if redo && redo_resp.clicked() {
            run(app, ui.ctx(), "edit.redo");
        }
        let panels_resp = top_icon(ui, "layers", TOUCH, true, t, tl!("Panels"));
        if panels_resp.clicked() {
            if panels_open {
                set_sheet(ui.ctx(), None);
            } else {
                set_sheet(ui.ctx(), Some(Group::Layers));
            }
            set_menu(ui.ctx(), false);
            set_tools(ui.ctx(), false);
        }
    });
}

/// Large brush-size steppers above the tool strip. The desktop options bar is still there for
/// every setting, but size is the one a finger changes constantly while painting.
fn brush_quick(app: &mut PhotocraftApp, ui: &mut egui::Ui, t: &Tokens) {
    if app.session.active().is_none() || !app.ui.tool.is_brushlike() {
        return;
    }
    let (mut smaller, mut larger) = (false, false);
    egui::Panel::bottom("mobile_brush_quick")
        .exact_size(50.0)
        .frame(
            egui::Frame::NONE
                .fill(t.chrome)
                .inner_margin(egui::Margin::symmetric(8, 4)),
        )
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.label(
                    RichText::new(tl!("Brush Size"))
                        .size(12.0)
                        .color(t.text_dim),
                );
                ui.add_space(4.0);
                if stepper(ui, "-", t).clicked() {
                    smaller = true;
                }
                let size = app.session.tools.brush.size;
                ui.add_sized(
                    [68.0, 40.0],
                    egui::Label::new(
                        RichText::new(format!("{size:.0} px"))
                            .size(15.0)
                            .color(t.text)
                            .strong(),
                    ),
                );
                if stepper(ui, "+", t).clicked() {
                    larger = true;
                }
            });
        });
    if smaller {
        let _ = app.run("tools.decreaseBrushSize", json!({}));
    }
    if larger {
        let _ = app.run("tools.increaseBrushSize", json!({}));
    }
}

fn stepper(ui: &mut egui::Ui, label: &str, t: &Tokens) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(44.0, 40.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, t.radius_sm, t.hover);
    }
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(21.0),
        t.text,
    );
    resp.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    resp
}

fn bottom_tools(app: &mut PhotocraftApp, ui: &mut egui::Ui, t: &Tokens) {
    egui::ScrollArea::horizontal()
        .id_salt("mobile-tool-strip")
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            ui.horizontal(|ui| {
                if grid_tile(ui, "grid-3x3", tl!("Tools"), false, t, tl!("All tools")).clicked() {
                    set_tools(ui.ctx(), true);
                    set_menu(ui.ctx(), false);
                    set_sheet(ui.ctx(), None);
                }
                for &tool in PRIMARY_TOOLS {
                    let selected = app.ui.tool == tool;
                    if tool_tile(ui, tool, selected, t).clicked() {
                        tool_memory::select(app, tool);
                        set_tools(ui.ctx(), false);
                    }
                }
            });
        });
}

fn tool_tile(ui: &mut egui::Ui, tool: Tool, selected: bool, t: &Tokens) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(TOUCH, 56.0), Sense::click());
    paint_tile(ui, rect, resp.hovered(), selected, t);
    icons::paint(
        ui,
        Rect::from_center_size(pos2(rect.center().x, rect.top() + 19.0), Vec2::splat(24.0)),
        icons::tool_icon(tool),
        23.0,
        tile_tint(selected, t),
    );
    let label_w = rect.width() - 4.0;
    let label = short_tool(tool);
    let galley = crate::tab_strip::elided(
        ui,
        label,
        FontId::proportional(9.5),
        if selected { t.accent_text } else { t.text_dim },
        label_w,
    );
    ui.painter().galley(
        pos2(
            rect.center().x - galley.size().x / 2.0,
            rect.bottom() - galley.size().y - 4.0,
        ),
        galley,
        t.text_dim,
    );
    resp.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), tl!(tool.label()))
    });
    resp.on_hover_text(tl!(tool.label()))
}

fn grid_tile(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    selected: bool,
    t: &Tokens,
    tip: &str,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(TOUCH, 56.0), Sense::click());
    paint_tile(ui, rect, resp.hovered(), selected, t);
    icons::paint(
        ui,
        Rect::from_center_size(pos2(rect.center().x, rect.top() + 19.0), Vec2::splat(24.0)),
        icon,
        23.0,
        tile_tint(selected, t),
    );
    let galley = crate::tab_strip::elided(
        ui,
        label,
        FontId::proportional(9.5),
        if selected { t.accent_text } else { t.text_dim },
        rect.width() - 4.0,
    );
    ui.painter().galley(
        pos2(
            rect.center().x - galley.size().x / 2.0,
            rect.bottom() - galley.size().y - 4.0,
        ),
        galley,
        t.text_dim,
    );
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), tip));
    resp.on_hover_text(tip)
}

fn paint_tile(ui: &egui::Ui, rect: Rect, hovered: bool, selected: bool, t: &Tokens) {
    if selected {
        ui.painter().rect_filled(rect, t.radius_sm, t.accent_soft);
        ui.painter().rect_stroke(
            rect,
            t.radius_sm,
            Stroke::new(1.0, t.accent_border),
            StrokeKind::Inside,
        );
    } else if hovered {
        ui.painter().rect_filled(rect, t.radius_sm, t.hover);
    }
}

fn tile_tint(selected: bool, t: &Tokens) -> Color32 {
    if selected { t.accent_text } else { t.icon }
}

fn top_icon(
    ui: &mut egui::Ui,
    name: &str,
    size: f32,
    enabled: bool,
    t: &Tokens,
    tip: &str,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    if resp.hovered() && enabled {
        ui.painter().rect_filled(rect, t.radius_sm, t.hover);
    }
    let tint = if enabled {
        if resp.hovered() { t.text } else { t.icon }
    } else {
        t.text_faint
    };
    icons::paint(ui, rect, name, (size * 0.52).round(), tint);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, tip));
    resp.on_hover_text(tip)
}

fn hamburger(ui: &mut egui::Ui, size: f32, t: &Tokens) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, t.radius_sm, t.hover);
    }
    let c = if resp.hovered() { t.text } else { t.icon };
    let (cx, cy) = (rect.center().x, rect.center().y);
    for dy in [-6.0, 0.0, 6.0] {
        ui.painter().line_segment(
            [pos2(cx - 9.0, cy + dy), pos2(cx + 9.0, cy + dy)],
            Stroke::new(2.0, c),
        );
    }
    resp.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), tl!("Menu"))
    });
    resp.on_hover_text(tl!("Menu"))
}

fn color_swatch(ui: &mut egui::Ui, color: [f32; 4], size: f32, t: &Tokens) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, t.radius_sm, t.hover);
    }
    let r = Rect::from_center_size(rect.center(), Vec2::splat(size - 10.0));
    ui.painter()
        .circle_filled(r.center(), r.width() / 2.0, c32(color));
    ui.painter()
        .circle_stroke(r.center(), r.width() / 2.0, Stroke::new(1.5, t.card_border));
    resp.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), tl!("Color"))
    });
    resp.on_hover_text(tl!("Color"))
}

fn c32(c: [f32; 4]) -> Color32 {
    let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    Color32::from_rgba_unmultiplied(q(c[0]), q(c[1]), q(c[2]), q(c[3]))
}

fn document_title(app: &PhotocraftApp) -> String {
    app.session
        .active()
        .map(|d| format!("{}{}", if d.is_dirty() { "*" } else { "" }, d.doc.name))
        .unwrap_or_else(|| "PhotoCraft".to_string())
}

fn panel_sheet(app: &mut PhotocraftApp, ui: &mut egui::Ui, t: &Tokens) {
    let ctx = ui.ctx().clone();
    let Some(group) = sheet_group(&ctx) else {
        return;
    };
    let pro = t.pro;
    let max_h = (ctx.content_rect().height() * 0.78).max(240.0);
    let default_h = (ctx.content_rect().height() * 0.48).clamp(260.0, max_h);
    let mut close = false;

    egui::Panel::bottom("mobile_panel_sheet")
        .resizable(true)
        .default_size(default_h)
        .size_range(200.0..=max_h)
        .frame(
            egui::Frame::NONE
                .fill(t.dock)
                .inner_margin(egui::Margin::symmetric(10, 6)),
        )
        .show(ui, |ui| {
            let (grab, _) = ui.allocate_exact_size(vec2(ui.available_width(), 5.0), Sense::hover());
            ui.painter().rect_filled(
                Rect::from_center_size(grab.center(), vec2(38.0, 3.0)),
                1.5,
                t.separator,
            );
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                let chip_w = 60.0;
                let avail = (ui.available_width() - TOUCH - 6.0).max(120.0);
                egui::ScrollArea::horizontal()
                    .id_salt("mobile-panel-groups")
                    .max_width(avail)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.x = 3.0;
                        ui.horizontal(|ui| {
                            for &g in SHEET_GROUPS {
                                if group_chip(ui, g, g == group, chip_w, t).clicked() {
                                    set_sheet(&ctx, Some(g));
                                    *g.shown_mut(&mut app.ui.panels) = true;
                                }
                            }
                        });
                    });
                if top_icon(ui, "x", TOUCH, true, t, tl!("Close panels")).clicked() {
                    close = true;
                }
            });
            ui.add_space(2.0);
            let tabs = group.tabs(pro);
            let sel = (*group.tab_mut(&mut app.ui.dock_tabs)).min(tabs.len().saturating_sub(1));
            if tabs.len() > 1 {
                egui::ScrollArea::horizontal()
                    .id_salt("mobile-panel-tabs")
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.x = 3.0;
                        ui.horizontal(|ui| {
                            for (i, name) in tabs.iter().enumerate() {
                                let selected = i == sel;
                                let (rect, resp) =
                                    ui.allocate_exact_size(vec2(84.0, 38.0), Sense::click());
                                if selected {
                                    ui.painter().rect_filled(rect, t.radius_sm, t.accent_soft);
                                    ui.painter().rect_stroke(
                                        rect,
                                        t.radius_sm,
                                        Stroke::new(1.0, t.accent_border),
                                        StrokeKind::Inside,
                                    );
                                } else if resp.hovered() {
                                    ui.painter().rect_filled(rect, t.radius_sm, t.hover);
                                }
                                ui.painter().text(
                                    rect.center(),
                                    Align2::CENTER_CENTER,
                                    tl!(*name),
                                    FontId::proportional(12.5),
                                    if selected { t.accent_text } else { t.text_dim },
                                );
                                resp.widget_info(|| {
                                    egui::WidgetInfo::labeled(
                                        egui::WidgetType::Button,
                                        ui.is_enabled(),
                                        tl!(*name),
                                    )
                                });
                                if resp.clicked() {
                                    *group.tab_mut(&mut app.ui.dock_tabs) = i;
                                }
                            }
                        });
                    });
            }
            ui.add_space(4.0);
            widgets_hairline(ui, t);
            ui.add_space(4.0);
            let tab = (*group.tab_mut(&mut app.ui.dock_tabs)).min(tabs.len().saturating_sub(1));
            if group.scrolls_itself(tab) {
                ui.set_min_height(ui.available_height());
                panels::dock_body(app, ui, group, tab);
            } else {
                egui::ScrollArea::vertical()
                    .id_salt(("mobile-panel-scroll", group.key()))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        panels::dock_body(app, ui, group, tab);
                    });
            }
        });

    if close || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        set_sheet(&ctx, None);
    }
}

fn group_chip(
    ui: &mut egui::Ui,
    group: Group,
    selected: bool,
    width: f32,
    t: &Tokens,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(width, 38.0), Sense::click());
    if selected {
        ui.painter().rect_filled(rect, t.radius_sm, t.accent_soft);
        ui.painter().rect_stroke(
            rect,
            t.radius_sm,
            Stroke::new(1.0, t.accent_border),
            StrokeKind::Inside,
        );
    } else if resp.hovered() {
        ui.painter().rect_filled(rect, t.radius_sm, t.hover);
    }
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        group_label(group),
        FontId::proportional(11.5),
        if selected { t.accent_text } else { t.text_dim },
    );
    resp.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            ui.is_enabled(),
            group_label(group),
        )
    });
    resp
}

fn group_label(group: Group) -> &'static str {
    match group {
        Group::Color => tl!("Color"),
        Group::Properties => tl!("Properties"),
        Group::Character => tl!("Character"),
        Group::Navigator => tl!("Navigator"),
        Group::History => tl!("History"),
        Group::Layers => tl!("Layers"),
        Group::Brushes => tl!("Brushes"),
    }
}

fn menu_sheet(app: &mut PhotocraftApp, ctx: &egui::Context, t: &Tokens) {
    let width = (ctx.content_rect().width() - 32.0).clamp(260.0, 480.0);
    let max_h = (ctx.content_rect().height() - 80.0).max(300.0);
    let mut close = false;
    let mut new_doc = false;
    let mut action: Option<String> = None;
    let mut open: Option<Group> = None;

    let just_opened = opened_this_frame(ctx, MENU_ID);
    let modal = egui::Modal::new(egui::Id::new(MENU_ID)).show(ctx, |ui| {
        ui.set_width(width);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(tl!("Menu"))
                    .font(theme::semibold(20.0))
                    .color(t.text),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if top_icon(ui, "x", 44.0, true, t, tl!("Close menu")).clicked() {
                    close = true;
                }
            });
        });
        ui.add_space(4.0);
        widgets_hairline(ui, t);
        ui.add_space(4.0);

        // The four actions a phone user reaches for first.
        let quick_w = ((width - 3.0 * 6.0) / 4.0).max(54.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            for (label, id) in [
                ("New", "file.new"),
                ("Open", "file.open"),
                ("Save", "file.save"),
                ("Export", "file.export.quickExportAsPng"),
            ] {
                let enabled = menus::is_enabled(app, id);
                let resp = ui.add_enabled(
                    enabled,
                    egui::Button::new(RichText::new(tl!(label)).size(12.5))
                        .min_size(vec2(quick_w, 46.0)),
                );
                if resp.clicked() {
                    if id == "file.new" {
                        new_doc = true;
                    } else {
                        action = Some(id.to_string());
                    }
                }
            }
        });
        ui.add_space(8.0);
        ui.label(
            RichText::new(tl!("Panels"))
                .font(theme::semibold(12.5))
                .color(t.text_faint),
        );
        ui.add_space(2.0);
        egui::Grid::new("mobile-menu-panels")
            .num_columns(3)
            .spacing([6.0, 6.0])
            .show(ui, |ui| {
                for (i, &g) in SHEET_GROUPS.iter().enumerate() {
                    let resp = ui.add(
                        egui::Button::new(RichText::new(group_label(g)).size(12.5))
                            .min_size(vec2((width - 12.0) / 3.0 - 4.0, 42.0)),
                    );
                    if resp.clicked() {
                        open = Some(g);
                    }
                    if i % 3 == 2 {
                        ui.end_row();
                    }
                }
            });

        egui::ScrollArea::vertical()
            .id_salt("mobile-menu-scroll")
            .max_height((max_h - 300.0).max(80.0))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if app.session.documents().len() > 1 {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(tl!("Documents"))
                            .font(theme::semibold(12.5))
                            .color(t.text_faint),
                    );
                    ui.add_space(2.0);
                    let active = app.session.active_index();
                    let docs: Vec<(usize, String)> = app
                        .session
                        .documents()
                        .iter()
                        .enumerate()
                        .map(|(i, d)| (i, d.doc.name.clone()))
                        .collect();
                    for (i, name) in docs {
                        let selected = active == Some(i);
                        let resp = ui.add_sized(
                            [ui.available_width(), 44.0],
                            egui::Button::new(RichText::new(name).size(14.0))
                                .selected(selected)
                                .truncate(),
                        );
                        if resp.clicked() {
                            app.session.set_active(i);
                            close = true;
                        }
                    }
                }
                for section in MENU_SECTIONS {
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(tl!(section.title))
                            .font(theme::semibold(12.5))
                            .color(t.text_faint),
                    );
                    ui.add_space(2.0);
                    for &(label, id) in section.items {
                        let enabled = menus::is_enabled(app, id);
                        let w = ui.available_width();
                        let resp = ui
                            .add_enabled_ui(enabled, |ui| {
                                ui.add_sized(
                                    [w, 48.0],
                                    egui::Button::new(RichText::new(tl!(label)).size(15.0))
                                        .truncate(),
                                )
                            })
                            .inner;
                        if enabled && resp.clicked() {
                            if id == "file.new" {
                                new_doc = true;
                            } else {
                                action = Some(id.to_string());
                            }
                        }
                    }
                }
                ui.add_space(12.0);
                ui.label(
                    RichText::new(tl!("PhotoCraft for Android"))
                        .size(11.5)
                        .color(t.text_faint),
                );
                ui.add_space(6.0);
            });
    });

    if modal.should_close() && !just_opened {
        close = true;
    }
    if close {
        set_menu(ctx, false);
    }
    if new_doc {
        set_new(ctx, true);
        set_menu(ctx, false);
    }
    if let Some(g) = open {
        open_panel(app, ctx, g);
    }
    if let Some(id) = action {
        set_menu(ctx, false);
        run(app, ctx, &id);
    }
}

fn new_sheet(app: &mut PhotocraftApp, ctx: &egui::Context, t: &Tokens) {
    let width = (ctx.content_rect().width() - 32.0).clamp(260.0, 460.0);
    let max_h = (ctx.content_rect().height() - 80.0).max(300.0);
    let draft_id = egui::Id::new(NEW_ID).with("draft");
    let mut draft = ctx
        .data(|d| d.get_temp::<NewDraft>(draft_id))
        .unwrap_or_default();
    let (mut close, mut create) = (false, false);
    let valid = draft
        .width
        .trim()
        .parse::<u32>()
        .is_ok_and(|v| (1..=300_000).contains(&v))
        && draft
            .height
            .trim()
            .parse::<u32>()
            .is_ok_and(|v| (1..=300_000).contains(&v));

    let modal = egui::Modal::new(egui::Id::new(NEW_ID)).show(ctx, |ui| {
        ui.set_width(width);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(tl!("New document"))
                    .font(theme::semibold(20.0))
                    .color(t.text),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if top_icon(ui, "x", 44.0, true, t, tl!("Close")).clicked() {
                    close = true;
                }
            });
        });
        ui.add_space(4.0);
        widgets_hairline(ui, t);
        ui.add_space(4.0);
        egui::ScrollArea::vertical()
            .id_salt("mobile-new-scroll")
            .max_height((max_h - 150.0).max(200.0))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.label(
                    RichText::new(tl!("Presets"))
                        .font(theme::semibold(12.5))
                        .color(t.text_faint),
                );
                ui.add_space(3.0);
                for row in [
                    [("Square", 1080u32, 1080u32), ("Landscape", 1920, 1080)],
                    [("Portrait", 1080, 1920), ("Large square", 2048, 2048)],
                ] {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        for (label, w, h) in row {
                            let selected =
                                draft.width == w.to_string() && draft.height == h.to_string();
                            let resp = ui.add_sized(
                                [(width - 18.0) / 2.0, 42.0],
                                egui::Button::new(
                                    RichText::new(format!("{label}\n{w} × {h}")).size(11.5),
                                )
                                .selected(selected),
                            );
                            if resp.clicked() {
                                draft.width = w.to_string();
                                draft.height = h.to_string();
                            }
                        }
                    });
                    ui.add_space(4.0);
                }
                ui.add_space(6.0);
                ui.label(RichText::new(tl!("Name")).size(12.0).color(t.text_dim));
                ui.add_sized(
                    [width, 44.0],
                    egui::TextEdit::singleline(&mut draft.name).font(FontId::proportional(16.0)),
                );
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    for (label, value) in [
                        (tl!("Width"), &mut draft.width),
                        (tl!("Height"), &mut draft.height),
                    ] {
                        ui.vertical(|ui| {
                            ui.set_width((width - 8.0) / 2.0);
                            ui.label(RichText::new(label).size(12.0).color(t.text_dim));
                            ui.add_sized(
                                [ui.available_width(), 44.0],
                                egui::TextEdit::singleline(value).font(FontId::proportional(16.0)),
                            );
                        });
                    }
                });
                ui.label(RichText::new(tl!("Pixels")).size(11.0).color(t.text_faint));
                ui.add_space(8.0);
                ui.label(
                    RichText::new(tl!("Background"))
                        .size(12.0)
                        .color(t.text_dim),
                );
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    for (label, value) in [
                        ("White", "white"),
                        ("Transparent", "transparent"),
                        ("Black", "black"),
                    ] {
                        let selected = draft.background == value;
                        let resp = ui.add_sized(
                            [(width - 12.0) / 3.0, 42.0],
                            egui::Button::new(RichText::new(tl!(label)).size(12.0))
                                .selected(selected),
                        );
                        if resp.clicked() {
                            draft.background = value.to_string();
                        }
                    }
                });
                ui.add_space(10.0);
            });
        ui.add_space(4.0);
        let create_button = ui.add_enabled(
            valid,
            egui::Button::new(
                RichText::new(tl!("Create"))
                    .size(16.0)
                    .color(t.primary_text),
            )
            .fill(t.accent)
            .min_size(vec2(width, 52.0)),
        );
        if create_button.clicked() {
            create = true;
        }
    });
    if modal.should_close() {
        close = true;
    }
    ctx.data_mut(|d| d.insert_temp(draft_id, draft.clone()));
    if create {
        let (Ok(w), Ok(h)) = (
            draft.width.trim().parse::<u32>(),
            draft.height.trim().parse::<u32>(),
        ) else {
            return;
        };
        let params = json!({
            "width": w,
            "height": h,
            "name": if draft.name.trim().is_empty() { "Untitled" } else { draft.name.trim() },
            "background": draft.background,
            "resolution": 72,
        });
        match app.run("file.new", params) {
            Ok(_) => set_new(ctx, false),
            Err(e) => {
                app.ui.status = e.clone();
                app.ui.status_error = true;
                crate::notices::post(
                    app,
                    tl!("Could not create the document"),
                    vec![e],
                    true,
                    None,
                );
            }
        }
    } else if close {
        set_new(ctx, false);
    }
}

fn tools_sheet(app: &mut PhotocraftApp, ctx: &egui::Context, t: &Tokens) {
    let width = (ctx.content_rect().width() - 32.0).clamp(260.0, 520.0);
    let max_h = (ctx.content_rect().height() - 80.0).max(300.0);
    let mut close = false;
    let mut chosen: Option<Tool> = None;
    let cell_w = ((width - 2.0 * 6.0) / 3.0).floor();

    let just_opened = opened_this_frame(ctx, TOOLS_ID);
    let modal = egui::Modal::new(egui::Id::new(TOOLS_ID)).show(ctx, |ui| {
        ui.set_width(width);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(tl!("Tools"))
                    .font(theme::semibold(20.0))
                    .color(t.text),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if top_icon(ui, "x", 44.0, true, t, tl!("Close tools")).clicked() {
                    close = true;
                }
            });
        });
        ui.add_space(4.0);
        widgets_hairline(ui, t);
        ui.add_space(4.0);
        egui::ScrollArea::vertical()
            .id_salt("mobile-tools-scroll")
            .max_height((max_h - 80.0).max(200.0))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (title, tools) in TOOL_GROUPS {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(tl!(*title))
                            .font(theme::semibold(12.5))
                            .color(t.text_faint),
                    );
                    ui.add_space(2.0);
                    egui::Grid::new(("mobile-tool-grid", title))
                        .num_columns(3)
                        .spacing([6.0, 6.0])
                        .show(ui, |ui| {
                            for (i, &tool) in tools.iter().enumerate() {
                                if tool_cell(ui, tool, app.ui.tool == tool, cell_w, t).clicked() {
                                    chosen = Some(tool);
                                }
                                if i % 3 == 2 {
                                    ui.end_row();
                                }
                            }
                        });
                }
                ui.add_space(12.0);
            });
    });

    if modal.should_close() && !just_opened {
        close = true;
    }
    if close {
        set_tools(ctx, false);
    }
    if let Some(tool) = chosen {
        tool_memory::select(app, tool);
        set_tools(ctx, false);
    }
}

fn tool_cell(
    ui: &mut egui::Ui,
    tool: Tool,
    selected: bool,
    width: f32,
    t: &Tokens,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(width, 66.0), Sense::click());
    if selected {
        ui.painter().rect_filled(rect, t.radius_sm, t.accent_soft);
        ui.painter().rect_stroke(
            rect,
            t.radius_sm,
            Stroke::new(1.0, t.accent_border),
            StrokeKind::Inside,
        );
    } else if resp.hovered() {
        ui.painter().rect_filled(rect, t.radius_sm, t.hover);
    }
    icons::paint(
        ui,
        Rect::from_center_size(pos2(rect.center().x, rect.top() + 22.0), Vec2::splat(28.0)),
        icons::tool_icon(tool),
        27.0,
        tile_tint(selected, t),
    );
    let label = short_tool(tool);
    let galley = crate::tab_strip::elided(
        ui,
        label,
        FontId::proportional(10.5),
        if selected { t.accent_text } else { t.text_dim },
        width - 8.0,
    );
    ui.painter().galley(
        pos2(
            rect.center().x - galley.size().x / 2.0,
            rect.bottom() - galley.size().y - 6.0,
        ),
        galley,
        t.text_dim,
    );
    resp.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), tl!(tool.label()))
    });
    resp.on_hover_text(tl!(tool.label()))
}

fn short_tool(tool: Tool) -> &'static str {
    match tool {
        Tool::RectMarquee => "Marquee",
        Tool::EllipseMarquee => "Ellipse",
        Tool::SingleRowMarquee => "Row",
        Tool::SingleColumnMarquee => "Column",
        Tool::PolygonLasso => "Polygon",
        Tool::MagneticLasso => "Magnetic",
        Tool::MagicWand => "Wand",
        Tool::QuickSelection => "Quick",
        Tool::ObjectSelection => "Object",
        Tool::PerspectiveCrop => "Perspective",
        Tool::BackgroundEraser => "BG Eraser",
        Tool::MagicEraser => "Magic Eraser",
        Tool::ColorReplacement => "Replace",
        Tool::PaintBucket => "Bucket",
        Tool::Type => "Type",
        Tool::VerticalType => "Vertical Type",
        Tool::HorizontalTypeMask => "Type Mask",
        Tool::VerticalTypeMask => "V Type Mask",
        Tool::RotateView => "Rotate",
        Tool::SpotHealing => "Spot Heal",
        Tool::Healing => "Heal",
        Tool::ContentAwareMove => "Content Move",
        Tool::RedEye => "Red Eye",
        Tool::CloneStamp => "Clone",
        Tool::PatternStamp => "Pattern",
        Tool::HistoryBrush => "History",
        Tool::FreeformPen => "Freeform",
        Tool::CurvaturePen => "Curvature",
        Tool::AddAnchorPoint => "Add Anchor",
        Tool::DeleteAnchorPoint => "Del Anchor",
        Tool::ConvertPoint => "Convert",
        Tool::PathSelection => "Path",
        Tool::DirectSelection => "Direct",
        Tool::EllipseShape => "Ellipse Shape",
        Tool::CustomShape => "Custom Shape",
        other => other.label().strip_suffix(" Tool").unwrap_or(other.label()),
    }
}

fn open_panel(app: &mut PhotocraftApp, ctx: &egui::Context, group: Group) {
    set_menu(ctx, false);
    set_tools(ctx, false);
    set_sheet(ctx, Some(group));
    *group.shown_mut(&mut app.ui.panels) = true;
}

fn run(app: &mut PhotocraftApp, ctx: &egui::Context, id: &str) {
    if let Err(e) = menus::invoke(app, ctx, id, json!({})) {
        app.ui.status = e.clone();
        app.ui.status_error = true;
        crate::notices::post(app, tl!("Command failed"), vec![e], true, None);
    }
}

fn widgets_hairline(ui: &mut egui::Ui, t: &Tokens) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().line_segment(
        [r.left_center(), r.right_center()],
        Stroke::new(1.0, t.separator),
    );
}

fn menu_open(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(egui::Id::new(MENU_ID)))
        .unwrap_or(false)
}

fn set_menu(ctx: &egui::Context, open: bool) {
    if open {
        remember_opened(ctx, MENU_ID);
    }
    ctx.data_mut(|d| d.insert_temp(egui::Id::new(MENU_ID), open));
    ctx.request_repaint();
}

fn tools_open(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(egui::Id::new(TOOLS_ID)))
        .unwrap_or(false)
}

fn set_tools(ctx: &egui::Context, open: bool) {
    if open {
        remember_opened(ctx, TOOLS_ID);
    }
    ctx.data_mut(|d| d.insert_temp(egui::Id::new(TOOLS_ID), open));
    ctx.request_repaint();
}

fn remember_opened(ctx: &egui::Context, id: &str) {
    let frame = ctx.cumulative_frame_nr();
    ctx.data_mut(|d| d.insert_temp(egui::Id::new(id).with("opened"), frame));
}

fn opened_this_frame(ctx: &egui::Context, id: &str) -> bool {
    let frame = ctx.cumulative_frame_nr();
    ctx.data(|d| d.get_temp::<u64>(egui::Id::new(id).with("opened")))
        .is_some_and(|f| f == frame)
}

fn sheet_group(ctx: &egui::Context) -> Option<Group> {
    ctx.data(|d| d.get_temp::<Option<Group>>(egui::Id::new(SHEET_ID)))
        .flatten()
}

fn set_sheet(ctx: &egui::Context, group: Option<Group>) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new(SHEET_ID), group));
    ctx.request_repaint();
}

fn new_open(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(egui::Id::new(NEW_ID)))
        .unwrap_or(false)
}

fn set_new(ctx: &egui::Context, open: bool) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new(NEW_ID), open));
    ctx.request_repaint();
}
