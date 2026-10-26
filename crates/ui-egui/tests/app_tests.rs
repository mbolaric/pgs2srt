use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use eframe::App as _;
use egui::{pos2, vec2, Frame, Rect};
use pgs_core::{I18nService, UiLanguage};
use pgs_ocr::ModelInfo;
use ui_egui::app::*;
use ui_egui::i18n::{format_language_label, tr};
use ui_egui::models::*;
use ui_egui::theme::{self, ThemeKind, Tokens};
use ui_egui::PgsApp;

/// The UI language is process-wide: tests that set it, or lay out translated text, take turns.
fn lock_ui_language() -> MutexGuard<'static, ()> {
    static UI_LANGUAGE: Mutex<()> = Mutex::new(());
    UI_LANGUAGE.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Since egui 0.37 a panel's id is its salt combined with the parent `Ui`, here the root `Ui`.
fn inspector_panel_id() -> egui::Id {
    egui::ViewportId::ROOT.root_ui_id().with("inspector_panel")
}

fn render_headless_frame(app: &mut PgsApp, ctx: &egui::Context, width: f32, height: f32) -> Vec<egui::epaint::ClippedShape> {
    let raw =
        egui::RawInput { screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(width, height))), ..Default::default() };
    let mut output = ctx.run_ui(raw, |ui| {
        // Exercise full UI layout directly without requiring a native window handle
        app.sync_system_theme(ui.ctx());
        app.ui_tab_strip(ui);
        app.ui_command_toolbar(ui);
        app.ui_status_bar(ui);
        app.ui_inspector_panel(ui);
        let t = Tokens::get(ui.ctx());
        egui::CentralPanel::default().frame(Frame::NONE.fill(t.pasteboard)).show(ui, |ui| match app.tab {
            WorkspaceTab::Queue => {
                if app.items.is_empty() {
                    app.ui_empty_queue(ui);
                } else {
                    app.ui_populated_queue(ui);
                }
            }
            WorkspaceTab::Languages => {
                app.ui_models_workspace(ui);
            }
        });
    });
    output.textures_delta.clear();
    output.shapes
}

#[test]
fn renders_cleanly_in_light_and_dark_themes_and_both_languages_at_multiple_sizes() {
    let _language = lock_ui_language();
    let ctx = egui::Context::default();
    theme::install_fonts(&ctx);
    let mut app = PgsApp::default();

    for kind in [ThemeKind::Light, ThemeKind::Dark] {
        theme::apply(&ctx, kind);
        app.theme_kind = kind;

        for &ui_lang in I18nService::supported() {
            app.set_ui_language(ui_lang);

            // 1. Empty Queue Home View (also assert card description never overlaps action link)
            app.tab = WorkspaceTab::Queue;
            app.items.clear();
            for (w, h) in [(1920.0, 1080.0), (1280.0, 800.0), (920.0, 580.0)] {
                let shapes = render_headless_frame(&mut app, &ctx, w, h);
                let texts: Vec<_> = shapes
                    .iter()
                    .filter_map(|s| match &s.shape {
                        egui::Shape::Text(t) => Some(t),
                        _ => None,
                    })
                    .collect();
                let action_lbl = tr("action.open_models");
                let body_lbl = tr("empty.card3_body");
                if let (Some(body_shape), Some(act_shape)) =
                    (texts.iter().find(|t| t.galley.job.text == body_lbl), texts.iter().find(|t| t.galley.job.text == action_lbl))
                {
                    assert!(
                        body_shape.pos.y + body_shape.galley.size().y <= act_shape.pos.y,
                        "Feature card body overlapped action link in {ui_lang:?} at {w}x{h}"
                    );
                }
            }

            // 2. Populated Queue with Pending, Converting, Completed, Failed items
            app.items = vec![
                SubtitleItem {
                    id: 1,
                    input: PathBuf::from("/movies/Inception.2010.eng.sup").into(),
                    file_size: 28_500_000,
                    output_path: PathBuf::from("/movies/Inception.2010.eng.eng.srt"),
                    language: "eng".into(),
                    detected_language: Some("eng".into()),
                    custom_output_override: false,
                    probing_language: false,
                    total_cues_hint: Some(1240),
                    status: ItemStatus::Pending,
                    srt: None,
                    downloaded: false,
                },
                SubtitleItem {
                    id: 2,
                    input: PathBuf::from("/movies/Dark.S01E01.deu.sup").into(),
                    file_size: 16_200_000,
                    output_path: PathBuf::from("/movies/Dark.S01E01.deu.deu.srt"),
                    language: "deu".into(),
                    detected_language: Some("deu".into()),
                    custom_output_override: false,
                    probing_language: false,
                    total_cues_hint: Some(420),
                    status: ItemStatus::Converting {
                        cue: 185,
                        total: 420,
                        start_ts: 90_000 * 65,
                        stop_ts: 90_000 * 68,
                        last_text: "Wann ist Mikkel?".into(),
                    },
                    srt: None,
                    downloaded: false,
                },
                SubtitleItem {
                    id: 3,
                    input: PathBuf::from("/movies/Corrupt_Stream.sup").into(),
                    file_size: 1024,
                    output_path: PathBuf::from("/movies/Corrupt_Stream.eng.srt"),
                    language: "eng".into(),
                    detected_language: None,
                    custom_output_override: true,
                    probing_language: false,
                    total_cues_hint: None,
                    status: ItemStatus::Failed("Invalid PGS segment header".into()),
                    srt: None,
                    downloaded: false,
                },
            ];
            app.selected_id = Some(2);
            for (w, h) in [(1920.0, 1080.0), (1280.0, 800.0), (920.0, 580.0)] {
                render_headless_frame(&mut app, &ctx, w, h);
            }

            // 3. OCR Language Models View
            app.tab = WorkspaceTab::Languages;
            for (w, h) in [(1920.0, 1080.0), (1280.0, 800.0), (920.0, 580.0)] {
                render_headless_frame(&mut app, &ctx, w, h);
            }
        }
    }

    // Verify eframe::App trait implementation compiles and is callable via trait bound
    fn _assert_eframe_app<T: eframe::App>() {}
    _assert_eframe_app::<PgsApp>();
    let _ = PgsApp::ui;
}

