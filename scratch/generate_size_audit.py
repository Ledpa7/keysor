"""Read-only audit fixture: extracts production drawing code, never installs cursors."""
from pathlib import Path
import hashlib
import os, winreg

root = Path(__file__).resolve().parents[1]
out = root / 'target' / 'size-audit'
(out / 'src').mkdir(parents=True, exist_ok=True)

def function(text, name):
    start = text.index('fn ' + name + '(')
    brace = text.index('{', start)
    depth = 1
    end = brace + 1
    while depth:
        depth += (text[end] == '{') - (text[end] == '}')
        end += 1
    return text[start:end]

gdi = (root / 'src/ui/win_gdi.rs').read_text(encoding='utf-8')
scheme = (root / 'src/cursor_scheme.rs').read_text(encoding='utf-8')
policy = function(gdi, 'get_indicator_dpi_scale').replace('let sys_dpi = get_dpi_scale_at_cursor();', '')
policy = policy.replace('fn get_indicator_dpi_scale() -> f64', 'fn policy(sys_dpi: f64) -> f64')
mask = function(scheme, 'transparency_mask')
draw = function(gdi, 'draw_cursor_artwork')
remote = function(gdi, 'render_remote_cursor')
(out / 'Cargo.toml').write_text('''[package]
name="keysor-size-audit"
default-run="keysor-size-audit"
version="0.0.0"
edition="2021"
[dependencies]
serde_json="1.0"
windows-sys={version="0.52.0",features=["Win32_Foundation","Win32_UI_WindowsAndMessaging","Win32_Graphics_Gdi","Win32_Graphics_GdiPlus"]}
''', encoding='utf-8')

