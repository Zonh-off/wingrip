use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SZ, REG_VALUE_TYPE, RegCloseKey,
    RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
};
use windows::core::w;

/// Dynamic helper that resolves current executable path inside quotes for Windows Run Registry key.
pub fn get_current_exe_path() -> Option<String> {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(path_str) = exe_path.to_str() {
            return Some(format!("\"{}\"", path_str));
        }
    }
    None
}

/// Checks HKEY_CURRENT_USER Run registry key to see if Wingrip is registered for Windows startup.
pub fn is_startup_enabled() -> bool {
    unsafe {
        let mut hkey = HKEY::default();
        let sub_key = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
        let name = w!("Wingrip");

        if RegOpenKeyExW(HKEY_CURRENT_USER, sub_key, 0, KEY_QUERY_VALUE, &mut hkey).is_ok() {
            let mut value_type = REG_VALUE_TYPE::default();
            let mut size = 0u32;

            let res = RegQueryValueExW(
                hkey,
                name,
                None,
                Some(&mut value_type),
                None,
                Some(&mut size),
            );

            let _ = RegCloseKey(hkey);
            res.is_ok()
        } else {
            false
        }
    }
}

/// Registers or unregisters Wingrip in the Windows registry to run automatically at startup.
pub fn set_startup_enabled(enabled: bool) -> Result<(), String> {
    unsafe {
        let mut hkey = HKEY::default();
        let sub_key = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
        let name = w!("Wingrip");

        if enabled {
            let exe_path =
                get_current_exe_path().ok_or("Could not resolve current executable path")?;
            let path_u16: Vec<u16> = exe_path.encode_utf16().chain(std::iter::once(0)).collect();

            let open_res = RegOpenKeyExW(HKEY_CURRENT_USER, sub_key, 0, KEY_SET_VALUE, &mut hkey);

            if open_res.is_ok() {
                let set_res = RegSetValueExW(
                    hkey,
                    name,
                    0,
                    REG_SZ,
                    Some(std::slice::from_raw_parts(
                        path_u16.as_ptr() as *const u8,
                        path_u16.len() * 2,
                    )),
                );
                let _ = RegCloseKey(hkey);
                if set_res.is_ok() {
                    Ok(())
                } else {
                    Err(format!("RegSetValueExW failed: {:?}", set_res))
                }
            } else {
                Err(format!("RegOpenKeyExW failed: {:?}", open_res))
            }
        } else {
            let open_res = RegOpenKeyExW(HKEY_CURRENT_USER, sub_key, 0, KEY_SET_VALUE, &mut hkey);

            if open_res.is_ok() {
                let del_res = RegDeleteValueW(hkey, name);
                let _ = RegCloseKey(hkey);
                if del_res.is_ok() || del_res.0 == 2 {
                    // 2 = ERROR_FILE_NOT_FOUND
                    Ok(())
                } else {
                    Err(format!("RegDeleteValueW failed: {:?}", del_res))
                }
            } else {
                Err(format!("RegOpenKeyExW failed: {:?}", open_res))
            }
        }
    }
}