#[test]
fn batch_duplicate_output_paths_are_rejected_cleanly() {
    let _language = lock_ui_language();
    let mut app = PgsApp::default();
    app.preferences.use_custom_output_dir = true;
    app.preferences.custom_output_dir = Some(PathBuf::from("/tmp/shared_out"));

    app.items = vec![
        SubtitleItem::new(1, PathBuf::from("/disc1/track1.sup"), "eng", Some(Path::new("/tmp/shared_out"))),
        SubtitleItem::new(2, PathBuf::from("/disc2/track1.sup"), "eng", Some(Path::new("/tmp/shared_out"))),
    ];

    app.start_batch();
    assert!(!app.is_batch_running);
    assert!(matches!(app.items[0].status, ItemStatus::Failed(_)));
    assert!(matches!(app.items[1].status, ItemStatus::Failed(_)));
}

#[test]
fn inspector_panel_width_remains_rock_steady_across_frames() {
    let _language = lock_ui_language();
    let ctx = egui::Context::default();
    theme::install_fonts(&ctx);
    theme::apply(&ctx, ThemeKind::Light);
    let mut app = PgsApp::default();

    // Check 35 consecutive startup frames (empty queue) — must never creep wider.
    let mut widths = Vec::new();
    for _ in 0..35 {
        render_headless_frame(&mut app, &ctx, 1180.0, 740.0);
        let state =
            egui::containers::panel::PanelState::load(&ctx, inspector_panel_id()).expect("inspector_panel state must exist");
        widths.push(state.outer_rect.width());
    }
    let first_settled = widths[1];
    for (frame_idx, &w) in widths.iter().enumerate().skip(1) {
        assert!((w - first_settled).abs() < 0.01, "Inspector panel drifted at startup frame {frame_idx}: {first_settled} -> {w}");
    }

    // Also check 35 consecutive frames with a selected item in the queue.
    app.items.push(SubtitleItem::new(1, PathBuf::from("/movies/Film.hrv.sup"), "hrv", None));
    app.selected_id = Some(1);
    widths.clear();
    for _ in 0..35 {
        render_headless_frame(&mut app, &ctx, 1180.0, 740.0);
        let state =
            egui::containers::panel::PanelState::load(&ctx, inspector_panel_id()).expect("inspector_panel state must exist");
        widths.push(state.outer_rect.width());
    }
    let first_pop = widths[1];
    for (frame_idx, &w) in widths.iter().enumerate().skip(1) {
        assert!(
            (w - first_pop).abs() < 0.01,
            "Inspector panel drifted with selected item at frame {frame_idx}: {first_pop} -> {w}"
        );
    }
}

