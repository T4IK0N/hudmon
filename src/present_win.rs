//! windows: window with per-pixel transparency (UpdateLayeredWindow, premultiplied ARGB)
//! softbuffer 0.4 does not support the alpha channel, so we draw the overlay here with our own minimal GDI code
use std::{
    mem::{size_of, zeroed},
    ptr::null_mut,
};
use windows_sys::Win32::{
    Foundation::{GetLastError, HWND, POINT, SIZE},
    Graphics::Gdi::{
        CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject, AC_SRC_ALPHA,
        AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS, HBITMAP,
        HDC, HGDIOBJ,
    },
    UI::WindowsAndMessaging::{
        GetWindowLongW, SetWindowLongW, SetWindowPos, UpdateLayeredWindow, GWL_EXSTYLE,
        SWP_NOACTIVATE, SWP_NOZORDER, ULW_ALPHA, WS_EX_LAYERED, WS_EX_NOACTIVATE,
        WS_EX_TRANSPARENT,
    },
};

pub struct LayeredPresenter {
    hwnd: HWND,
    dc: HDC,
    bmp: HBITMAP,
    old: HGDIOBJ,
    bits: *mut u32,
    size: (usize, usize),
    geo: (i32, i32, usize, usize),
    last_err: u32,
}

impl LayeredPresenter {
    pub fn new(hwnd: isize) -> Self {
        let hwnd = hwnd as HWND;
        unsafe {
            let ex = GetWindowLongW(hwnd, GWL_EXSTYLE);
            SetWindowLongW(hwnd, GWL_EXSTYLE, ex & !(WS_EX_LAYERED as i32));
            SetWindowLongW(
                hwnd,
                GWL_EXSTYLE,
                ex | (WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE) as i32,
            );
            Self {
                hwnd,
                dc: CreateCompatibleDC(null_mut()),
                bmp: null_mut(),
                old: null_mut(),
                bits: null_mut(),
                size: (0, 0),
                geo: (i32::MIN, 0, 0, 0),
                last_err: 0,
            }
        }
    }

    fn ensure_bitmap(&mut self, w: usize, h: usize) {
        if self.size == (w, h) && !self.bmp.is_null() {
            return;
        }
        unsafe {
            if !self.bmp.is_null() {
                SelectObject(self.dc, self.old);
                DeleteObject(self.bmp as HGDIOBJ);
            }
            let mut bi: BITMAPINFO = zeroed();
            bi.bmiHeader = BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w as i32,
                biHeight: -(h as i32), // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                ..zeroed()
            };
            let mut bits: *mut core::ffi::c_void = null_mut();
            self.bmp = CreateDIBSection(self.dc, &bi, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
            self.bits = bits as *mut u32;
            self.old = SelectObject(self.dc, self.bmp as HGDIOBJ);
            self.size = (w, h);
        }
    }

    /// `pixels`: w*h 0xAARRGGBB; (x, y) – window position on screen; w, h – size of the window
    pub fn present(&mut self, pixels: &[u32], w: usize, h: usize, x: i32, y: i32) {
        if w == 0 || h == 0 || pixels.len() != w * h {
            return;
        }
        self.ensure_bitmap(w, h);
        if self.bits.is_null() {
            return;
        }
        unsafe {
            if self.geo != (x, y, w, h) {
                SetWindowPos(
                    self.hwnd,
                    null_mut(),
                    x,
                    y,
                    w as i32,
                    h as i32,
                    SWP_NOACTIVATE | SWP_NOZORDER,
                );
                self.geo = (x, y, w, h);
            }
            std::ptr::copy_nonoverlapping(pixels.as_ptr(), self.bits, w * h);
            let dst = POINT { x, y };
            let size = SIZE {
                cx: w as i32,
                cy: h as i32,
            };
            let src = POINT { x: 0, y: 0 };
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            let ok = UpdateLayeredWindow(
                self.hwnd,
                null_mut(),
                &dst,
                &size,
                self.dc,
                &src,
                0,
                &blend,
                ULW_ALPHA,
            );
            if ok == 0 {
                let err = GetLastError();
                if err != self.last_err {
                    self.last_err = err;
                    crate::metrics::fps::log(&format!(
                        "UpdateLayeredWindow nie powiodlo sie (blad {err}), rozmiar {w}x{h}"
                    ));
                }
            }
        }
    }
}

impl Drop for LayeredPresenter {
    fn drop(&mut self) {
        unsafe {
            if !self.bmp.is_null() {
                SelectObject(self.dc, self.old);
                DeleteObject(self.bmp as HGDIOBJ);
            }
            DeleteDC(self.dc);
        }
    }
}
