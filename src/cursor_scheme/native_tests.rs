//! Isolated-process desktop tests driven by scripts/test-portable-cursor-safety.ps1.
use super::*;
use windows_sys::Win32::Graphics::Gdi::*;

fn snapshot() -> Vec<serde_json::Value> {
    unsafe {
        let dc = CreateCompatibleDC(0);
        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = 64;
        bmi.bmiHeader.biHeight = -64;
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        let mut bits = std::ptr::null_mut();
        let bitmap = CreateDIBSection(dc,&bmi,DIB_RGB_COLORS,&mut bits,0,0);
        assert_ne!(bitmap,0);
        assert!(!bits.is_null());
        let old = SelectObject(dc,bitmap);
        let pixels = std::slice::from_raw_parts_mut(bits as *mut u32,64*64);
        let mut output = Vec::new();
        for &id in IDS {
            pixels.fill(0xff332211);
            let cursor = LoadCursorW(0,id as usize as *const u16);
            assert_ne!(cursor,0,"cursor role {id}");
            assert_ne!(DrawIconEx(dc,0,0,cursor,32,32,0,0,DI_NORMAL),0);
            GdiFlush();
            let visible = pixels.iter().filter(|&&p| p & 0xffffff != 0x332211).count();
            let hash = pixels.iter().fold(0xcbf29ce484222325u64,|h,p|
                p.to_le_bytes().into_iter().fold(h,|h,b|(h^b as u64).wrapping_mul(0x100000001b3)));
            output.push(serde_json::json!({"id":id,"visible_pixels":visible,"hash":format!("{hash:016x}")}));
        }
        SelectObject(dc,old); DeleteObject(bitmap); DeleteDC(dc);
        output
    }
}

fn write_report(value: serde_json::Value) {
    let path=std::env::var_os("KEYSOR_SAFETY_REPORT").expect("Isolated desktop harness required");
    std::fs::write(path,value.to_string()).unwrap();
}

fn prepare_artwork() {
    use windows_sys::Win32::Graphics::GdiPlus::*;
    unsafe {
        let input=GdiplusStartupInput{GdiplusVersion:1,DebugEventCallback:0,SuppressBackgroundThread:0,SuppressExternalCodecs:0};
        let mut token=0;
        assert_eq!(GdiplusStartup(&mut token,&input,std::ptr::null_mut()),0);
        let artwork=crate::ui::win_gdi::render_native_cursor_32();
        assert_ne!(artwork,0);
        let mut state=SCHEME.lock().unwrap();
        state.artwork=CopyIcon(artwork);
        state.remote_artwork=artwork;
    }
    crate::platform::windows::set_chrome_remote_mode(true);
}

#[test]
#[ignore = "Checks absent-Guardian fail-safe on the live desktop"]
fn native_missing_guardian() {
    assert!(!std::env::current_exe().unwrap().with_file_name("keysor-guardian.exe").exists());
    crate::watchdog::spawn();
    assert!(!guardian_ready());
    prepare_artwork();
    set_custom(true);
    assert!(!status().1,"Must not replace cursors without independent Guardian");
    write_report(serde_json::json!({"custom_applied":false,"cursors":snapshot()}));
}

#[test]
#[ignore = "Reads the live desktop cursor roles; isolated desktop harness only"]
fn native_snapshot() {
    write_report(serde_json::json!({"cursors":snapshot()}));
}

#[test]
#[ignore = "Installs real Keysor artwork and waits to be terminated by the desktop harness"]
fn native_probe_child() {
    use std::os::windows::process::CommandExt;
    let guardian=std::env::var_os("KEYSOR_SAFETY_GUARDIAN").expect("Guardian path required");
    let report=crate::config::get_config_path().with_file_name(format!("watchdog-{}.json",std::process::id()));
    let child=std::process::Command::new(guardian).arg("--watchdog")
        .arg(std::process::id().to_string()).arg(&report).creation_flags(0x08000000).spawn().unwrap();
    for _ in 0..100 {
        if guardian_ready(){break;}
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(guardian_ready());
    if std::env::var_os("KEYSOR_SAFETY_NO_MONITOR").is_none() {
        crate::watchdog::monitor_guardian(child.id());
    }
    prepare_artwork();
    set_custom(true);
    assert!(status().1);
    let cursors=snapshot();
    assert!(cursors.iter().all(|v|v["visible_pixels"].as_u64().unwrap()>10));
    write_report(serde_json::json!({"pid":std::process::id(),"guardian_pid":child.id(),
        "watchdog_report":report,"applied":true,"cursors":cursors}));
    for _ in 0..1200 {
        if !status().1 {
            write_report(serde_json::json!({"pid":std::process::id(),"guardian_pid":child.id(),
                "guardian_lost":true,"cursors":snapshot()}));
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    std::thread::sleep(std::time::Duration::from_secs(60));
}

#[test]
#[ignore = "Intentionally creates an invisible system arrow; harness must recover in finally"]
fn native_blank_fixture() {
    // An intentionally bad pre-existing cursor, never used in product code.
    unsafe {
        let mask_bits=[255u8;128];
        let mask=CreateBitmap(32,32,1,1,mask_bits.as_ptr() as *const _);
        let color_bits=[0u32;1024];
        let color=CreateBitmap(32,32,1,32,color_bits.as_ptr() as *const _);
        let info=ICONINFO{fIcon:0,xHotspot:0,yHotspot:0,hbmMask:mask,hbmColor:color};
        let cursor=CreateIconIndirect(&info);
        DeleteObject(mask);DeleteObject(color);
        assert_ne!(cursor,0);
        assert_ne!(SetSystemCursor(cursor,32512),0);
    }
    write_report(serde_json::json!({"cursors":snapshot()}));
}
