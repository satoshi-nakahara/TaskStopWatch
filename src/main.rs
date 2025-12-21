# ![windows_subsystem = "windows"]
use eframe::egui;
use egui::FontFamily;
use egui::{RichText, Color32};
use chrono::Local;
use chrono::Duration as ChronoDuration;
use std::time::{Duration, Instant};
use arboard::Clipboard;
use egui::Sense;
use egui::Key;

fn main() -> eframe::Result<()> {
    // 引数から作業名を取得（なければデフォルト）
    let task_name = std::env::args().nth(1).unwrap_or_else(|| "作業".to_owned());
    // 第二引数をメモのデフォルト表示に使う（なければMarkdownでの太字表示を示すトークン）
    // HTML の <br> を改行に置換する
    let memo_default_raw = std::env::args().nth(2).unwrap_or_else(|| "（memo）".to_owned());
    let memo_default = memo_default_raw.replace("<br/>", "\n").replace("<br>", "\n");

    // ウィンドウの設定: memo の内容に合わせて初期サイズを計算する
    // 最大行長と行数を正しく計算
    let memo_lines_count = memo_default.lines().count().max(1);
    // let max_chars = memo_default
    //     .lines()
    //     .map(|l| l.chars().count())
    //     .max()
    //     .unwrap_or(20usize)
    //     .max(20);
    // 単純な推定: 1文字あたり約8px、1行あたり20px として計算
    // let width = (200.0 + (max_chars as f32) * 8.0).clamp(200.0, 1200.0);
    let width = 300.0_f32;
    // 高さは固定要素分 + メモ行分 + ボタンが隠れない余白を確保
    let fixed_ui_height = 140.0; // ヘッダ、タイマー、余白など
    let memo_area = (memo_lines_count as f32) * 22.0; // 1行あたりの高さ
    let button_pad = 80.0; // 完了ボタンと余白のためのスペース
    let height = (fixed_ui_height + memo_area + button_pad).clamp(140.0, 1400.0);
    let options = eframe::NativeOptions {
        viewport: egui::viewport::ViewportBuilder::default()
            .with_inner_size([width, height]),
        ..Default::default()
    };
    // 第三引数は分数（整数）で受け取る（オプション）
    let end_minutes = std::env::args().nth(3).and_then(|s| s.parse::<u64>().ok());

    // アプリケーションの実行
    eframe::run_native(
        "Stop Watch",
        options,
        Box::new(move |cc| {
            // 日本語フォントの設定
            setup_custom_fonts(&cc.egui_ctx);
            // Windows では Win32 API を使ってウィンドウを最前面に設定する
            #[cfg(target_os = "windows")]
            {
                let title = "Stop Watch".to_string();
                std::thread::spawn(move || {
                    // 少し待ってウィンドウが作られるのを待つ
                    std::thread::sleep(std::time::Duration::from_millis(200));
                    use widestring::U16CString;
                    use winapi::shared::windef::HWND;
                    use winapi::um::winuser::{FindWindowW, SetWindowPos, SWP_NOMOVE, SWP_NOSIZE, HWND_TOPMOST};

                    let wide = U16CString::from_str(&title).unwrap();
                    unsafe {
                        let hwnd: HWND = FindWindowW(std::ptr::null_mut(), wide.as_ptr());
                        if !hwnd.is_null() {
                            SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
                        }
                    }
                });
            }
            Ok(Box::new(MyApp::new(task_name.clone(), memo_default.clone(), end_minutes)))
        }),
    )
}

