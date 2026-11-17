//! Context cursors share Keysor's green strokes and dark outline.
use super::*;
use windows_sys::Win32::Graphics::Gdi::*;

type Segment = (f32, f32, f32, f32);

fn segments(id: u32) -> Vec<Segment> {
    let mut lines = Vec::new();
    let mut line = |a, b, c, d| lines.push((a, b, c, d));
    match id {
        32513 => { // Text insertion: hotspot at the middle of the stem.
            line(16., 5., 16., 27.);
            line(13., 5., 19., 5.);
            line(13., 27., 19., 27.);
        }
        32646 | 32648 | 32649 => {
            // Original Keysor strokes, translated with its 32px native hotspot.
            line(14.,15.,17.,28.);
            line(14.,15.,28.,26.);
            line(16.,24.,23.,13.);
        }
        _ => {
            let axes: &[(f32,f32)] = match id {
                32642 => &[(0.707,0.707)],
                32643 => &[(0.707,-0.707)],
                32644 => &[(1.,0.)],
                32645 => &[(0.,1.)],
                _ => &[],
            };
            for &(dx,dy) in axes {
                line(16.-dx*11.,16.-dy*11.,16.+dx*11.,16.+dy*11.);
                for sign in [-1.,1.] {
                    let (x,y) = (16.+sign*dx*11.,16.+sign*dy*11.);
                    for side in [-1.,1.] {
                        line(x,y,x-sign*dx*3.5-dy*side*3.5,y-sign*dy*3.5+dx*side*3.5);
                    }
                }
            }
        }
    }
    lines
}

fn distance(x: f32, y: f32, &(a,b,c,d): &Segment) -> f32 {
    let (dx,dy) = (c-a,d-b);
    let t = (((x-a)*dx+(y-b)*dy)/(dx*dx+dy*dy)).clamp(0.,1.);
    ((x-a-t*dx).powi(2)+(y-b-t*dy).powi(2)).sqrt()
}

pub(super) fn pixels(id: u32, size: usize) -> Vec<u32> {
    let lines = segments(id);
    let lift = if matches!(id,32646 | 32648 | 32649) { 0.5 } else { 0.0 };
    let keysor = lift > 0.0;
    let mut pixels = vec![0;size*size];
    for y in 0..size { for x in 0..size {
        let mut sum = [0u32;4];
        for sy in 0..4 { for sx in 0..4 {
            let px = (x as f32+(sx as f32+0.5)/4.)*32./size as f32;
            let py = (y as f32+(sy as f32+0.5)/4.)*32./size as f32;
            let d = lines.iter().map(|s| distance(px,py,s)).fold(f32::INFINITY,f32::min);
            if d <= if keysor { 2.0 } else { 1.65 } {
                sum[3]+=255;
                if keysor {
                    // Apply the same 50% white mix to the black outline.
                    for channel in &mut sum[..3] { *channel += 128; }
                }
            }
            if d <= if keysor { 1.25 } else { 0.9 } {
                let rgb = if keysor {
                    // Same green gradient as the base cursor, lifted toward white.
                    let t = (((px-14.)*14.+(py-15.)*11.)/317.).clamp(0.,1.);
                    [47.*(1.-t),255.*(1.-t)+77.*t,173.*(1.-t)+32.*t]
                        .map(|c| (c+(255.-c)*lift).round() as u32)
                } else { [40,224,130] };
                let outline = if keysor { 128 } else { 0 };
                sum[0] += rgb[2] - outline;
                sum[1] += rgb[1] - outline;
                sum[2] += rgb[0] - outline;
            }
        }}
        pixels[y*size+x]=(0..4).fold(0,|p,c|p|(((sum[c]+8)/16)<<(c*8)));
    }}
    pixels
}

pub(super) fn render(id: u32, size: i32) -> isize {
    let pixels = pixels(id,size as usize);
    unsafe {
        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = size;
        bmi.bmiHeader.biHeight = -size;
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        let mut bits = std::ptr::null_mut();
        let bitmap = CreateDIBSection(0,&bmi,DIB_RGB_COLORS,&mut bits,0,0);
        if bitmap == 0 { return 0; }
        if bits.is_null() { DeleteObject(bitmap); return 0; }
        std::ptr::copy_nonoverlapping(pixels.as_ptr(),bits as *mut u32,pixels.len());
        let mask_bits = transparency_mask(&pixels,size as usize,size as usize);
        let mask = CreateBitmap(size,size,1,1,mask_bits.as_ptr() as *const _);
        let cursor = if mask != 0 {
            let (x,y) = if matches!(id,32646 | 32648 | 32649) { (14,15) } else { (16,16) };
            let info = ICONINFO { fIcon:0,xHotspot:(x*size/32) as u32,
                yHotspot:(y*size/32) as u32,hbmMask:mask,hbmColor:bitmap };
            let cursor = CreateIconIndirect(&info);
            DeleteObject(mask);
            cursor
        } else { 0 };
        DeleteObject(bitmap);
        cursor
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn native_context_handles_keep_size_and_hotspot() {
        for size in [32, 48, 64] {
            for id in [32513,32642,32643,32644,32645,32646,32648,32649] {
                let cursor = super::render(id,size);
                assert_ne!(cursor,0);
                unsafe {
                    let mut info: super::ICONINFO = std::mem::zeroed();
                    let ok = super::GetIconInfo(cursor,&mut info);
                    super::DestroyCursor(cursor);
                    assert_ne!(ok,0);
                    let expected = if matches!(id,32646 | 32648 | 32649) { (14,15) } else { (16,16) };
                    let hotspot = (info.xHotspot,info.yHotspot);
                    super::DeleteObject(info.hbmColor);
                    super::DeleteObject(info.hbmMask);
                    assert_eq!(hotspot,((expected.0*size/32) as u32,(expected.1*size/32) as u32));
                }
            }
        }
    }

    #[test]
    fn context_shapes_are_distinct_visible_and_inside_canvas() {
        let ids = [32513,32642,32643,32644,32645,32646,32648,32649];
        let images: Vec<_> = ids.iter().map(|&id| super::pixels(id,32)).collect();
        for image in &images[6..] {
            assert_eq!(image,&images[5], "The last three roles share the same shape and color");
        }
        for (i,pixels) in images.iter().enumerate() {
            assert!(pixels.iter().filter(|&&p|p>>24>128).count()>30);
            assert!(pixels.iter().any(|&p|p>>24>0 && p>>24<255));
            for j in 0..32 {
                assert_eq!((pixels[j]|pixels[31*32+j]|pixels[j*32]|pixels[j*32+31])>>24,0);
            }
            if i < 6 {
                for other in &images[..i] { assert_ne!(pixels,other); }
            }
        }
        if let Some(path) = std::env::var_os("KEYSOR_CONTEXT_PREVIEW") {
            // Portable RGB contact sheet, using the actual rasterizer.
            let mut output = b"P6\n384 48\n255\n".to_vec();
            for y in 0..48 { for x in 0..384 {
                let pixel = if (8..40).contains(&y) && (8..40).contains(&(x%48)) {
                    images[x/48][(y-8)*32+x%48-8]
                } else { 0 };
                let alpha = pixel>>24;
                for shift in [16,8,0] {
                    output.push((((pixel>>shift)&255)+238*(255-alpha)/255).min(255) as u8);
                }
            }}
            std::fs::write(path,output).unwrap();
        }
    }
}