#[test]
fn language_popup_stays_open_when_clicking_search_input() {
    let _language = lock_ui_language();
    let ctx = egui::Context::default();
    theme::install_fonts(&ctx);
    theme::apply(&ctx, ThemeKind::Light);
    let mut app = PgsApp::default();
    app.preferences.splitter_position = 340.0;
    app.preferences.show_inspector = true;
    app.show_inspector = true;
    app.set_ui_language(UiLanguage::English);
    app.items.push(SubtitleItem::new(1, PathBuf::from("/movies/Test.hrv.sup"), "hrv", None));
    app.selected_id = Some(1);

    let run_events = |app: &mut PgsApp, events: Vec<egui::Event>, time: f64| {
        let raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1180.0, 740.0))),
            time: Some(time),
            events,
            ..Default::default()
        };
        let mut out = ctx.run_ui(raw, |ui| {
            app.ui_tab_strip(ui);
            app.ui_command_toolbar(ui);
            app.ui_status_bar(ui);
            app.ui_inspector_panel(ui);
            let t = Tokens::get(ui.ctx());
            egui::CentralPanel::default().frame(Frame::NONE.fill(t.pasteboard)).show(ui, |ui| app.ui_populated_queue(ui));
        });
        out.textures_delta.clear();
        out.shapes
    };

    // Frame 0: initial layout
    let shapes = run_events(&mut app, vec![], 0.0);

    // Locate the Inspector language dropdown button via its Croatian label text
    let target_label = format_language_label("hrv", UiLanguage::English);
    let insp_min_x = 1180.0 - app.preferences.splitter_position;
    let insp_drop_pos = shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.job.text.starts_with("hrv") || t.galley.job.text == target_label => {
                // Pick the one inside the right inspector (x >= insp_min_x)
                (t.pos.x >= insp_min_x).then_some(pos2(t.pos.x + 20.0, t.pos.y + 4.0))
            }
            _ => None,
        })
        .next()
        .expect("Expected Inspector language dropdown button");

    // Normal click (press + release within 50ms) to open the Inspector language popup
    run_events(
        &mut app,
        vec![
            egui::Event::PointerMoved(insp_drop_pos),
            egui::Event::PointerButton {
                pos: insp_drop_pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
        0.05,
    );
    run_events(
        &mut app,
        vec![egui::Event::PointerButton {
            pos: insp_drop_pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
        0.08,
    );
    // Settle 1 frame after popup sizing pass so popup widgets are painted at final positions.
    let popup_shapes = run_events(&mut app, vec![], 0.12);

    assert!(egui::Popup::is_any_open(&ctx), "Language popup should be open after clicking dropdown button");

    // Find the search input placeholder inside the open popup
    let hint_text = tr("models.search_hint");
    let search_pos = popup_shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.job.text == hint_text => Some(pos2(t.pos.x + 30.0, t.pos.y + 4.0)),
            _ => None,
        })
        .next()
        .expect("Expected search input inside open language popup");

    // Perform a NORMAL short click (press at 0.20s, release at 0.24s) inside the search input!
    run_events(
        &mut app,
        vec![
            egui::Event::PointerMoved(search_pos),
            egui::Event::PointerButton {
                pos: search_pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
        0.20,
    );
    run_events(
        &mut app,
        vec![egui::Event::PointerButton {
            pos: search_pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
        0.24,
    );
    // Settle one more frame and type "hrv"
    run_events(&mut app, vec![egui::Event::Text("hrv".into())], 0.30);

    assert!(egui::Popup::is_any_open(&ctx), "Language popup MUST stay open after a normal click inside search input");
    assert_eq!(app.popup_lang_search, "hrv");
}

#[test]
fn popup_arrow_keys_navigate_filtered_list_and_enter_selects() {
    let _language = lock_ui_language();
    let ctx = egui::Context::default();
    theme::install_fonts(&ctx);
    theme::apply(&ctx, ThemeKind::Light);
    let mut app = PgsApp::default();
    app.preferences.splitter_position = 340.0;
    app.preferences.show_inspector = true;
    app.show_inspector = true;
    app.installed_langs = vec!["deu".into(), "eng".into(), "fra".into(), "hrv".into(), "spa".into()];
    app.selected_id = Some(1);
    app.set_ui_language(UiLanguage::English);
    app.items.push(SubtitleItem::new(1, PathBuf::from("/movies/Sample.eng.sup"), "eng", None));

    let run_events = |app: &mut PgsApp, events: Vec<egui::Event>, time: f64| {
        let raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1180.0, 740.0))),
            time: Some(time),
            events,
            ..Default::default()
        };
        let mut out = ctx.run_ui(raw, |ui| {
            app.handle_queue_keyboard(ui.ctx());
            app.ui_tab_strip(ui);
            app.ui_command_toolbar(ui);
            app.ui_status_bar(ui);
            app.ui_inspector_panel(ui);
            let t = Tokens::get(ui.ctx());
            egui::CentralPanel::default().frame(Frame::NONE.fill(t.pasteboard)).show(ui, |ui| app.ui_populated_queue(ui));
        });
        out.textures_delta.clear();
        out.shapes
    };

    let shapes = run_events(&mut app, vec![], 0.0);
    let eng_label = format_language_label("eng", UiLanguage::English);
    let insp_min_x = 1180.0 - app.preferences.splitter_position;
    let insp_drop_pos = shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.job.text == eng_label && t.pos.x >= insp_min_x => {
                Some(pos2(t.pos.x + 20.0, t.pos.y + 4.0))
            }
            _ => None,
        })
        .next()
        .expect("Expected Inspector language dropdown");

    // Open popup
    run_events(
        &mut app,
        vec![
            egui::Event::PointerMoved(insp_drop_pos),
            egui::Event::PointerButton {
                pos: insp_drop_pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
        0.05,
    );
    run_events(
        &mut app,
        vec![egui::Event::PointerButton {
            pos: insp_drop_pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
        0.08,
    );
    run_events(&mut app, vec![], 0.12);
    // "eng" is index 1 in ["deu", "eng", "fra", "hrv", "spa"]
    assert_eq!(app.popup_highlight_idx, 1);

    // Press ArrowDown twice -> index 3 ("hrv") while search input has focus!
    let key_press = |key: egui::Key| egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    run_events(&mut app, vec![key_press(egui::Key::ArrowDown)], 0.16);
    assert_eq!(app.popup_highlight_idx, 2);
    run_events(&mut app, vec![key_press(egui::Key::ArrowDown)], 0.20);
    assert_eq!(app.popup_highlight_idx, 3);

    // Press Enter -> selects "hrv" and closes popup!
    run_events(&mut app, vec![key_press(egui::Key::Enter)], 0.24);
    run_events(&mut app, vec![], 0.28);
    assert_eq!(app.items[0].language, "hrv");
    assert!(!egui::Popup::is_any_open(&ctx));
}

#[test]
fn cinema_preview_box_never_jumps_in_height_during_conversion() {
    let _language = lock_ui_language();
    let ctx = egui::Context::default();
    theme::install_fonts(&ctx);
    theme::apply(&ctx, ThemeKind::Light);
    let mut app = PgsApp::default();
    app.items.push(SubtitleItem::new(1, PathBuf::from("/movies/Film.hrv.sup"), "hrv", None));
    app.selected_id = Some(1);

    let cinema_bg = Tokens::for_kind(ThemeKind::Light).cinema_bg;
    let find_cinema_rect = |shapes: &[egui::epaint::ClippedShape]| -> Rect {
        shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Rect(r) if r.fill == cinema_bg => Some(r.rect),
                _ => None,
            })
            .expect("Expected cinema preview rect")
    };

    // 1. Empty preview state (before any cue arrives)
    render_headless_frame(&mut app, &ctx, 1180.0, 740.0);
    let base_rect = find_cinema_rect(&render_headless_frame(&mut app, &ctx, 1180.0, 740.0));

    // 2. Converting cues with wildly varying bitmap sizes and 0..4 lines of text
    let frames = [
        (120usize, 24usize, ""),
        (480, 42, "Dobar dan!"),
        (820, 96, "Što radiš ovdje?\nNe znam što se dogodilo."),
        (640, 150, "Prvi red\nDrugi red\nTreći red\nČetvrti red"),
    ];

    for (seq, &(w, h, txt)) in frames.iter().enumerate() {
        app.items[0].status = ItemStatus::Converting {
            cue: seq + 1,
            total: 200,
            start_ts: 90_000 * 10,
            stop_ts: 90_000 * 13,
            last_text: txt.to_string(),
        };
        app.live_preview = Some(PreviewData {
            item_id: 1,
            cue: seq + 1,
            total: 200,
            start_ts: 90_000 * 10,
            stop_ts: 90_000 * 13,
            text: txt.to_string(),
            rgba: Some((w, h, Arc::<[u8]>::from(vec![220u8; w * h * 4]))),
            seq: (seq + 10) as u64,
        });

        let rect = find_cinema_rect(&render_headless_frame(&mut app, &ctx, 1180.0, 740.0));
        assert!(
            (rect.top() - base_rect.top()).abs() < 0.01 && (rect.height() - base_rect.height()).abs() < 0.01,
            "Cinema preview box jumped at cue {seq}: base={base_rect:?}, got={rect:?}"
        );
    }
}

#[test]
fn queue_list_supports_up_down_home_end_delete_keyboard_navigation() {
    let _language = lock_ui_language();
    let ctx = egui::Context::default();
    theme::install_fonts(&ctx);
    theme::apply(&ctx, ThemeKind::Light);
    let mut app = PgsApp::default();
    app.items = vec![
        SubtitleItem::new(10, PathBuf::from("/movies/A.eng.sup"), "eng", None),
        SubtitleItem::new(20, PathBuf::from("/movies/B.hrv.sup"), "hrv", None),
        SubtitleItem::new(30, PathBuf::from("/movies/C.deu.sup"), "deu", None),
    ];
    app.selected_id = Some(10);

    let send_key = |app: &mut PgsApp, key: egui::Key, time: f64| {
        let raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1180.0, 740.0))),
            time: Some(time),
            events: vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };
        let mut out = ctx.run_ui(raw, |ui| {
            app.handle_queue_keyboard(ui.ctx());
            app.ui_populated_queue(ui);
        });
        out.textures_delta.clear();
    };

    send_key(&mut app, egui::Key::ArrowDown, 0.1);
    assert_eq!(app.selected_id, Some(20));

    send_key(&mut app, egui::Key::ArrowDown, 0.2);
    assert_eq!(app.selected_id, Some(30));

    send_key(&mut app, egui::Key::Home, 0.3);
    assert_eq!(app.selected_id, Some(10));

    send_key(&mut app, egui::Key::End, 0.4);
    assert_eq!(app.selected_id, Some(30));

    send_key(&mut app, egui::Key::ArrowUp, 0.5);
    assert_eq!(app.selected_id, Some(20));

    send_key(&mut app, egui::Key::Delete, 0.6);
    assert_eq!(app.items.len(), 2);
    assert_eq!(app.selected_id, Some(30));
}

