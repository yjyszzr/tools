use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::io::Write;

pub fn encrypt_folder(folder_path: &str, password: &str) -> Result<String, String> {
    let folder_path = Path::new(folder_path);
    
    // 确保文件夹存在
    if !folder_path.is_dir() {
        return Err(format!("Folder '{}' does not exist", folder_path.display()));
    }
    
    // 获取父目录和文件夹名
    let parent_dir = folder_path.parent().unwrap_or(Path::new("."));
    let folder_name = folder_path.file_name()
        .ok_or("Cannot get folder name")?
        .to_string_lossy()
        .to_string();
    
    // 在父目录中创建加密文件
    let encrypted_file = parent_dir.join(format!("{}.aes", folder_name));
    let encrypted_file_path = encrypted_file.to_string_lossy().to_string();
    
    if cfg!(target_os = "macos") || cfg!(target_os = "linux") {
        // 在 macOS 或 Linux 上使用 tar 和 openssl
        
        // 创建临时文件名
        let temp_tar = parent_dir.join(format!("{}.tar", folder_name));
        let temp_tar_path = temp_tar.to_string_lossy().to_string();
        
        // 获取文件夹的绝对路径和名称
        let abs_folder_path = folder_path.canonicalize()
            .map_err(|e| format!("Failed to get absolute path: {}", e))?;
        let parent_abs_path = abs_folder_path.parent().unwrap_or(Path::new("/"));
        
        // 创建一个临时 tar 文件，保留完整目录结构
        // -C 选项指定父目录作为基准，然后只打包目标文件夹
        let tar_command = Command::new("tar")
            .args(&[
                "-cf", 
                &temp_tar_path, 
                "-C", 
                &parent_abs_path.to_string_lossy().to_string(), 
                &folder_name
            ])
            .output()
            .map_err(|e| format!("Failed to execute tar command: {}", e))?;
        
        if !tar_command.status.success() {
            return Err(format!("Failed to package folder: {}", String::from_utf8_lossy(&tar_command.stderr)));
        }
        
        // 使用 openssl 加密 tar 文件
        let mut openssl_command = Command::new("openssl");
        openssl_command
            .arg("enc")
            .arg("-aes-256-cbc")  // 使用 CBC 模式替代 GCM
            .arg("-salt")
            .arg("-pbkdf2")
            .arg("-pass")
            .arg("stdin")  // 从标准输入读取密码
            .arg("-in")
            .arg(&temp_tar_path)
            .arg("-out")
            .arg(&encrypted_file_path)
            .stdin(Stdio::piped()) // 设置标准输入
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        
        // 启动命令并获取子进程
        let mut child = openssl_command
            .spawn()
            .map_err(|e| format!("Failed to start openssl command: {}", e))?;
        
        // 将密码写入标准输入
        if let Some(stdin) = child.stdin.as_mut() {
            stdin.write_all(password.as_bytes())
                .map_err(|e| format!("Failed to write password: {}", e))?;
        } else {
            return Err("Unable to get openssl standard input".to_string());
        }
        
        // 等待命令完成
        let output = child.wait_with_output()
            .map_err(|e| format!("Failed to wait for openssl command to complete: {}", e))?;
        
        if !output.status.success() {
            return Err(format!("Failed to encrypt file: {}", String::from_utf8_lossy(&output.stderr)));
        }
        
        // 删除临时 tar 文件
        fs::remove_file(&temp_tar_path)
            .map_err(|e| format!("Failed to delete temporary file: {}", e))?;
        
    } else if cfg!(target_os = "windows") {
        // 在 Windows 上使用 7z
        
        let mut sevenzip_command = Command::new("7z");
        sevenzip_command.arg("a").arg("-t7z").arg("-mhe=on");
        
        // 添加密码参数（如果有）
        if !password.is_empty() {
            let password_arg = format!("-p{}", password);
            sevenzip_command.arg(password_arg);
        }
        
        let folder_path_str = folder_path.to_string_lossy().to_string();
        sevenzip_command.arg(&encrypted_file_path).arg(&folder_path_str);
        
        let output = sevenzip_command.output()
            .map_err(|e| format!("Failed to execute 7z command: {}", e))?;
        
        if !output.status.success() {
            return Err(format!("Failed to encrypt folder: {}", String::from_utf8_lossy(&output.stderr)));
        }
    } else {
        return Err("Unsupported operating system".to_string());
    }
    
    Ok(format!("Folder has been encrypted to: {}", encrypted_file_path))
}