// フォント設定用の関数
fn setup_custom_fonts(ctx: &egui::Context) {
    // フォント設定を取得
    let mut fonts = egui::FontDefinitions::default();
    // Try to load Meiryo from Windows font directory when on Windows.
    // Fallback to bundled NotoSansJP if Meiryo isn't available.
    #[cfg(target_os = "windows")]
    {
        // common Meiryo filenames to try
        let candidates = [
            r"C:\\Windows\\Fonts\\meiryo.ttf",
            r"C:\\Windows\\Fonts\\meiryob.ttf",
            r"C:\\Windows\\Fonts\\meiryo.ttc",
            r"C:\\Windows\\Fonts\\meiryob.ttc",
        ];
        let mut loaded_meiryo = false;
        for p in &candidates {
            if let Ok(bytes) = std::fs::read(p) {
                fonts.font_data.insert(
                    "meiryo".to_owned(),
                    egui::FontData::from_owned(bytes).into(),
                );
                // Put Meiryo at highest priority for proportional family
                fonts
                    .families
                    .entry(FontFamily::Proportional)
                    .or_default()
                    .insert(0, "meiryo".to_owned());
                // also add to monospace family as a fallback for CJK characters
                fonts
                    .families
                    .entry(FontFamily::Monospace)
                    .or_default()
                    .push("meiryo".to_owned());
                loaded_meiryo = true;
                break;
            }
        }

        if !loaded_meiryo {
            // fallback to bundled Noto
            fonts.font_data.insert(
                "noto_sans_jp".to_owned(),
                egui::FontData::from_static(include_bytes!("../assets/NotoSansJP-VariableFont_wght.ttf")).into(),
            );
            fonts
                .families
                .entry(FontFamily::Proportional)
                .or_default()
                .insert(0, "noto_sans_jp".to_owned()); // 一番優先度高く追加
            fonts
                .families
                .entry(FontFamily::Monospace)
                .or_default()
                .push("noto_sans_jp".to_owned());
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // on non-Windows, use bundled Noto
        fonts.font_data.insert(
            "noto_sans_jp".to_owned(),
            egui::FontData::from_static(include_bytes!("../assets/NotoSansJP-VariableFont_wght.ttf")).into(),
        );
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "noto_sans_jp".to_owned()); // 一番優先度高く追加
        fonts
            .families
            .entry(FontFamily::Monospace)
            .or_default()
            .push("noto_sans_jp".to_owned());
    }
    
    // フォント設定を適用
    ctx.set_fonts(fonts);
    // テキストを黒に固定する
    //ctx.style_mut(|s| s.visuals.override_text_color = Some(Color32::BLACK));

    // 背景色を黄色にする（egui の Visuals を使う）
    ctx.style_mut(|s| {
        // Light モードをベースにして背景を置き換えるのが素直です
        //s.visuals = egui::Visuals::light();
        // 背景（ウィンドウの底面）を黄色に設定
        s.visuals.window_fill = egui::Color32::from_rgb(255, 255, 0); // やや薄い黄色
        // 必要なら他の要素色も調整
        s.visuals.panel_fill = egui::Color32::from_rgb(255, 255, 0);
        s.visuals.override_text_color = Some(egui::Color32::BLACK); // 文字は黒に
    });

}

// アプリケーションの状態を保持する構造体
struct MyApp {
    name: String,
    name_edit: bool,
    start: Instant,
    clip_msg: Option<String>,
    // memo state: the full text, and whether we're in edit mode
    memo: String,
    memo_edit: bool,
    // optional end time (countdown) in Instant and local datetime for display
    end_instant: Option<Instant>,
    end_time_local: Option<chrono::DateTime<Local>>,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            name: "山田太郎".to_owned(),
            name_edit: false,
            start: Instant::now(),
            clip_msg: None,
            memo: "**メモを編集**".to_owned(),
            memo_edit: false,
            end_instant: None,
            end_time_local: None,
        }
    }
}

impl MyApp {
    fn new(name: String, memo_default: String, end_minutes: Option<u64>) -> Self {
        let start = Instant::now();
        // compute optional end times
        let (end_instant, end_time_local) = if let Some(m) = end_minutes {
            let secs = m.saturating_mul(60);
            let ei = start + Duration::from_secs(secs);
            let el = Local::now() + ChronoDuration::minutes(m as i64);
            (Some(ei), Some(el))
        } else {
            (None, None)
        };

        Self {
            name,
            name_edit: false,
            start,
            clip_msg: None,
            memo: memo_default,
            memo_edit: false,
            end_instant,
            end_time_local,
        }
    }
}

fn new_frame(bgcolor: egui::Color32) -> egui::Frame {
    // Use the current egui API: Frame::new(), CornerRadius::same, Stroke::new
    egui::Frame::new()
        .fill(bgcolor)
        .stroke(egui::Stroke::new(0.0, egui::Color32::BLACK))
        .corner_radius(egui::CornerRadius::same(0))
}

