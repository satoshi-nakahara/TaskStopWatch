# ![windows_subsystem = "windows"]
use eframe::egui;
use egui::FontFamily;
use egui::{RichText, Color32};
use chrono::Local;
use std::time::{Duration, Instant};
use arboard::Clipboard;

fn main() -> eframe::Result<()> {
    // ウィンドウの設定
        let options = eframe::NativeOptions {
        viewport: egui::viewport::ViewportBuilder::default()
            .with_inner_size([200.0, 100.0]),
        ..Default::default()
    };
    
    // 引数から作業名を取得（なければデフォルト）
    let task_name = std::env::args().nth(1).unwrap_or_else(|| "作業".to_owned());

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
            Ok(Box::new(MyApp::new(task_name.clone())))
        }),
    )
}

// フォント設定用の関数
fn setup_custom_fonts(ctx: &egui::Context) {
    // フォント設定を取得
    let mut fonts = egui::FontDefinitions::default();
    
    // 日本語フォント（可変ウェイト）を追加
    fonts.font_data.insert(
        "noto_sans_jp".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/NotoSansJP-VariableFont_wght.ttf")).into(),
    );
    
    // フォントファミリーに追加
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "noto_sans_jp".to_owned()); // 一番優先度高く追加
    
    // モノスペースフォントにも日本語フォントを追加
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .push("noto_sans_jp".to_owned());
    
    // フォント設定を適用
    ctx.set_fonts(fonts);
    // テキストを黒に固定する
    //ctx.style_mut(|s| s.visuals.override_text_color = Some(Color32::BLACK));
}

// アプリケーションの状態を保持する構造体
struct MyApp {
    name: String,
    start: Instant,
    clip_msg: Option<String>,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            name: "山田太郎".to_owned(),
            start: Instant::now(),
            clip_msg: None,
        }
    }
}

impl MyApp {
    fn new(name: String) -> Self {
        Self {
            name,
            start: Instant::now(),
            clip_msg: None,
        }
    }
}

// アプリケーションの描画とロジックを実装
impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {

            // 経過時間を hh:mm:ss 形式で表示
            let elapsed = self.start.elapsed();
            let secs = elapsed.as_secs();
            let hh = secs / 3600;
            let mm = (secs % 3600) / 60;
            let ss = secs % 60;
            ui.label(
                RichText::new(format!("{:02}:{:02}:{:02}", hh, mm, ss))
                    .color(Color32::BLACK)
                    .size(20.0)       // フォントサイズ（ポイント）
                    .strong()         // 太字
            );
            // 作業名を表示（明示的に黒で描画）
            ui.label(
                RichText::new(&self.name)
                    .color(Color32::BLACK)
                    .size(14.0)       // フォントサイズ（ポイント）
                    // .strong()         // 太字
            );
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
    }
}
