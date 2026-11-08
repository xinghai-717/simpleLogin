use keyring::Entry;

const SERVICE_NAME: &str = "SimpleLogin";

/// 保存密码
pub fn save_password(username: &String, password: &String) -> Result<(), String> {
    let entry = Entry::new(SERVICE_NAME, username)
        .map_err(|e| format!("创建密码条目失败: {}", e))?;
    entry.set_password(password)
        .map_err(|e| format!("保存密码失败: {}", e))?;
    Ok(())
}

/// 读取密码
pub fn load_password(username: &String) -> Result<Option<String>, String> {
    let entry = Entry::new(SERVICE_NAME, username)
        .map_err(|e| format!("创建密码条目失败: {}", e))?;

    match entry.get_password() {
        Ok(pwd) => Ok(Some(pwd)),
        Err(keyring::Error::NoEntry) => Ok(None), // 没有密码
        Err(e) => Err(format!("读取密码失败: {}", e)),
    }
}

/// 删除密码
pub fn delete_password(username: &String) -> Result<(), String> {
    let entry = Entry::new(SERVICE_NAME, username)
        .map_err(|e| format!("创建密码条目失败: {}", e))?;

    match entry.delete_credential() {
        Ok(_) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()), // 没有密码
        Err(e) => Err(format!("删除密码失败: {}", e)),
    }
}