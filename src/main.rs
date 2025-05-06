mod encryptor;

use eframe::egui;
use std::env;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // 测试使用默认选项，不设置窗口大小，避免与不同版本的eframe不兼容，test_comitt2
    let options = eframe::NativeOptions::default();

    // 处理返回的Result以避免警告
    if let Err(e) = eframe::run_native(
        "Folder Encryptor",
        options,
        Box::new(|cc| {
            // 配置字体和样式
            setup_custom_fonts(&cc.egui_ctx);
            Box::new(MyApp::default())
        }),
    ) {
        eprintln!("Error: {}", e);
    }
}

fn setup_custom_fonts(ctx: &egui::Context) {
    // 加载默认字体
    let fonts = egui::FontDefinitions::default();

    // 尝试使用系统内置的中文友好字体
    #[cfg(target_os = "windows")]
    {
        fonts.font_data.insert(
            "msyh".to_owned(),
            egui::FontData::from_owned(
                std::fs::read("C:\\Windows\\Fonts\\msyh.ttc").unwrap_or_else(|_| vec![]), // 读取失败则返回空向量
            ),
        );
        if !fonts
            .font_data
            .get("msyh")
            .map_or(true, |f| f.font.is_empty())
        {
            fonts
                .families
                .get_mut(&egui::FontFamily::Proportional)
                .unwrap()
                .insert(0, "msyh".to_owned());
        }
    }

    // 不再尝试加载macOS特定字体，避免找不到字体的错误

    // 设置较大的默认字体大小
    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (
            egui::TextStyle::Heading,
            egui::FontId::new(24.0, egui::FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Body,
            egui::FontId::new(18.0, egui::FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Button,
            egui::FontId::new(18.0, egui::FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Small,
            egui::FontId::new(14.0, egui::FontFamily::Proportional),
        ),
    ]
    .into();

    ctx.set_fonts(fonts);
    ctx.set_style(style);
}

enum OperationResult {
    Success(String),
    Error(String),
    None,
}

struct MyApp {
    selected_path: Option<String>,
    password: String,
    encrypting: bool,
    decrypting: bool,
    status_message: Option<String>,
    operation_in_progress: bool,
    operation_result: Arc<Mutex<OperationResult>>,
    is_encrypt_mode: bool,
    show_password: bool,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            selected_path: None,
            password: String::new(),
            encrypting: false,
            decrypting: false,
            status_message: None,
            operation_in_progress: false,
            operation_result: Arc::new(Mutex::new(OperationResult::None)),
            is_encrypt_mode: true,
            show_password: false,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 检查上次操作的结果
        let mut result = OperationResult::None;
        {
            let mut locked_result = self.operation_result.lock().unwrap();
            std::mem::swap(&mut result, &mut *locked_result);
        }

        // 如果有结果，更新状态
        match result {
            OperationResult::Success(message) => {
                self.operation_in_progress = false;
                self.status_message =
                    Some(message + " (Please select folder/file again to refresh view)");
                self.selected_path = None; // 清除选中路径，强制用户重新选择
            }
            OperationResult::Error(error) => {
                self.operation_in_progress = false;
                self.status_message = Some(format!("Error: {}", error));
            }
            OperationResult::None => {}
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            // 居中显示标题
            ui.vertical_centered(|ui| {
                ui.heading("Folder Operations");
            });
            ui.add_space(10.0);

            // 用一个框包裹主要内容
            egui::Frame::none()
                .inner_margin(egui::Margin::same(20.0))
                .fill(ui.style().visuals.window_fill)
                .stroke(ui.style().visuals.widgets.noninteractive.bg_stroke)
                .rounding(egui::Rounding::same(8.0))
                .show(ui, |ui| {
                    ui.vertical_centered_justified(|ui| {
                        ui.horizontal(|ui| {
                            if ui.radio_value(&mut self.is_encrypt_mode, true, "Encrypt Mode").clicked() {
                                self.selected_path = None;
                                self.status_message = None;
                            }
                            if ui.radio_value(&mut self.is_encrypt_mode, false, "Decrypt Mode").clicked() {
                                self.selected_path = None;
                                self.status_message = None;
                            }
                        });

                        ui.add_space(10.0);

                        if ui.button(if self.is_encrypt_mode { "Select Folder to Encrypt" } else { "Select File to Decrypt" }).clicked() {
                            // 确保每次选择前清除旧的状态
                            self.status_message = None;

                            // 短暂延迟，确保文件系统状态已更新
                            // 注意：在生产环境中应考虑更稳健的方式
                            std::thread::sleep(std::time::Duration::from_millis(100));

                            if self.is_encrypt_mode {
                                // 选择文件夹进行加密
                                if let Some(dir) = rfd::FileDialog::new()
                                    .set_directory(std::env::current_dir().unwrap_or_default()) // 重新设置为当前目录，强制刷新
                                    .pick_folder()
                                {
                                    self.selected_path = Some(dir.display().to_string());
                                    self.status_message = None;
                                }
                            } else {
                                // 选择加密文件进行解密
                                if let Some(file) = rfd::FileDialog::new()
                                    .set_directory(std::env::current_dir().unwrap_or_default()) // 重新设置为当前目录，强制刷新
                                    .add_filter("Encrypted files", &["aes"])
                                    .set_title("Select encrypted file")
                                    .pick_file()
                                {
                                    self.selected_path = Some(file.display().to_string());
                                    self.status_message = None;
                                }
                            }
                        }

                        ui.add_space(10.0);

                        ui.horizontal(|ui| {
                            ui.label("Password:");

                            let _response = ui.add(
                                egui::TextEdit::singleline(&mut self.password)
                                    .password(!self.show_password) // 当show_password为false时显示密码掩码
                            );

                            // 添加一个切换密码显示状态的按钮
                            if ui.button(if self.show_password { "Hide" } else { "Show" }).clicked() {
                                self.show_password = !self.show_password;
                            }
                        });

                        ui.add_space(10.0);

                        ui.add_enabled_ui(self.selected_path.is_some() && !self.operation_in_progress, |ui| {
                            if self.is_encrypt_mode {
                                if ui.button("Encrypt Folder").clicked() {
                                    self.operation_in_progress = true;
                                    self.encrypting = true;
                                    self.decrypting = false;
                                    self.status_message = Some("Encrypting folder...".to_string());

                                    if let Some(folder_path) = self.selected_path.clone() {
                                        // 检查是否为目录
                                        if !Path::new(&folder_path).is_dir() {
                                            self.operation_in_progress = false;
                                            self.status_message = Some(format!("Error: '{}' is not a directory", folder_path));
                                            return;
                                        }

                                        let password = self.password.clone();
                                        let result_arc = self.operation_result.clone();
                                        let ctx = ctx.clone();

                                        // 在新线程中执行加密操作，以避免阻塞UI
                                        thread::spawn(move || {
                                            let result = encryptor::encrypt_folder(&folder_path, &password);

                                            // 存储结果
                                            let operation_result = match result {
                                                Ok(message) => OperationResult::Success(message + " (Please select folder/file again to refresh view)"),
                                                Err(err) => OperationResult::Error(err),
                                            };

                                            // 更新共享状态
                                            {
                                                let mut locked_result = result_arc.lock().unwrap();
                                                *locked_result = operation_result;
                                            }

                                            // 通知UI需要更新
                                            ctx.request_repaint();
                                        });
                                    }
                                }
                            } else {
                                if ui.button("Decrypt File").clicked() {
                                    self.operation_in_progress = true;
                                    self.encrypting = false;
                                    self.decrypting = true;
                                    self.status_message = Some("Decrypting file...".to_string());

                                    if let Some(file_path) = self.selected_path.clone() {
                                        // 检查是否为文件
                                        if !Path::new(&file_path).is_file() {
                                            self.operation_in_progress = false;
                                            self.status_message = Some(format!("Error: '{}' is not a file", file_path));
                                            return;
                                        }

                                        let password = self.password.clone();
                                        let result_arc = self.operation_result.clone();
                                        let ctx = ctx.clone();

                                        // 在新线程中执行解密操作，以避免阻塞UI
                                        thread::spawn(move || {
                                            let result = encryptor::decrypt_folder(&file_path, &password);

                                            // 存储结果
                                            let operation_result = match result {
                                                Ok(message) => OperationResult::Success(message + " (Please select folder/file again to refresh view)"),
                                                Err(err) => OperationResult::Error(err),
                                            };

                                            // 更新共享状态
                                            {
                                                let mut locked_result = result_arc.lock().unwrap();
                                                *locked_result = operation_result;
                                            }

                                            // 通知UI需要更新
                                            ctx.request_repaint();
                                        });
                                    }
                                }
                            }
                        });

                        ui.add_space(15.0);

                        if let Some(path) = self.selected_path.clone() {
                            ui.horizontal(|ui| {
                                ui.label(format!("Selected path: {}", path));

                                // 添加刷新按钮
                                if ui.button("🔄 Refresh").clicked() {
                                    // 清除选中路径，强制用户重新选择
                                    self.selected_path = None;
                                    self.status_message = Some("Please select a folder or file again to see latest changes".to_string());
                                }
                            });
                        }

                        if let Some(ref message) = self.status_message {
                            ui.add_space(10.0);
                            let text_color = if message.starts_with("Error") {
                                ui.style().visuals.error_fg_color
                            } else {
                                ui.style().visuals.text_color()
                            };

                            ui.colored_label(text_color, message);
                        }

                        if self.operation_in_progress {
                            ui.add_space(10.0);
                            ui.horizontal_centered(|ui| {
                                ui.spinner();
                                ui.label("Operation in progress...");
                            });
                        }
                    });
                });
        });
    }
}