// アプリケーションの描画とロジックを実装
impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        //egui::CentralPanel::default().show(ctx, |ui| {
        egui::CentralPanel::default()
            .frame(new_frame(egui::Color32::from_rgb(255, 250, 0)))
            .show(ctx, |ui| {
                // Add a small left margin (~2mm) in pixels computed from points and
                // the current pixels_per_point scaling so it respects DPI scaling.
                let mm = 2.0_f32;
                // convert mm -> inches -> points (1pt = 1/72 inch), then to pixels
                let points = (mm / 25.4_f32) * 72.0_f32;
                let left_px = points * ctx.pixels_per_point();

                ui.horizontal(|ui| {
                    ui.add_space(left_px);
                    ui.vertical(|ui| {
                        // 現在時刻と経過時間を計算
                        let now_instant = Instant::now();
                        let elapsed = now_instant.duration_since(self.start);
                        let secs = elapsed.as_secs();
                        let hh = secs / 3600;
                        let mm = (secs % 3600) / 60;
                        let ss = secs % 60;

                        // 終了時刻（あれば）を上部に表示（〆の右に残り/経過時間を表示）
                        if let Some(end_local) = &self.end_time_local {
                            if let Some(end_inst) = self.end_instant {
                                // compute remaining or over time text
                                if now_instant <= end_inst {
                                    let rem = end_inst.duration_since(now_instant);
                                    let rsecs = rem.as_secs();
                                    let rh = rsecs / 3600;
                                    let rm = (rsecs % 3600) / 60;
                                    let rs = rsecs % 60;
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(format!("〆{}", end_local.format("%H:%M")))
                                                .color(Color32::RED)
                                                .size(22.0)
                                                .strong(),
                                        );
                                        ui.add_space(8.0);
                                        ui.label(
                                            RichText::new(format!("残り時間 {:02}:{:02}:{:02}", rh, rm, rs))
                                                .color(Color32::BLACK)
                                                .size(14.0)
                                                .strong(),
                                        );
                                    });
                                } else {
                                    let over = now_instant.duration_since(end_inst);
                                    let osecs = over.as_secs();
                                    let oh = osecs / 3600;
                                    let om = (osecs % 3600) / 60;
                                    let os = osecs % 60;
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(format!("〆{}", end_local.format("%H:%M")))
                                                .color(Color32::RED)
                                                .size(22.0)
                                                .strong(),
                                        );
                                        ui.add_space(8.0);
                                        ui.label(
                                            RichText::new(format!("経過時間 {:02}:{:02}:{:02}", oh, om, os))
                                                .color(Color32::BLACK)
                                                .size(14.0)
                                                .strong(),
                                        );
                                    });
                                }
                            } else {
                                // end_time_local present but no instant (shouldn't happen normally)
                                ui.label(
                                    RichText::new(format!("〆{}", end_local.format("%H:%M")))
                                        .color(Color32::RED)
                                        .size(22.0)
                                        .strong(),
                                );
                            }
                        }

                        // 経過時間を hh:mm:ss 形式で表示
                        ui.label(
                            RichText::new(format!("経過時間 {:02}:{:02}:{:02}", hh, mm, ss))
                                .color(Color32::BLACK)
                                .size(20.0)       // フォントサイズ（ポイント）
                                .strong()         // 太字
                        );
                        // 作業名表示: ラベルモード / 編集モードを切り替え
                        if !self.name_edit {
                            // Make task name larger and bold for prominence
                            let label = RichText::new(&self.name)
                                .color(Color32::BLACK)
                                .size(22.0)
                                .strong();
                            if ui.add(egui::Label::new(label).sense(Sense::click())).clicked() {
                                self.name_edit = true;
                            }
                        } else {
                            // 編集モード: 1行入力。Esc で確定してラベルモードに戻る。
                            let mut name_buf = self.name.clone();
                            let _resp = ui.add(egui::TextEdit::singleline(&mut name_buf));
                            // イベントで Escape を検出
                            let events = ctx.input(|i| i.events.clone());
                            let mut commit = false;
                            for ev in events.iter() {
                                if let egui::Event::Key { key, pressed, .. } = ev {
                                    if *pressed && *key == Key::Escape {
                                        commit = true;
                                    }
                                }
                            }
                            // 反映
                            self.name = name_buf;
                            if commit {
                                self.name_edit = false;
                            }
                        }

                        // ----- メモ領域 -----
                        ui.separator();
                        if !self.memo_edit {
                            // ラベルモード: 行ごとにチェックボックス行を解釈して表示
                            let mut lines: Vec<String> = self.memo.lines().map(|s| s.to_string()).collect();
                            for i in 0..lines.len() {
                                let line = &lines[i];
                                let trimmed = line.trim_start();
                                if trimmed.starts_with("- [ ]") || trimmed.starts_with("- [x]") || trimmed.starts_with("- [X]") {
                                    // checkbox line
                                    let checked = trimmed.chars().nth(3) == Some('x') || trimmed.chars().nth(3) == Some('X');
                                    let rhs = trimmed[5..].trim_start().to_string();
                                    ui.horizontal(|ui| {
                                        let mut checked_bool = checked;
                                        let resp = ui.checkbox(&mut checked_bool, "");
                                        if resp.clicked() {
                                            // toggle in memo: modify lines vec and then assign back to self.memo
                                            let mut t = lines[i].clone();
                                            if t.contains("- [ ]") {
                                                t = t.replacen("- [ ]", "- [x]", 1);
                                            } else {
                                                t = t.replacen("- [x]", "- [ ]", 1);
                                                t = t.replacen("- [X]", "- [ ]", 1);
                                            }
                                            lines[i] = t;
                                            self.memo = lines.join("\n");
                                        }
                                        let label = RichText::new(rhs).color(Color32::BLACK);
                                        if ui.add(egui::Label::new(label).sense(Sense::click())).clicked() {
                                            self.memo_edit = true;
                                        }
                                    });
                                } else {
                                    // normal line: support **bold** naive replacement
                                    let text = if line.contains("**") {
                                        let s = line.replace("**", "");
                                        RichText::new(s).color(Color32::BLACK).strong()
                                    } else {
                                        RichText::new(line.as_str()).color(Color32::BLACK)
                                    };
                                    if ui.add(egui::Label::new(text).sense(Sense::click())).clicked() {
                                        self.memo_edit = true;
                                    }
                                }
                            }
                        } else {
                            // 編集モード: 複数行テキスト編集
                            let mut edit = self.memo.clone();
                            let _resp = ui.add(egui::TextEdit::multiline(&mut edit).desired_rows(6));
                            // 入力イベントを見て Escape を確定キー、Alt+Enter を改行にする
                            let mut commit = false;
                            let events = ctx.input(|i| i.events.clone());
                            for ev in events.iter() {
                                if let egui::Event::Key { key, pressed, modifiers, .. } = ev {
                                    if *pressed {
                                        if *key == Key::Escape {
                                            // Escape: 確定（編集終了）
                                            commit = true;
                                        } else if *key == Key::Enter {
                                            // Alt+Enter: 改行を挿入
                                            if modifiers.alt {
                                                edit.push('\n');
                                            }
                                        }
                                    }
                                }
                            }
                            // 編集内容を常に反映
                            self.memo = edit;
                            if commit {
                                self.memo_edit = false;
                            }
                        }
                        // 完了ボタン: 押されたら現在時刻を HHMM 形式でクリップボードに保存
                        if ui.add(egui::Button::new("完了!")).clicked() {
                            let now_hhmm = Local::now().format("%H%M").to_string();
                            match Clipboard::new() {
                                Ok(mut cb) => {
                                    match cb.set_text(now_hhmm.clone()) {
                                        Ok(()) => {
                                            self.clip_msg = Some(format!("{} をクリップボードにコピーしました", now_hhmm));
                                            // コピー成功したのでアプリを終了する
                                            std::process::exit(0);
                                        }
                                        Err(e) => self.clip_msg = Some(format!("クリップボード保存に失敗: {:?}", e)),
                                    }
                                }
                                Err(e) => {
                                    self.clip_msg = Some(format!("クリップボード初期化失敗: {:?}", e));
                                }
                            }
                        }

                        if let Some(msg) = &self.clip_msg {
                            ui.colored_label(Color32::BLACK, msg);
                        }

                        // リアルタイム更新を促す
                        ctx.request_repaint_after(Duration::from_millis(200));
                    });
                });
            });
    }
}