#[test]
fn queue_rows_and_language_model_rows_never_overlap_and_keep_exact_gaps() {
    let _language = lock_ui_language();
    let ctx = egui::Context::default();
    theme::install_fonts(&ctx);
    theme::apply(&ctx, ThemeKind::Light);

    // 1. Queue Workspace: Pending, Converting, Completed, and Failed cards
    let mut item_conv = SubtitleItem::new(2, PathBuf::from("/movies/Two.hrv.sup"), "hrv", None);
    item_conv.status =
        ItemStatus::Converting { cue: 42, total: 100, start_ts: 90_000, stop_ts: 180_000, last_text: "Bok!".into() };
    let mut item_done = SubtitleItem::new(3, PathBuf::from("/movies/Three.deu.sup"), "deu", None);
    item_done.status = ItemStatus::Completed { cues_count: 180 };
    let mut item_fail = SubtitleItem::new(4, PathBuf::from("/movies/Four.fra.sup"), "fra", None);
    item_fail.status = ItemStatus::Failed("Missing model".into());

    let mut app = PgsApp::default();
    app.items = vec![SubtitleItem::new(1, PathBuf::from("/movies/One.eng.sup"), "eng", None), item_conv, item_done, item_fail];
    app.selected_id = Some(1);
    app.installed_langs = vec!["eng".into(), "hrv".into()];
    app.catalog = vec![
        ModelInfo { code: "bos".into(), size_bytes: 2_400_000, ..Default::default() }, // uninstalled above installed
        ModelInfo { code: "hrv".into(), size_bytes: 3_100_000, ..Default::default() }, // installed above uninstalled
        ModelInfo { code: "deu".into(), size_bytes: 5_800_000, ..Default::default() }, // uninstalled above uninstalled
        ModelInfo { code: "srp".into(), size_bytes: 2_900_000, ..Default::default() }, // uninstalled above installed
        ModelInfo { code: "eng".into(), size_bytes: 4_100_000, ..Default::default() }, // installed
    ];

    let queue_shapes = render_headless_frame(&mut app, &ctx, 1180.0, 820.0);
    let mut queue_rects: Vec<Rect> = queue_shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Rect(r) if r.rect.width() > 400.0 && (r.rect.height() - QUEUE_ROW_HEIGHT).abs() < 0.5 => Some(r.rect),
            _ => None,
        })
        .collect();
    queue_rects.sort_by(|a, b| a.top().partial_cmp(&b.top()).unwrap());
    assert_eq!(queue_rects.len(), 4, "Expected 4 painted queue cards");
    for pair in queue_rects.windows(2) {
        let gap = pair[1].top() - pair[0].bottom();
        assert!(
            (gap - 8.0).abs() < 0.05,
            "Queue rows overlapped or lost 8pt spacing: prev={:?}, next={:?}, gap={gap}",
            pair[0],
            pair[1]
        );
    }

    // 2. OCR Languages Workspace: mixed uninstalled ("Download" button) and installed ("Installed" badge) rows
    app.tab = WorkspaceTab::Languages;
    let model_shapes = render_headless_frame(&mut app, &ctx, 1180.0, 820.0);
    let mut model_rects: Vec<Rect> = model_shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Rect(r) if r.rect.width() > 400.0 && (r.rect.height() - MODEL_ROW_HEIGHT).abs() < 0.5 => Some(r.rect),
            _ => None,
        })
        .collect();
    model_rects.sort_by(|a, b| a.top().partial_cmp(&b.top()).unwrap());
    assert_eq!(model_rects.len(), 5, "Expected 5 painted OCR language model rows");
    for pair in model_rects.windows(2) {
        let gap = pair[1].top() - pair[0].bottom();
        assert!(
            (gap - 6.0).abs() < 0.05,
            "OCR language rows overlapped or lost 6pt spacing: prev={:?}, next={:?}, gap={gap}",
            pair[0],
            pair[1]
        );
    }
}

