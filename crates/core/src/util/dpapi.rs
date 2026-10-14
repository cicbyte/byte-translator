//! Windows DPAPI 静态加密（敏感配置落盘保护）。
//!
//! 存储格式：`dpapi:<base64(CryptProtectData)>`；无前缀 = 明文（旧数据
//! 兼容读取，下次保存自动升级为密文）。密文绑定当前 Windows 用户，
//! 同机其它账户拿到文件也解不出明文。
//!
//! 非 Windows 平台退化为透传（目标平台为 Windows）。

use anyhow::{Context, Result};

const PREFIX: &str = "dpapi:";

/// 明文 → 存储格式（空串原样，表示「未设置」）
pub fn protect(plain: &str) -> Result<String> {
    if plain.is_empty() {
        return Ok(String::new());
    }
    #[cfg(target_os = "windows")]
    {
        let cipher = dpapi_crypt(plain.as_bytes(), true).context("DPAPI 加密失败")?;
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(cipher);
        Ok(format!("{PREFIX}{b64}"))
    }
    #[cfg(not(target_os = "windows"))]
    Ok(plain.to_string())
}

/// 存储格式 → 明文：dpapi: 前缀解密；否则视为存量明文原样返回。
/// 解密失败（跨用户/损坏）返回空串，视作未设置。
pub fn unprotect(stored: &str) -> String {
    if let Some(b64) = stored.strip_prefix(PREFIX) {
        #[cfg(target_os = "windows")]
        {
            use base64::Engine as _;
            match base64::engine::general_purpose::STANDARD.decode(b64) {
                Ok(cipher) => match dpapi_crypt(&cipher, false) {
                    Ok(plain) => return String::from_utf8_lossy(&plain).into_owned(),
                    Err(_) => return String::new(),
                },
                Err(_) => return String::new(),
            }
        }
        #[cfg(not(target_os = "windows"))]
        return b64.to_string();
    }
    stored.to_string()
}

#[cfg(target_os = "windows")]
fn dpapi_crypt(data: &[u8], encrypt: bool) -> Result<Vec<u8>> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::LocalFree;
    use windows::Win32::Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPT_INTEGER_BLOB,
    };

    let input = CRYPT_INTEGER_BLOB {
        cbData: data.len() as u32,
        pbData: data.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    // 无附加熵：同一 Windows 用户即可解密
    let ok = if encrypt {
        unsafe { CryptProtectData(&input, PCWSTR::null(), None, None, None, 0, &mut output) }
    } else {
        unsafe { CryptUnprotectData(&input, None, None, None, None, 0, &mut output) }
    };
    if ok.is_err() {
        anyhow::bail!("dpapi_crypt encrypt={encrypt} 调用失败");
    }
    let bytes =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
    unsafe {
        let _ = LocalFree(Some(windows::Win32::Foundation::HLOCAL(output.pbData as _)));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_plaintext_compat() {
        let stored = protect("sk-secret-123").unwrap();
        assert!(stored.starts_with("dpapi:"), "应带前缀: {stored}");
        assert_ne!(stored, "sk-secret-123");
        assert_eq!(unprotect(&stored), "sk-secret-123");
        // 存量明文兼容
        assert_eq!(unprotect("sk-legacy"), "sk-legacy");
        // 空值
        assert_eq!(protect("").unwrap(), "");
        assert_eq!(unprotect(""), "");
    }

    #[test]
    fn corrupted_cipher_yields_empty() {
        #[cfg(target_os = "windows")]
        {
            use base64::Engine as _;
            let bad = format!(
                "dpapi:{}",
                base64::engine::general_purpose::STANDARD.encode([1u8, 2, 3])
            );
            assert_eq!(unprotect(&bad), "");
        }
    }
}