template = r'''
#![allow(dead_code,unused_imports)]
use std::sync::{Mutex,OnceLock};
use std::result::Result::Ok;
use windows_sys::Win32::Graphics::{Gdi::*,GdiPlus::*};
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use serde_json::json;
#[derive(PartialEq,Clone,Copy)] enum ClickType{None,Left,Right,Scroll}
static CLICK_TYPE:OnceLock<Mutex<ClickType>>=OnceLock::new();
static CLICK_SCALE:OnceLock<Mutex<f32>>=OnceLock::new();
fn is_currently_snapped()->bool{false}
mod hook {
 use super::*;
 pub struct State {pub is_dragging:bool,pub active_scroll_keys:Vec<u32>}
 pub static APP_STATE:OnceLock<Mutex<State>>=OnceLock::new();
}
mod cursor_scheme { use super::*; __MASK__ }
use cursor_scheme::transparency_mask;
#[path="__CONTEXT__"] mod context;
__POLICY__
__DRAW__
__REMOTE__
#[link(name="user32")]
extern "system" {
 fn GetSystemMetricsForDpi(index:i32,dpi:u32)->i32;
 fn SetThreadDpiAwarenessContext(context:isize)->isize;
 fn SetProcessDpiAwarenessContext(context:isize)->i32;
}
#[link(name="Shcore")]
extern "system" {fn GetDpiForMonitor(monitor:isize,kind:i32,x:*mut u32,y:*mut u32)->i32;}
unsafe extern "system" fn collect_monitor(monitor:isize,_:isize,_:*mut windows_sys::Win32::Foundation::RECT,data:isize)->i32 {
 let mut info:MONITORINFOEXW=std::mem::zeroed();info.monitorInfo.cbSize=std::mem::size_of::<MONITORINFOEXW>() as u32;
 if GetMonitorInfoW(monitor,&mut info as *mut _ as *mut MONITORINFO)!=0 {
  let mut x=0;let mut y=0;let dpi_result=GetDpiForMonitor(monitor,0,&mut x,&mut y);
  let name=String::from_utf16_lossy(&info.szDevice[..info.szDevice.iter().position(|&v|v==0).unwrap_or(32)]);
  let rect=info.monitorInfo.rcMonitor;
  (*(data as *mut Vec<serde_json::Value>)).push(json!({"device":name,"width":rect.right-rect.left,"height":rect.bottom-rect.top,
    "dpi_result":dpi_result,"dpi_x":x,"dpi_y":y,"primary":info.monitorInfo.dwFlags&1!=0}));
 } 1
}
fn save(name:&str,n:usize,pixels:&[u32])->serde_json::Value {
 let bytes:Vec<u8>=pixels.iter().flat_map(|p|p.to_le_bytes()).collect();
 std::fs::write(format!("{name}.bgra"),bytes).unwrap();
 let mut xs=Vec::new();let mut ys=Vec::new();let mut opaque=0;let mut soft=0;
 for (i,p) in pixels.iter().enumerate(){
  if p>>24>0{xs.push(i%n);ys.push(i/n);}
  if p>>24>128{opaque+=1;}
  if p>>24>0 && p>>24<255{soft+=1;}
 }
 assert!(opaque>10);
 let bounds=vec![*xs.iter().min().unwrap(),*ys.iter().min().unwrap(),*xs.iter().max().unwrap()+1,*ys.iter().max().unwrap()+1];
 json!({"name":name,"canvas":n,"bounds":bounds,"opaque_pixels":opaque,"soft_pixels":soft})
}
unsafe fn local(scale:f64,click:f32)->Vec<u32>{
 let size=(32.0*scale) as i32;
 let dc=CreateCompatibleDC(0);
 let mut bmi:BITMAPINFO=std::mem::zeroed();
 bmi.bmiHeader.biSize=std::mem::size_of::<BITMAPINFOHEADER>() as u32;
 bmi.bmiHeader.biWidth=size;bmi.bmiHeader.biHeight=-size;
 bmi.bmiHeader.biPlanes=1;bmi.bmiHeader.biBitCount=32;
 let mut bits=std::ptr::null_mut();
 let bitmap=CreateDIBSection(dc,&bmi,DIB_RGB_COLORS,&mut bits,0,0);
 assert!(bitmap!=0 && !bits.is_null());
 let old=SelectObject(dc,bitmap);
 std::ptr::write_bytes(bits,0,(size*size*4) as usize);
 draw_cursor_artwork(dc,scale,click,(0.,0.));GdiFlush();
 let pixels=std::slice::from_raw_parts(bits as *const u32,(size*size) as usize).to_vec();
 SelectObject(dc,old);DeleteObject(bitmap);DeleteDC(dc);pixels
}
unsafe fn remote_pixels()->Vec<u32>{
 let cursor=render_remote_cursor(32);assert_ne!(cursor,0);
 let mut info:ICONINFO=std::mem::zeroed();assert_ne!(GetIconInfo(cursor,&mut info),0);
 DestroyCursor(cursor);
 let mut bmi:BITMAPINFO=std::mem::zeroed();bmi.bmiHeader.biSize=std::mem::size_of::<BITMAPINFOHEADER>() as u32;
 bmi.bmiHeader.biWidth=32;bmi.bmiHeader.biHeight=-32;bmi.bmiHeader.biPlanes=1;bmi.bmiHeader.biBitCount=32;
 let mut pixels=vec![0u32;1024];let dc=GetDC(0);
 assert_eq!(GetDIBits(dc,info.hbmColor,0,32,pixels.as_mut_ptr() as *mut _,&mut bmi,DIB_RGB_COLORS),32);
 ReleaseDC(0,dc);DeleteObject(info.hbmColor);DeleteObject(info.hbmMask);pixels
}
fn main(){std::env::set_current_dir(env!("CARGO_MANIFEST_DIR")).unwrap();unsafe{
 let process_pm=SetProcessDpiAwarenessContext(-4)!=0;
 let previous=SetThreadDpiAwarenessContext(-4);
 let mut monitors:Vec<serde_json::Value>=Vec::new();
 assert_ne!(EnumDisplayMonitors(0,std::ptr::null(),Some(collect_monitor),&mut monitors as *mut _ as isize),0);
 let input=GdiplusStartupInput{GdiplusVersion:1,DebugEventCallback:0,SuppressBackgroundThread:0,SuppressExternalCodecs:0};
 let mut token=0;assert_eq!(GdiplusStartup(&mut token,&input,std::ptr::null_mut()),0);
 let mut renders=Vec::new();
 for size in [32,40,48,64]{for (label,click) in [("normal",1.),("held",0.7)]{
  renders.push(save(&format!("local-{size}-{label}"),size,&local(size as f64/32.,click)));
 }}
 renders.push(save("remote-32-normal",32,&remote_pixels()));
 for size in [32,40,48,56,64,72,80,96,112,128]{for id in [32513,32642,32643,32644,32645,32646,32648,32649]{
  let pixels=context::pixels(id,size);renders.push(save(&format!("context-{id}-{size}"),size,&pixels));
 }}
 let legacy_metric=GetSystemMetrics(SM_CXCURSOR);
 let matrix:Vec<_>=[100,110,115,120,125,140,150,175,179,180,200,225,250,300,350,400].into_iter().map(|percent|{
  let dpi=(96.*percent as f64/100.).round() as u32;
  let canvas=(32.*policy(percent as f64/100.)) as i32;
  json!({"percent":percent,"dpi":dpi,"local_canvas":canvas,"local_logical_dip":canvas as f64/(percent as f64/100.),
    "context_legacy_metric":legacy_metric,"context_metric_for_dpi":GetSystemMetricsForDpi(SM_CXCURSOR,dpi),
    "remote_native_canvas":32,"remote_overlay_canvas":canvas})
 }).collect();
 let value=json!({"method":"Production drawing functions extracted verbatim; normal/held stub state; off-screen DIB only; no cursor installation, hooks, keyboard input or settings changes",
  "process_pm_v2_set":process_pm,"thread_pm_v2_set":previous!=0,"monitors":monitors,"legacy_metric":legacy_metric,"matrix":matrix,"renders":renders});
 std::fs::write("native-results.json",serde_json::to_string_pretty(&value).unwrap()).unwrap();
 GdiplusShutdown(token);if previous!=0{SetThreadDpiAwarenessContext(previous);}
}}
'''
template = (template.replace('__MASK__', 'pub(crate) '+mask)
            .replace('__CONTEXT__', str(root / 'src/cursor_scheme/context.rs').replace('\\','/'))
            .replace('__POLICY__', policy).replace('__DRAW__', draw).replace('__REMOTE__', remote))