#[test]
fn ui_language_selector_popup_fits_width_tightly_and_switches_locale() {
    let _language = lock_ui_language();
    let ctx = egui::Context::default();
    theme::install_fonts(&ctx);
    theme::apply(&ctx, ThemeKind::Light);
    let mut app = PgsApp::default();
    app.set_ui_language(UiLanguage::English);

    let run_events = |app: &mut PgsApp, events: Vec<egui::Event>, time: f64| {
        let raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1280.0, 800.0))),
            time: Some(time),
            events,
            ..Default::default()
        };
        let mut out = ctx.run_ui(raw, |ui| {
            app.ui_tab_strip(ui);
            app.ui_command_toolbar(ui);
            app.ui_status_bar(ui);
            app.ui_inspector_panel(ui);
            let t = Tokens::get(ui.ctx());
            egui::CentralPanel::default().frame(Frame::NONE.fill(t.pasteboard)).show(ui, |ui| app.ui_empty_queue(ui));
        });
        out.textures_delta.clear();
        out.shapes
    };

    let shapes = run_events(&mut app, vec![], 0.0);
    let ui_lang_btn_pos = shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.job.text == "EN" && t.pos.y < TAB_STRIP_HEIGHT => {
                Some(pos2(t.pos.x + 4.0, t.pos.y + 4.0))
            }
            _ => None,
        })
        .next()
        .expect("Expected top-bar UI language dropdown button ('EN')");

    // Click to open the UI language popup
    run_events(
        &mut app,
        vec![
            egui::Event::PointerMoved(ui_lang_btn_pos),
            egui::Event::PointerButton {
                pos: ui_lang_btn_pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
        0.05,
    );
    run_events(
        &mut app,
        vec![egui::Event::PointerButton {
            pos: ui_lang_btn_pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
        0.08,
    );
    let popup_shapes = run_events(&mut app, vec![], 0.12);
    assert!(egui::Popup::is_any_open(&ctx), "UI language popup must open on click");

    let hr_pos = popup_shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.job.text == "Hrvatski (HR)" => Some(pos2(t.pos.x + 12.0, t.pos.y + 4.0)),
            _ => None,
        })
        .next()
        .expect("Expected 'Hrvatski (HR)' item inside open UI language popup");

    // Verify the painted popup menu container rect fits tightly (never stretching to default 600pt)
    fn collect_rects(shape: &egui::Shape, out: &mut Vec<Rect>) {
        match shape {
            egui::Shape::Rect(r) => out.push(r.rect),
            egui::Shape::Vec(shapes) => {
                for s in shapes {
                    collect_rects(s, out);
                }
            }
            _ => {}
        }
    }
    let mut all_rects = Vec::new();
    for cs in &popup_shapes {
        collect_rects(&cs.shape, &mut all_rects);
    }
    let popup_frame_rect = all_rects
        .into_iter()
        .filter(|r| r.contains(hr_pos) && r.height() > 50.0 && r.height() < 220.0)
        .max_by(|a, b| a.height().total_cmp(&b.height()))
        .expect("Expected painted UI language popup frame rect");

    assert!(
        popup_frame_rect.width() >= 116.0 && popup_frame_rect.width() <= 190.0,
        "UI language popup width must fit tightly around locale labels, got {} ({popup_frame_rect:?})",
        popup_frame_rect.width()
    );

    // Click "Hrvatski (HR)" and verify locale changes and popup closes
    run_events(
        &mut app,
        vec![
            egui::Event::PointerMoved(hr_pos),
            egui::Event::PointerButton {
                pos: hr_pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
        0.16,
    );
    run_events(
        &mut app,
        vec![egui::Event::PointerButton {
            pos: hr_pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
        0.20,
    );
    run_events(&mut app, vec![], 0.24);
    assert_eq!(app.preferences.ui_language, UiLanguage::Croatian);
    assert!(!egui::Popup::is_any_open(&ctx));
}

#[test]
fn splitter_position_restores_from_preferences_and_persists() {
    let _language = lock_ui_language();
    let ctx = egui::Context::default();
    theme::install_fonts(&ctx);
    theme::apply(&ctx, ThemeKind::Light);

    let mut app = PgsApp::default();
    app.preferences.splitter_position = 420.0;
    render_headless_frame(&mut app, &ctx, 1180.0, 740.0);

    let state = egui::containers::panel::PanelState::load(&ctx, inspector_panel_id()).expect("inspector_panel state must exist");
    assert_eq!(state.outer_rect.width(), 420.0);
    assert_eq!(app.preferences.splitter_position, 420.0);
}

#[test]
fn show_inspector_restores_from_preferences() {
    let _language = lock_ui_language();
    let ctx = egui::Context::default();
    theme::install_fonts(&ctx);
    theme::apply(&ctx, ThemeKind::Light);

    let mut app = PgsApp::default();
    app.preferences.show_inspector = false;
    app.show_inspector = false;
    render_headless_frame(&mut app, &ctx, 1180.0, 740.0);

    let state = egui::containers::panel::PanelState::load(&ctx, inspector_panel_id());
    assert!(state.is_none(), "Inspector panel must not be rendered when show_inspector is false");
}

#[test]
fn window_resizer_hit_test_corners_and_edges() {
    let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 600.0));
    let border = WindowResizer::DEFAULT_BORDER_THICKNESS;
    let corner = WindowResizer::DEFAULT_CORNER_SIZE;

    // Corners
    assert_eq!(WindowResizer::hit_test(rect, pos2(4.0, 4.0), border, corner), Some(egui::ResizeDirection::NorthWest));
    assert_eq!(WindowResizer::hit_test(rect, pos2(996.0, 4.0), border, corner), Some(egui::ResizeDirection::NorthEast));
    assert_eq!(WindowResizer::hit_test(rect, pos2(4.0, 596.0), border, corner), Some(egui::ResizeDirection::SouthWest));
    assert_eq!(WindowResizer::hit_test(rect, pos2(996.0, 596.0), border, corner), Some(egui::ResizeDirection::SouthEast));

    // Edges
    assert_eq!(WindowResizer::hit_test(rect, pos2(500.0, 3.0), border, corner), Some(egui::ResizeDirection::North));
    assert_eq!(WindowResizer::hit_test(rect, pos2(500.0, 597.0), border, corner), Some(egui::ResizeDirection::South));
    assert_eq!(WindowResizer::hit_test(rect, pos2(3.0, 300.0), border, corner), Some(egui::ResizeDirection::West));
    assert_eq!(WindowResizer::hit_test(rect, pos2(997.0, 300.0), border, corner), Some(egui::ResizeDirection::East));

    // Interior content area
    assert_eq!(WindowResizer::hit_test(rect, pos2(500.0, 300.0), border, corner), None);
    assert_eq!(WindowResizer::hit_test(rect, pos2(20.0, 20.0), border, corner), None);

    // Exterior area
    assert_eq!(WindowResizer::hit_test(rect, pos2(-10.0, 300.0), border, corner), None);
    assert_eq!(WindowResizer::hit_test(rect, pos2(1010.0, 300.0), border, corner), None);
}

#[test]
fn window_resizer_cursor_mapping() {
    assert_eq!(WindowResizer::cursor_for_direction(egui::ResizeDirection::North), egui::CursorIcon::ResizeNorth);
    assert_eq!(WindowResizer::cursor_for_direction(egui::ResizeDirection::South), egui::CursorIcon::ResizeSouth);
    assert_eq!(WindowResizer::cursor_for_direction(egui::ResizeDirection::East), egui::CursorIcon::ResizeEast);
    assert_eq!(WindowResizer::cursor_for_direction(egui::ResizeDirection::West), egui::CursorIcon::ResizeWest);
    assert_eq!(WindowResizer::cursor_for_direction(egui::ResizeDirection::NorthEast), egui::CursorIcon::ResizeNorthEast);
    assert_eq!(WindowResizer::cursor_for_direction(egui::ResizeDirection::NorthWest), egui::CursorIcon::ResizeNorthWest);
    assert_eq!(WindowResizer::cursor_for_direction(egui::ResizeDirection::SouthEast), egui::CursorIcon::ResizeSouthEast);
    assert_eq!(WindowResizer::cursor_for_direction(egui::ResizeDirection::SouthWest), egui::CursorIcon::ResizeSouthWest);
}

#[test]
fn window_resizer_handles_pointer_input_and_respects_maximized() {
    let _language = lock_ui_language();
    let ctx = egui::Context::default();
    let resizer = WindowResizer::default();

    // Frame with pointer in top-left resize zone
    let raw = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 600.0))),
        events: vec![egui::Event::PointerMoved(pos2(3.0, 3.0))],
        ..Default::default()
    };
    let mut out1 = ctx.run_ui(raw, |ui| {
        let dir = resizer.handle(ui.ctx());
        assert_eq!(dir, Some(egui::ResizeDirection::NorthWest));
        assert!(resizer.is_in_resize_zone(ui.ctx()));
    });
    out1.textures_delta.clear();

    // Frame with pointer in center
    let raw_center = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 600.0))),
        events: vec![egui::Event::PointerMoved(pos2(500.0, 300.0))],
        ..Default::default()
    };
    let mut out2 = ctx.run_ui(raw_center, |ui| {
        let dir = resizer.handle(ui.ctx());
        assert_eq!(dir, None);
        assert!(!resizer.is_in_resize_zone(ui.ctx()));
    });
    out2.textures_delta.clear();
}

