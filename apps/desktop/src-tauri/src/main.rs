// 桌面端入口：禁用 Windows 控制台窗口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    byte_translator_desktop_lib::run()
}