(out / 'src/main.rs').write_text(template, encoding='utf-8')
import json
(out / 'source-inputs.json').write_text(json.dumps({str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for p in [root/'src/ui/win_gdi.rs',root/'src/cursor_scheme.rs',root/'src/cursor_scheme/context.rs']},indent=2),encoding='utf-8')
# Separate follow-up binary; leaves the prior size audit output intact.
arrow_source=template[:template.index('fn main(){')]
with winreg.OpenKey(winreg.HKEY_CURRENT_USER,r'Control Panel\Cursors') as key:
    configured=os.path.expandvars(winreg.QueryValueEx(key,'Arrow')[0])
paths=[('windows-aero',str(Path(os.environ.get('SystemRoot','C:/Windows'))/'Cursors/aero_arrow.cur')),
       ('configured-arrow',configured)]
arrow_source+=r'''
unsafe fn file_cursor(path:&str,size:i32)->Vec<u32>{
 let path:Vec<u16>=path.encode_utf16().chain(Some(0)).collect();
 let cursor=LoadImageW(0,path.as_ptr(),IMAGE_CURSOR,size,size,LR_LOADFROMFILE);
 assert_ne!(cursor,0);
 let mut info:ICONINFO=std::mem::zeroed();assert_ne!(GetIconInfo(cursor,&mut info),0);DestroyCursor(cursor);
 assert_ne!(info.hbmColor,0);
 let mut bmi:BITMAPINFO=std::mem::zeroed();bmi.bmiHeader.biSize=std::mem::size_of::<BITMAPINFOHEADER>() as u32;
 bmi.bmiHeader.biWidth=size;bmi.bmiHeader.biHeight=-size;bmi.bmiHeader.biPlanes=1;bmi.bmiHeader.biBitCount=32;
 let mut pixels=vec![0u32;(size*size) as usize];let dc=GetDC(0);
 assert_eq!(GetDIBits(dc,info.hbmColor,0,size as u32,pixels.as_mut_ptr() as *mut _,&mut bmi,DIB_RGB_COLORS),size);
 ReleaseDC(0,dc);DeleteObject(info.hbmColor);DeleteObject(info.hbmMask);pixels
}
fn main(){std::env::set_current_dir(env!("CARGO_MANIFEST_DIR")).unwrap();unsafe{
 let _=SetProcessDpiAwarenessContext(-4);
 let mut values=Vec::new();
 for (label,path) in __PATHS__ {for size in [32,40,48,64]{
  values.push(save(&format!("{label}-{size}"),size as usize,&file_cursor(path,size)));
 }}
 std::fs::write("arrow-comparison.json",serde_json::to_string_pretty(&values).unwrap()).unwrap();
}}
'''.replace('__PATHS__','['+','.join('('+json.dumps(a,ensure_ascii=False)+','+json.dumps(b,ensure_ascii=False)+')' for a,b in paths)+']')
(out/'src/bin').mkdir(exist_ok=True)
(out/'src/bin/arrow_comparison.rs').write_text(arrow_source,encoding='utf-8')
print(out)