pub fn decrypt_folder(encrypted_file: &str, password: &str) -> Result<String, String> {
    let encrypted_path = Path::new(encrypted_file);
    
    // 确保加密文件存在
    if !encrypted_path.exists() {
        return Err(format!("Encrypted file '{}' does not exist", encrypted_file));
    }
    
    if !encrypted_path.is_file() {
        return Err(format!("'{}' is not a file", encrypted_file));
    }
    
    // 获取文件所在目录和文件名
    let parent_dir = encrypted_path.parent().unwrap_or(Path::new("."));
    
    // 创建输出目录名
    let output_name = encrypted_path
        .file_stem()
        .ok_or("Cannot get file name")?
        .to_string_lossy()
        .to_string();
    
    // 解压目标目录（在加密文件同级目录下）
    let output_dir = parent_dir.join(&output_name);
    
    if cfg!(target_os = "macos") || cfg!(target_os = "linux") {
        // 在 macOS 或 Linux 上使用 openssl 和 tar
        
        // 临时 tar 文件名
        let temp_tar = parent_dir.join(format!("{}.tar", output_name));
        let temp_tar_path = temp_tar.to_string_lossy().to_string();
        
        // 使用 openssl 解密
        let mut openssl_command = Command::new("openssl");
        openssl_command
            .arg("enc")
            .arg("-aes-256-cbc")  // 使用 CBC 模式替代 GCM
            .arg("-d")
            .arg("-salt")
            .arg("-pbkdf2")
            .arg("-pass")
            .arg("stdin")  // 从标准输入读取密码
            .arg("-in")
            .arg(encrypted_file)
            .arg("-out")
            .arg(&temp_tar_path)
            .stdin(Stdio::piped()) // 设置标准输入
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        
        // 启动命令并获取子进程
        let mut child = openssl_command
            .spawn()
            .map_err(|e| format!("Failed to start openssl command: {}", e))?;
        
        // 将密码写入标准输入
        if let Some(stdin) = child.stdin.as_mut() {
            stdin.write_all(password.as_bytes())
                .map_err(|e| format!("Failed to write password: {}", e))?;
        } else {
            return Err("Unable to get openssl standard input".to_string());
        }
        
        // 等待命令完成
        let output = child.wait_with_output()
            .map_err(|e| format!("Failed to wait for openssl command to complete: {}", e))?;
        
        if !output.status.success() {
            // 获取错误输出
            let stderr_output = String::from_utf8_lossy(&output.stderr);
            
            // 检查是否是密码错误导致的"bad decrypt"错误
            if stderr_output.contains("bad decrypt") {
                return Err("Decryption failed: Incorrect password".to_string());
            }
            
            return Err(format!("Failed to decrypt file: {}", stderr_output));
        }
        
        // 确保目标父目录存在
        let parent_dir_str = parent_dir.to_string_lossy().to_string();
        
        // 使用tar解压，直接解压到父目录
        // 不切换目录，使用-C选项指定解压目标目录
        let tar_command = Command::new("tar")
            .args(&[
                "-xf", 
                &temp_tar_path, 
                "-C", 
                &parent_dir_str
            ])
            .output()
            .map_err(|e| format!("Failed to execute tar command: {}", e))?;
        
        if !tar_command.status.success() {
            return Err(format!("Failed to extract file: {}", String::from_utf8_lossy(&tar_command.stderr)));
        }
        
        // 删除临时 tar 文件
        fs::remove_file(&temp_tar_path)
            .map_err(|e| format!("Failed to delete temporary file: {}", e))?;
        
    } else if cfg!(target_os = "windows") {
        // 在 Windows 上使用 7z
        
        let mut sevenzip_command = Command::new("7z");
        sevenzip_command.arg("x");
        
        // 添加密码参数（如果有）
        if !password.is_empty() {
            let password_arg = format!("-p{}", password);
            sevenzip_command.arg(password_arg);
        }
        
        // 指定输出目录为原始文件所在目录
        let output_arg = format!("-o{}", parent_dir.display());
        sevenzip_command.arg(encrypted_file).arg(output_arg);
        
        let output = sevenzip_command.output()
            .map_err(|e| format!("Failed to execute 7z command: {}", e))?;
        
        if !output.status.success() {
            let stderr_output = String::from_utf8_lossy(&output.stderr);
            
            // 检查是否是密码错误
            if stderr_output.contains("Wrong password") {
                return Err("Decryption failed: Incorrect password".to_string());
            }
            
            return Err(format!("Failed to decrypt folder: {}", stderr_output));
        }
    } else {
        return Err("Unsupported operating system".to_string());
    }
    
    Ok(format!("File has been decrypted to: {}", output_dir.display()))
} 