#[test]
fn clean_component_views_render_and_emit_typed_actions_in_isolation() {
    let _language = lock_ui_language();
    let ctx = egui::Context::default();
    theme::install_fonts(&ctx);
    theme::apply(&ctx, ThemeKind::Light);

    // 1. StatusBarView
    let status_view = StatusBarView::new(2, 4, Some("hrv"), 5);
    let raw =
        egui::RawInput { screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 600.0))), ..Default::default() };
    let mut out_status = ctx.run_ui(raw.clone(), |ui| {
        status_view.show(ui);
    });
    out_status.textures_delta.clear();

    // 2. ToastView
    let toast_view = ToastView::new("Test Notification", false);
    let mut out_toast = ctx.run_ui(raw.clone(), |ui| {
        toast_view.show(ui.ctx());
    });
    out_toast.textures_delta.clear();

    // 3. ToolbarView
    let toolbar_view = ToolbarView::new(false, 3, true, None, 4);
    let mut toolbar_actions = Vec::new();
    let mut out_toolbar = ctx.run_ui(raw.clone(), |ui| {
        toolbar_view.show(ui, &mut toolbar_actions);
    });
    out_toolbar.textures_delta.clear();

    // 4. TitleBarView
    let titlebar_view =
        TitleBarView::new(WorkspaceTab::Queue, 2, 6, true, ui_egui::ThemePreference::System, UiLanguage::English, true);
    let mut titlebar_actions = Vec::new();
    let mut out_titlebar = ctx.run_ui(raw.clone(), |ui| {
        titlebar_view.show(ui, &mut titlebar_actions);
    });
    out_titlebar.textures_delta.clear();

    // 5. EmptyQueueView
    let mut empty_actions = Vec::new();
    let mut out_empty = ctx.run_ui(raw.clone(), |ui| {
        EmptyQueueView::default().show(ui, &mut empty_actions);
    });
    out_empty.textures_delta.clear();

    // 6. LanguagesView
    let catalog = vec![
        ModelInfo { code: "hrv".to_string(), size_bytes: 1024 * 1024, ..Default::default() },
        ModelInfo { code: "eng".to_string(), size_bytes: 4 * 1024 * 1024, ..Default::default() },
    ];
    let installed = vec!["eng".to_string()];
    let lang_view = LanguagesView::new(&catalog, &installed, None, ModelFilter::All, "", UiLanguage::English, None);
    let mut lang_actions = Vec::new();
    let mut out_lang = ctx.run_ui(raw.clone(), |ui| {
        lang_view.show(ui, &mut lang_actions);
    });
    out_lang.textures_delta.clear();

    // 7. InspectorView (empty state)
    let mut search = String::new();
    let mut highlight = 0;
    let mut inspector_view =
        InspectorView::new(None, 340.0, &installed, UiLanguage::English, true, None, None, None, &mut search, &mut highlight);
    let mut inspector_actions = Vec::new();
    let mut out_insp = ctx.run_ui(raw, |ui| {
        inspector_view.show(ui, &mut inspector_actions);
    });
    out_insp.textures_delta.clear();
}
