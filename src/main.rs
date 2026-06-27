#![cfg_attr(windows, windows_subsystem = "windows")]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc};
use std::thread;

use anyhow::{Context, Result};
use eframe::egui;
use egui::{Color32, RichText, ScrollArea};
use image::imageops::FilterType;
use image::{GenericImageView, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};

// ==================== 数据结构定义 ====================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnchorPoint {
    TopLeft,
    TopCenter,
    TopRight,
    MiddleLeft,
    Center,
    MiddleRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

impl AnchorPoint {
    pub const ALL: [AnchorPoint; 9] = [
        AnchorPoint::TopLeft,
        AnchorPoint::TopCenter,
        AnchorPoint::TopRight,
        AnchorPoint::MiddleLeft,
        AnchorPoint::Center,
        AnchorPoint::MiddleRight,
        AnchorPoint::BottomLeft,
        AnchorPoint::BottomCenter,
        AnchorPoint::BottomRight,
    ];

    pub fn label(self) -> &'static str {
        match self {
            AnchorPoint::TopLeft => "↖ 左上",
            AnchorPoint::TopCenter => "↑ 上中",
            AnchorPoint::TopRight => "↗ 右上",
            AnchorPoint::MiddleLeft => "← 左中",
            AnchorPoint::Center => "⊙ 中心",
            AnchorPoint::MiddleRight => "→ 右中",
            AnchorPoint::BottomLeft => "↙ 左下",
            AnchorPoint::BottomCenter => "↓ 下中",
            AnchorPoint::BottomRight => "↘ 右下",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayoutMode {
    Single,
    Tiled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TilePattern {
    Grid,
    Brick,
    Diagonal,
}

impl TilePattern {
    pub const ALL: [TilePattern; 3] = [TilePattern::Grid, TilePattern::Brick, TilePattern::Diagonal];

    pub fn label(self) -> &'static str {
        match self {
            TilePattern::Grid => "标准网格",
            TilePattern::Brick => "砖墙交错",
            TilePattern::Diagonal => "对角线无缝",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NameConflictStrategy {
    Rename,
    Overwrite,
    Skip,
}

impl NameConflictStrategy {
    pub const ALL: [NameConflictStrategy; 3] = [
        NameConflictStrategy::Rename,
        NameConflictStrategy::Overwrite,
        NameConflictStrategy::Skip,
    ];

    pub fn label(self) -> &'static str {
        match self {
            NameConflictStrategy::Rename => "重命名（加 _wm 后缀）",
            NameConflictStrategy::Overwrite => "覆盖同名文件",
            NameConflictStrategy::Skip => "跳过同名文件",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    SameAsSource,
    Jpeg,
    Png,
}

impl ExportFormat {
    pub const ALL: [ExportFormat; 3] = [
        ExportFormat::SameAsSource,
        ExportFormat::Jpeg,
        ExportFormat::Png,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ExportFormat::SameAsSource => "与源图一致",
            ExportFormat::Jpeg => "JPEG",
            ExportFormat::Png => "PNG",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatermarkParams {
    pub layout_mode: LayoutMode,
    pub anchor: AnchorPoint,
    pub margin_px: u32,
    pub offset_x: i32,
    pub offset_y: i32,
    pub rotation_deg: f32,
    pub tile_pattern: TilePattern,
    pub tile_spacing_x_pct: f32,
    pub tile_spacing_y_pct: f32,
    pub opacity: f32,
    pub scale_percent: f32,
    pub absolute_width_px: Option<u32>,
}

impl Default for WatermarkParams {
    fn default() -> Self {
        Self {
            layout_mode: LayoutMode::Single,
            anchor: AnchorPoint::BottomRight,
            margin_px: 20,
            offset_x: 0,
            offset_y: 0,
            rotation_deg: 0.0,
            tile_pattern: TilePattern::Grid,
            tile_spacing_x_pct: 50.0,
            tile_spacing_y_pct: 50.0,
            opacity: 1.0,
            scale_percent: 50.0,
            absolute_width_px: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SourceImageInfo {
    pub path: PathBuf,
    pub dimensions: Option<(u32, u32)>,
    pub file_size_bytes: Option<u64>,
    pub is_valid: bool,
    pub error_msg: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExportProgress {
    pub current: usize,
    pub total: usize,
    pub current_file: String,
    pub success: usize,
    pub skipped: usize,
    pub failed: usize,
    pub failed_items: Vec<(String, String)>,
    pub is_cancelled: bool,
    pub is_finished: bool,
}

impl Default for ExportProgress {
    fn default() -> Self {
        Self {
            current: 0,
            total: 0,
            current_file: String::new(),
            success: 0,
            skipped: 0,
            failed: 0,
            failed_items: Vec::new(),
            is_cancelled: false,
            is_finished: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedConfig {
    pub params: WatermarkParams,
    pub last_output_dir: Option<PathBuf>,
    pub export_format: ExportFormat,
    pub jpeg_quality: u8,
    pub name_strategy: NameConflictStrategy,
}

impl Default for PersistedConfig {
    fn default() -> Self {
        Self {
            params: WatermarkParams::default(),
            last_output_dir: None,
            export_format: ExportFormat::SameAsSource,
            jpeg_quality: 85,
            name_strategy: NameConflictStrategy::Rename,
        }
    }
}

// ==================== 水印合成核心函数 ====================

pub fn apply_opacity(img: &RgbaImage, opacity: f32) -> RgbaImage {
    let opacity = opacity.clamp(0.0, 1.0);
    let mut result = img.clone();
    for pixel in result.pixels_mut() {
        let alpha = pixel.0[3] as f32 * opacity;
        pixel.0[3] = alpha.clamp(0.0, 255.0) as u8;
    }
    result
}

pub fn resize_watermark(
    watermark: &RgbaImage,
    source_width: u32,
    source_height: u32,
    scale_percent: f32,
    absolute_width_px: Option<u32>,
) -> RgbaImage {
    let (wm_w, wm_h) = watermark.dimensions();
    if wm_w == 0 || wm_h == 0 {
        return watermark.clone();
    }

    let target_w = if let Some(abs_w) = absolute_width_px {
        abs_w.max(1)
    } else {
        let short_edge = source_width.min(source_height) as f32;
        let scale = (scale_percent / 100.0).clamp(0.1, 2.0);
        (short_edge * scale).max(1.0) as u32
    };

    let ratio = target_w as f32 / wm_w as f32;
    let target_h = ((wm_h as f32 * ratio) as u32).max(1);

    let max_w = (source_width as f32 * 1.5) as u32;
    let max_h = (source_height as f32 * 1.5) as u32;
    let (final_w, final_h) = if target_w > max_w || target_h > max_h {
        let r = ((max_w as f32 / target_w as f32).min(max_h as f32 / target_h as f32)).min(1.0);
        ((target_w as f32 * r).max(1.0) as u32, (target_h as f32 * r).max(1.0) as u32)
    } else {
        (target_w, target_h)
    };

    image::imageops::resize(watermark, final_w, final_h, FilterType::Lanczos3)
}

pub fn compute_single_position(
    source_w: u32,
    source_h: u32,
    wm_w: u32,
    wm_h: u32,
    anchor: AnchorPoint,
    margin: u32,
    offset_x: i32,
    offset_y: i32,
) -> (i32, i32) {
    let margin = margin as i32;
    let sw = source_w as i32;
    let sh = source_h as i32;
    let ww = wm_w as i32;
    let wh = wm_h as i32;

    let (base_x, base_y) = match anchor {
        AnchorPoint::TopLeft => (margin, margin),
        AnchorPoint::TopCenter => ((sw - ww) / 2, margin),
        AnchorPoint::TopRight => (sw - ww - margin, margin),
        AnchorPoint::MiddleLeft => (margin, (sh - wh) / 2),
        AnchorPoint::Center => ((sw - ww) / 2, (sh - wh) / 2),
        AnchorPoint::MiddleRight => (sw - ww - margin, (sh - wh) / 2),
        AnchorPoint::BottomLeft => (margin, sh - wh - margin),
        AnchorPoint::BottomCenter => ((sw - ww) / 2, sh - wh - margin),
        AnchorPoint::BottomRight => (sw - ww - margin, sh - wh - margin),
    };

    let x = base_x + offset_x;
    let y = base_y + offset_y;

    let x = x.max(-ww + 1).min(sw - 1);
    let y = y.max(-wh + 1).min(sh - 1);

    (x, y)
}

pub fn rotate_image(img: &RgbaImage, angle_deg: f32) -> RgbaImage {
    let angle = angle_deg.to_radians();
    let (w, h) = img.dimensions();
    let cos = angle.cos().abs();
    let sin = angle.sin().abs();
    let new_w = ((w as f32 * cos) + (h as f32 * sin)).ceil() as u32;
    let new_h = ((w as f32 * sin) + (h as f32 * cos)).ceil() as u32;

    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    let ncx = new_w as f32 / 2.0;
    let ncy = new_h as f32 / 2.0;

    let mut out = RgbaImage::new(new_w.max(1), new_h.max(1));

    for (x, y, pixel) in out.enumerate_pixels_mut() {
        let dx = x as f32 - ncx;
        let dy = y as f32 - ncy;
        let src_x = dx * angle.cos() + dy * angle.sin() + cx;
        let src_y = -dx * angle.sin() + dy * angle.cos() + cy;

        if src_x >= 0.0 && src_x < w as f32 && src_y >= 0.0 && src_y < h as f32 {
            let x0 = src_x.floor() as u32;
            let y0 = src_y.floor() as u32;
            let x1 = (x0 + 1).min(w - 1);
            let y1 = (y0 + 1).min(h - 1);
            let fx = src_x - x0 as f32;
            let fy = src_y - y0 as f32;

            let p00 = img.get_pixel(x0, y0).0;
            let p10 = img.get_pixel(x1, y0).0;
            let p01 = img.get_pixel(x0, y1).0;
            let p11 = img.get_pixel(x1, y1).0;

            let mut result = [0u8; 4];
            for c in 0..4 {
                let top = p00[c] as f32 * (1.0 - fx) + p10[c] as f32 * fx;
                let bot = p01[c] as f32 * (1.0 - fx) + p11[c] as f32 * fx;
                result[c] = (top * (1.0 - fy) + bot * fy).clamp(0.0, 255.0) as u8;
            }
            *pixel = Rgba(result);
        } else {
            *pixel = Rgba([0, 0, 0, 0]);
        }
    }
    out
}

pub fn alpha_composite(base: &mut RgbaImage, overlay: &RgbaImage, pos_x: i32, pos_y: i32) {
    let (bw, bh) = base.dimensions();
    let (ow, oh) = overlay.dimensions();

    let start_x = pos_x.max(0) as u32;
    let start_y = pos_y.max(0) as u32;
    let end_x = (pos_x + ow as i32).min(bw as i32).max(0) as u32;
    let end_y = (pos_y + oh as i32).min(bh as i32).max(0) as u32;

    if start_x >= end_x || start_y >= end_y {
        return;
    }

    for y in start_y..end_y {
        for x in start_x..end_x {
            let ox = (x as i32 - pos_x) as u32;
            let oy = (y as i32 - pos_y) as u32;
            let op = overlay.get_pixel(ox, oy).0;
            let bp = base.get_pixel(x, y).0;

            if op[3] == 0 {
                continue;
            }

            let src_a = op[3] as f32 / 255.0;
            let dst_a = bp[3] as f32 / 255.0;
            let out_a = src_a + dst_a * (1.0 - src_a);

            let mut result = [0u8; 4];
            if out_a > 0.0 {
                for c in 0..3 {
                    result[c] = ((op[c] as f32 * src_a + bp[c] as f32 * dst_a * (1.0 - src_a)) / out_a)
                        .clamp(0.0, 255.0) as u8;
                }
            }
            result[3] = (out_a * 255.0).clamp(0.0, 255.0) as u8;

            base.put_pixel(x, y, Rgba(result));
        }
    }
}

pub fn compose_watermark(
    source: &RgbaImage,
    watermark: &RgbaImage,
    params: &WatermarkParams,
) -> RgbaImage {
    let (sw, sh) = source.dimensions();
    let mut result = source.clone();

    let scaled_wm = resize_watermark(
        watermark,
        sw,
        sh,
        params.scale_percent,
        params.absolute_width_px,
    );
    let transparent_wm = apply_opacity(&scaled_wm, params.opacity);

    match params.layout_mode {
        LayoutMode::Single => {
            let rotated_wm = if params.rotation_deg.abs() > 0.01 {
                rotate_image(&transparent_wm, params.rotation_deg)
            } else {
                transparent_wm
            };
            let (wm_w, wm_h) = rotated_wm.dimensions();
            let (x, y) = compute_single_position(
                sw,
                sh,
                wm_w,
                wm_h,
                params.anchor,
                params.margin_px,
                params.offset_x,
                params.offset_y,
            );
            alpha_composite(&mut result, &rotated_wm, x, y);
        }
        LayoutMode::Tiled => {
            let (wm_w, wm_h) = transparent_wm.dimensions();
            let gap_x = (wm_w as f32 * params.tile_spacing_x_pct / 100.0) as i32;
            let gap_y = (wm_h as f32 * params.tile_spacing_y_pct / 100.0) as i32;
            let step_x = wm_w as i32 + gap_x;
            let step_y = wm_h as i32 + gap_y;

            let start_x = -((wm_w as i32).max(step_x));
            let start_y = -((wm_h as i32).max(step_y));
            let end_x = sw as i32 + step_x;
            let end_y = sh as i32 + step_y;

            let mut y = start_y;
            let mut row = 0;
            while y < end_y {
                let row_offset = match params.tile_pattern {
                    TilePattern::Grid => 0,
                    TilePattern::Brick => {
                        if row % 2 == 1 {
                            step_x / 2
                        } else {
                            0
                        }
                    }
                    TilePattern::Diagonal => {
                        ((row as f32) * (step_x as f32 / 2.0)) as i32
                    }
                };
                let mut x = start_x + row_offset;
                while x < end_x {
                    alpha_composite(&mut result, &transparent_wm, x, y);
                    x += step_x;
                }
                y += step_y;
                row += 1;
            }
        }
    }

    result
}

// ==================== 导出相关辅助函数 ====================

pub fn determine_output_format(
    source_path: &Path,
    export_format: ExportFormat,
) -> (image::ImageFormat, &'static str) {
    match export_format {
        ExportFormat::SameAsSource => {
            let ext = source_path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase())
                .unwrap_or_default();
            match ext.as_str() {
                "jpg" | "jpeg" => (image::ImageFormat::Jpeg, "jpg"),
                "png" => (image::ImageFormat::Png, "png"),
                "bmp" => (image::ImageFormat::Bmp, "bmp"),
                "webp" => (image::ImageFormat::WebP, "webp"),
                _ => (image::ImageFormat::Png, "png"),
            }
        }
        ExportFormat::Jpeg => (image::ImageFormat::Jpeg, "jpg"),
        ExportFormat::Png => (image::ImageFormat::Png, "png"),
    }
}

pub fn resolve_conflict_path(
    dir: &Path,
    base_name: &str,
    ext: &str,
    strategy: NameConflictStrategy,
) -> (Option<PathBuf>, bool) {
    let candidate = dir.join(format!("{}_wm.{}", base_name, ext));

    match strategy {
        NameConflictStrategy::Overwrite => (Some(candidate), false),
        NameConflictStrategy::Skip => {
            if candidate.exists() {
                (None, true)
            } else {
                (Some(candidate), false)
            }
        }
        NameConflictStrategy::Rename => {
            if !candidate.exists() {
                (Some(candidate), false)
            } else {
                let mut n = 1;
                loop {
                    let p = dir.join(format!("{}_wm_{}.{}", base_name, ext, n));
                    if !p.exists() {
                        return (Some(p), false);
                    }
                    n += 1;
                    if n > 9999 {
                        break;
                    }
                }
                (None, true)
            }
        }
    }
}

pub fn process_single_image(
    src_path: &Path,
    watermark_rgba: &RgbaImage,
    params: &WatermarkParams,
    output_dir: &Path,
    strategy: NameConflictStrategy,
    export_format: ExportFormat,
    jpeg_quality: u8,
) -> Result<(String, String)> {
    let source = image::open(src_path)
        .with_context(|| format!("无法打开源图: {}", src_path.display()))?;
    let source_rgba = source.to_rgba8();

    let result_rgba = compose_watermark(&source_rgba, watermark_rgba, params);

    let (img_fmt, ext) = determine_output_format(src_path, export_format);
    let base_name = src_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("image");

    let (out_path_opt, skipped) = resolve_conflict_path(output_dir, base_name, ext, strategy);
    if skipped {
        return Ok((
            "skipped".to_string(),
            format!("文件已存在: {}", base_name),
        ));
    }
    let out_path = out_path_opt.context("无法生成输出文件名")?;

    if img_fmt == image::ImageFormat::Jpeg {
        let (width, height) = result_rgba.dimensions();
        let mut rgb_raw = Vec::with_capacity((width * height * 3) as usize);
        for p in result_rgba.pixels() {
            let a = p.0[3] as f32 / 255.0;
            rgb_raw.push((p.0[0] as f32 * a + 255.0 * (1.0 - a)) as u8);
            rgb_raw.push((p.0[1] as f32 * a + 255.0 * (1.0 - a)) as u8);
            rgb_raw.push((p.0[2] as f32 * a + 255.0 * (1.0 - a)) as u8);
        }
        let jpeg_encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
            std::io::BufWriter::new(std::fs::File::create(&out_path)?),
            jpeg_quality,
        );
        image::ImageEncoder::write_image(
            jpeg_encoder,
            &rgb_raw,
            width,
            height,
            image::ExtendedColorType::Rgb8,
        )?;
    } else {
        result_rgba.save_with_format(&out_path, img_fmt)?;
    }

    Ok(("success".to_string(), out_path.display().to_string()))
}

// ==================== UI 应用 ====================

struct WatermarkApp {
    source_images: Vec<SourceImageInfo>,
    current_preview_idx: usize,
    watermark_image: Option<Arc<RgbaImage>>,
    watermark_path: Option<PathBuf>,
    output_dir: Option<PathBuf>,

    params: WatermarkParams,
    auto_preview: bool,
    preview_dirty: bool,
    preview_texture: Option<egui::TextureHandle>,

    export_format: ExportFormat,
    jpeg_quality: u8,
    name_strategy: NameConflictStrategy,

    export_in_progress: bool,
    cancel_flag: Arc<AtomicBool>,
    progress_rx: Option<Receiver<ExportProgress>>,
    last_progress: Option<ExportProgress>,
    show_report_dialog: bool,

    status_message: String,
    toasts: Vec<Toast>,
}

struct Toast {
    message: String,
    color: Color32,
    remaining: f32,
}

impl Default for WatermarkApp {
    fn default() -> Self {
        let mut slf = Self {
            source_images: Vec::new(),
            current_preview_idx: 0,
            watermark_image: None,
            watermark_path: None,
            output_dir: None,
            params: WatermarkParams::default(),
            auto_preview: true,
            preview_dirty: true,
            preview_texture: None,
            export_format: ExportFormat::SameAsSource,
            jpeg_quality: 85,
            name_strategy: NameConflictStrategy::Rename,
            export_in_progress: false,
            cancel_flag: Arc::new(AtomicBool::new(false)),
            progress_rx: None,
            last_progress: None,
            show_report_dialog: false,
            status_message: "就绪".to_string(),
            toasts: Vec::new(),
        };
        slf.load_config();
        slf
    }
}

impl WatermarkApp {
    fn config_path() -> Option<PathBuf> {
        if let Some(dirs) = directories::ProjectDirs::from("com", "watermark", "tool") {
            let dir = dirs.config_dir().to_path_buf();
            let _ = std::fs::create_dir_all(&dir);
            Some(dir.join("config.json"))
        } else {
            None
        }
    }

    fn load_config(&mut self) {
        if let Some(path) = Self::config_path() {
            if let Ok(data) = std::fs::read_to_string(&path) {
                if let Ok(cfg) = serde_json::from_str::<PersistedConfig>(&data) {
                    self.params = cfg.params;
                    self.output_dir = cfg.last_output_dir;
                    self.export_format = cfg.export_format;
                    self.jpeg_quality = cfg.jpeg_quality;
                    self.name_strategy = cfg.name_strategy;
                }
            }
        }
    }

    fn save_config(&self) {
        let cfg = PersistedConfig {
            params: self.params.clone(),
            last_output_dir: self.output_dir.clone(),
            export_format: self.export_format,
            jpeg_quality: self.jpeg_quality,
            name_strategy: self.name_strategy,
        };
        if let Some(path) = Self::config_path() {
            if let Ok(json) = serde_json::to_string_pretty(&cfg) {
                let _ = std::fs::write(&path, json);
            }
        }
    }

    fn mark_preview_dirty(&mut self) {
        self.preview_dirty = true;
        self.preview_texture = None;
    }

    fn add_toast(&mut self, msg: impl Into<String>, color: Color32) {
        self.toasts.push(Toast {
            message: msg.into(),
            color,
            remaining: 3.5,
        });
    }

    fn add_source_images(&mut self, paths: Vec<PathBuf>) {
        for p in paths {
            let (dims, size, valid, err) = match image::open(&p) {
                Ok(img) => {
                    let dims = Some(img.dimensions());
                    drop(img);
                    let sz = std::fs::metadata(&p).ok().map(|m| m.len());
                    (dims, sz, true, None)
                }
                Err(e) => (None, None, false, Some(e.to_string())),
            };
            self.source_images.push(SourceImageInfo {
                path: p,
                dimensions: dims,
                file_size_bytes: size,
                is_valid: valid,
                error_msg: err,
            });
        }
        if self.current_preview_idx >= self.source_images.len() {
            self.current_preview_idx = 0;
        }
        self.mark_preview_dirty();
    }

    fn render_preview(&mut self, ui: &mut egui::Ui) {
        if self.source_images.is_empty() {
            return;
        }

        let idx = self.current_preview_idx;
        let info = &self.source_images[idx];
        if !info.is_valid {
            ui.colored_label(Color32::RED, format!("❌ 当前图片损坏: {}", info.error_msg.as_deref().unwrap_or("未知错误")));
            return;
        }

        let src_path = info.path.clone();
        let render_needed = self.preview_dirty || self.preview_texture.is_none();

        if render_needed {
            match image::open(&src_path) {
                Ok(src_img) => {
                    let max_w = ui.available_width().min(900.0) as u32;
                    let (w, h) = src_img.dimensions();
                    let ratio = (max_w as f32 / w as f32).min(1.0);
                    let dw = ((w as f32 * ratio) as u32).max(1);
                    let dh = ((h as f32 * ratio) as u32).max(1);

                    let disp_w = dw as usize;
                    let disp_h = dh as usize;

                    let src_thumb = image::imageops::resize(
                        &src_img.to_rgba8(),
                        dw,
                        dh,
                        FilterType::Lanczos3,
                    );

                    let final_img = if let Some(wm) = &self.watermark_image {
                        compose_watermark(&src_thumb, wm, &self.params)
                    } else {
                        src_thumb
                    };

                    let color_img = egui::ColorImage::from_rgba_unmultiplied(
                        [disp_w, disp_h],
                        &final_img,
                    );
                    let tex = ui.ctx().load_texture(
                        format!("preview_{}_{}", idx, std::time::SystemTime::now().elapsed().map(|d| d.as_millis()).unwrap_or(0)),
                        color_img,
                        egui::TextureOptions::LINEAR,
                    );
                    self.preview_texture = Some(tex);
                    self.preview_dirty = false;
                }
                Err(e) => {
                    ui.colored_label(Color32::RED, format!("❌ 预览加载失败: {}", e));
                    return;
                }
            }
        }

        if let Some(tex) = &self.preview_texture {
            ui.image(tex);
        }
    }

    fn start_export(&mut self) {
        if self.export_in_progress {
            return;
        }
        if self.source_images.is_empty() {
            self.add_toast("请先选择源图片", Color32::RED);
            return;
        }
        if self.watermark_image.is_none() {
            self.add_toast("请先选择水印图片", Color32::RED);
            return;
        }

        let valid_images: Vec<SourceImageInfo> = self
            .source_images
            .iter()
            .filter(|i| i.is_valid)
            .cloned()
            .collect();
        if valid_images.is_empty() {
            self.add_toast("没有有效的源图片", Color32::RED);
            return;
        }

        let output_dir = match self.output_dir.clone() {
            Some(d) => d,
            None => {
                if let Some(first) = valid_images.first() {
                    if let Some(parent) = first.path.parent() {
                        let d = parent.join("watermarked_output");
                        let _ = std::fs::create_dir_all(&d);
                        d
                    } else {
                        PathBuf::from("./watermarked_output")
                    }
                } else {
                    PathBuf::from("./watermarked_output")
                }
            }
        };

        if let Err(e) = std::fs::create_dir_all(&output_dir) {
            self.add_toast(format!("无法创建输出目录: {}", e), Color32::RED);
            return;
        }

        if std::fs::metadata(&output_dir).is_err() {
            self.add_toast("输出目录不存在或无写入权限", Color32::RED);
            return;
        }

        self.save_config();

        let cancel_flag = Arc::new(AtomicBool::new(false));
        self.cancel_flag = cancel_flag.clone();

        let wm = self.watermark_image.clone().unwrap();
        let params = self.params.clone();
        let strategy = self.name_strategy;
        let fmt = self.export_format;
        let jpeg_q = self.jpeg_quality;
        let total = valid_images.len();

        let (tx, rx): (Sender<ExportProgress>, Receiver<ExportProgress>) = mpsc::channel();
        self.progress_rx = Some(rx);
        self.last_progress = None;
        self.export_in_progress = true;
        self.show_report_dialog = false;

        thread::spawn(move || {
            let mut progress = ExportProgress {
                current: 0,
                total,
                current_file: String::new(),
                success: 0,
                skipped: 0,
                failed: 0,
                failed_items: Vec::new(),
                is_cancelled: false,
                is_finished: false,
            };

            for (i, info) in valid_images.iter().enumerate() {
                if cancel_flag.load(Ordering::SeqCst) {
                    progress.is_cancelled = true;
                    break;
                }
                progress.current = i + 1;
                progress.current_file = info
                    .path
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_default();
                let _ = tx.send(progress.clone());

                match process_single_image(
                    &info.path,
                    &wm,
                    &params,
                    &output_dir,
                    strategy,
                    fmt,
                    jpeg_q,
                ) {
                    Ok((status, _)) => match status.as_str() {
                        "success" => progress.success += 1,
                        "skipped" => progress.skipped += 1,
                        _ => progress.success += 1,
                    },
                    Err(e) => {
                        progress.failed += 1;
                        progress.failed_items.push((
                            info.path
                                .file_name()
                                .map(|f| f.to_string_lossy().to_string())
                                .unwrap_or_else(|| "?".into()),
                            e.to_string(),
                        ));
                    }
                }
            }

            progress.is_finished = true;
            let _ = tx.send(progress);
        });
    }

    fn poll_export_progress(&mut self) {
        let mut finished = false;
        if let Some(rx) = &self.progress_rx {
            while let Ok(p) = rx.try_recv() {
                self.last_progress = Some(p.clone());
                if p.is_finished {
                    finished = true;
                }
            }
        }
        if finished {
            self.export_in_progress = false;
            self.show_report_dialog = true;
            self.progress_rx = None;
        }
    }

    fn reset_params(&mut self) {
        self.params = WatermarkParams::default();
        self.mark_preview_dirty();
        self.add_toast("已重置所有参数", Color32::from_rgb(0x4C, 0xAF, 0x50));
    }

    fn current_watermark_size_display(&self) -> String {
        if let Some(wm) = &self.watermark_image {
            if let Some(src) = self.source_images.get(self.current_preview_idx) {
                if let Some((sw, sh)) = src.dimensions {
                    let scaled = resize_watermark(wm, sw, sh, self.params.scale_percent, self.params.absolute_width_px);
                    let (w, h) = scaled.dimensions();
                    return format!("当前尺寸：{} × {} px", w, h);
                }
            }
            let (w, h) = wm.dimensions();
            format!("原始尺寸：{} × {} px", w, h)
        } else {
            "未选择水印".to_string()
        }
    }
}

impl eframe::App for WatermarkApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_export_progress();

        let dt = ctx.input(|i| i.stable_dt);
        for t in self.toasts.iter_mut() {
            t.remaining -= dt;
        }
        self.toasts.retain(|t| t.remaining > 0.0);

        if ctx.input(|i| i.viewport().close_requested()) {
            self.save_config();
        }

        self.render_ui(ctx);
        self.render_report_dialog(ctx);
        self.render_toasts(ctx);
    }
}

impl WatermarkApp {
    fn render_ui(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("control_panel")
            .resizable(true)
            .default_width(340.0)
            .width_range(280.0..=500.0)
            .show(ctx, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("🎛 控制面板");
                    ui.separator();
                    self.render_source_files_ui(ui);
                    ui.separator();
                    self.render_watermark_ui(ui);
                    ui.separator();
                    self.render_output_dir_ui(ui);
                    ui.separator();
                    self.render_layout_ui(ui);
                    ui.separator();
                    self.render_style_ui(ui);
                    ui.separator();
                    self.render_preview_ctrls_ui(ui);
                    ui.separator();
                    self.render_export_ui(ui);
                    ui.add_space(16.0);
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_preview_panel_ui(ui);
        });
    }

    fn render_source_files_ui(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("🖼 源图片", |ui| {
            ui.horizontal(|ui| {
                if ui.button("📁 选择图片 (多选)").clicked() {
                    if let Some(paths) = rfd::FileDialog::new()
                        .add_filter("图片文件", &["jpg", "jpeg", "png", "bmp", "webp"])
                        .pick_files()
                    {
                        self.source_images.clear();
                        self.add_source_images(paths);
                    }
                }
                if ui.button("➕ 追加").clicked() {
                    if let Some(paths) = rfd::FileDialog::new()
                        .add_filter("图片文件", &["jpg", "jpeg", "png", "bmp", "webp"])
                        .pick_files()
                    {
                        self.add_source_images(paths);
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label(format!("已选：{} 张（有效 {}，损坏 {}）",
                    self.source_images.len(),
                    self.source_images.iter().filter(|i| i.is_valid).count(),
                    self.source_images.iter().filter(|i| !i.is_valid).count(),
                ));
                if ui.button("🗑 清空").clicked() {
                    self.source_images.clear();
                    self.current_preview_idx = 0;
                    self.mark_preview_dirty();
                }
            });
            ui.add_space(4.0);
            let mut items: Vec<(usize, String, bool, bool)> = Vec::new();
            for (i, info) in self.source_images.iter().enumerate() {
                let name = info.path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "?".into());
                let dims_str = info.dimensions.map(|(w, h)| format!("{}×{}", w, h)).unwrap_or_else(|| "?".into());
                let size_str = info.file_size_bytes.map(|s| {
                    if s < 1024 { format!("{}B", s) }
                    else if s < 1024*1024 { format!("{}KB", s/1024) }
                    else { format!("{}MB", s/(1024*1024)) }
                }).unwrap_or_else(|| "?".into());
                let label = if info.is_valid {
                    format!("[{}] {}  {}  {}", i + 1, name, dims_str, size_str)
                } else {
                    format!("× [{}] {}  (损坏)", i + 1, name)
                };
                items.push((i, label, i == self.current_preview_idx, info.is_valid));
            }
            let mut sel_idx: Option<usize> = None;
            let mut remove_idx: Option<usize> = None;
            let cur_idx = self.current_preview_idx;

            egui::ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
                for (i, label, is_sel, is_valid) in items.iter() {
                    let resp = ui.selectable_label(*is_sel, label);
                    if resp.clicked() && *is_valid && *i != cur_idx {
                        sel_idx = Some(*i);
                    }
                    let rect = resp.rect;
                    let remove_rect = egui::Rect::from_min_size(
                        egui::pos2(rect.right() - 18.0, rect.top() + 2.0),
                        egui::vec2(16.0, 16.0),
                    );
                    if ui.put(remove_rect, egui::Button::new("✖").frame(false).small()).clicked() {
                        remove_idx = Some(*i);
                    }
                }
            });

            if let Some(idx) = sel_idx {
                self.current_preview_idx = idx;
                self.mark_preview_dirty();
            }
            if let Some(idx) = remove_idx {
                self.source_images.remove(idx);
                if self.current_preview_idx >= self.source_images.len() {
                    self.current_preview_idx = self.source_images.len().saturating_sub(1);
                }
                self.mark_preview_dirty();
            }
        });
    }

    fn render_watermark_ui(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("💧 水印图片", |ui| {
            ui.horizontal(|ui| {
                if ui.button("选择 PNG 水印").clicked() {
                    if let Some(p) = rfd::FileDialog::new()
                        .add_filter("PNG (推荐透明背景)", &["png"])
                        .pick_file()
                    {
                        if let Ok(img) = image::open(&p) {
                            let rgba = img.to_rgba8();
                            let has_alpha = rgba.pixels().any(|p| p.0[3] < 255);
                            if !has_alpha {
                                self.add_toast("提示: 建议使用带透明背景的 PNG 以获得最佳效果", Color32::YELLOW);
                            }
                            self.watermark_image = Some(Arc::new(rgba));
                            self.watermark_path = Some(p);
                            self.mark_preview_dirty();
                        } else {
                            self.add_toast("无法读取水印图片", Color32::RED);
                        }
                    }
                }
                if self.watermark_path.is_some() {
                    if ui.button("取消").clicked() {
                        self.watermark_image = None;
                        self.watermark_path = None;
                        self.mark_preview_dirty();
                    }
                    if ui.button("重选").clicked() {
                        if let Some(p) = rfd::FileDialog::new()
                            .add_filter("PNG (推荐透明背景)", &["png"])
                            .pick_file()
                        {
                            if let Ok(img) = image::open(&p) {
                                self.watermark_image = Some(Arc::new(img.to_rgba8()));
                                self.watermark_path = Some(p);
                                self.mark_preview_dirty();
                            }
                        }
                    }
                }
            });
            if let Some(p) = &self.watermark_path {
                ui.label(format!("当前：{}", p.file_name().unwrap().to_string_lossy()));
                if let Some(wm) = &self.watermark_image {
                    let (w, h) = wm.dimensions();
                    ui.label(format!("尺寸：{} × {} px", w, h));
                }
            } else {
                ui.label(RichText::new("未选择水印图片").weak().small());
            }
        });
    }

    fn render_output_dir_ui(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("📤 导出目录", |ui| {
            ui.horizontal(|ui| {
                if ui.button("选择目录").clicked() {
                    if let Some(d) = rfd::FileDialog::new().pick_folder() {
                        self.output_dir = Some(d);
                    }
                }
                if self.output_dir.is_some() {
                    if ui.button("取消").clicked() {
                        self.output_dir = None;
                    }
                    if ui.button("重选").clicked() {
                        if let Some(d) = rfd::FileDialog::new().pick_folder() {
                            self.output_dir = Some(d);
                        }
                    }
                }
            });
            let path_display = self
                .output_dir
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "默认：源目录/watermarked_output".into());
            ui.label(RichText::new(path_display).small());
        });
    }

    fn render_layout_ui(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("📍 布局模式", |ui| {
            ui.horizontal(|ui| {
                let mut dirty = false;
                let resp = ui.selectable_label(self.params.layout_mode == LayoutMode::Single, "① 单个水印");
                if resp.clicked() && self.params.layout_mode != LayoutMode::Single {
                    self.params.layout_mode = LayoutMode::Single;
                    dirty = true;
                }
                let resp = ui.selectable_label(self.params.layout_mode == LayoutMode::Tiled, "② 平铺水印");
                if resp.clicked() && self.params.layout_mode != LayoutMode::Tiled {
                    self.params.layout_mode = LayoutMode::Tiled;
                    dirty = true;
                }
                if dirty && self.auto_preview { self.mark_preview_dirty(); }
            });

            ui.add_space(6.0);

            match self.params.layout_mode {
                LayoutMode::Single => self.render_single_layout_ui(ui),
                LayoutMode::Tiled => self.render_tiled_layout_ui(ui),
            }
        });
    }

    fn render_single_layout_ui(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("9 宫格锚点").strong());
        let mut anchor_changed = false;
        egui::Grid::new("anchor_grid").num_columns(3).spacing([4.0, 4.0]).show(ui, |ui| {
            for row in 0..3 {
                for col in 0..3 {
                    let idx = row * 3 + col;
                    let ap = AnchorPoint::ALL[idx];
                    let is_sel = self.params.anchor == ap;
                    let btn = egui::Button::new(ap.label()).min_size(egui::vec2(86.0, 28.0));
                    let resp = if is_sel {
                        ui.add_sized([86.0, 28.0], btn.fill(Color32::from_rgb(0x1E, 0x88, 0xE5)))
                    } else {
                        ui.add_sized([86.0, 28.0], btn)
                    };
                    if resp.clicked() && !is_sel {
                        self.params.anchor = ap;
                        anchor_changed = true;
                    }
                }
                ui.end_row();
            }
        });
        ui.add_space(6.0);

        let mut dirty = false;
        let resp = ui.add(
            egui::Slider::new(&mut self.params.margin_px, 0..=500).text("边缘边距 (px)")
        );
        if resp.changed() { dirty = true; }
        ui.horizontal(|ui| {
            ui.label("X 偏移: ");
            let r = ui.add(egui::DragValue::new(&mut self.params.offset_x).speed(1).range(-10000..=10000));
            if r.changed() { dirty = true; }
            ui.label(" Y 偏移: ");
            let r = ui.add(egui::DragValue::new(&mut self.params.offset_y).speed(1).range(-10000..=10000));
            if r.changed() { dirty = true; }
        });
        ui.add_space(4.0);
        let resp = ui.add(
            egui::Slider::new(&mut self.params.rotation_deg, -180.0..=180.0).text("旋转角度 (°)").suffix("°")
        );
        if resp.changed() { dirty = true; }
        ui.horizontal(|ui| {
            if ui.button("↺ -90°").clicked() {
                self.params.rotation_deg = (self.params.rotation_deg - 90.0).rem_euclid(360.0);
                if self.params.rotation_deg > 180.0 { self.params.rotation_deg -= 360.0; }
                dirty = true;
            }
            if ui.button("↻ +90°").clicked() {
                self.params.rotation_deg = (self.params.rotation_deg + 90.0).rem_euclid(360.0);
                if self.params.rotation_deg > 180.0 { self.params.rotation_deg -= 360.0; }
                dirty = true;
            }
            if ui.button("⇅ 180°").clicked() {
                self.params.rotation_deg = (self.params.rotation_deg + 180.0).rem_euclid(360.0);
                if self.params.rotation_deg > 180.0 { self.params.rotation_deg -= 360.0; }
                dirty = true;
            }
        });

        if (anchor_changed || dirty) && self.auto_preview {
            self.mark_preview_dirty();
        }
    }

    fn render_tiled_layout_ui(&mut self, ui: &mut egui::Ui) {
        let mut dirty = false;
        ui.label(RichText::new("平铺子模式").strong());
        egui::ComboBox::from_id_source("tile_pattern")
            .selected_text(self.params.tile_pattern.label())
            .show_ui(ui, |ui| {
                for mode in TilePattern::ALL {
                    let resp = ui.selectable_label(self.params.tile_pattern == mode, mode.label());
                    if resp.clicked() && self.params.tile_pattern != mode {
                        self.params.tile_pattern = mode;
                        dirty = true;
                    }
                }
            });
        ui.add_space(6.0);
        let resp = ui.add(
            egui::Slider::new(&mut self.params.tile_spacing_x_pct, 0.0..=200.0)
                .text("水平间距 (%)")
                .suffix("%")
        );
        if resp.changed() { dirty = true; }
        let resp = ui.add(
            egui::Slider::new(&mut self.params.tile_spacing_y_pct, 0.0..=200.0)
                .text("垂直间距 (%)")
                .suffix("%")
        );
        if resp.changed() { dirty = true; }
        ui.label(RichText::new("0% = 无缝紧贴，100% = 间距 = 水印尺寸").weak().small());

        if dirty && self.auto_preview { self.mark_preview_dirty(); }
    }

    fn render_style_ui(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("🎨 样式参数（透明度 / 缩放）", |ui| {
            let mut dirty = false;
            let resp = ui.add(
                egui::Slider::new(&mut self.params.opacity, 0.01..=1.0).text("透明度")
            );
            if resp.changed() { dirty = true; }

            ui.separator();
            ui.label(RichText::new("缩放控制").strong());
            let resp = ui.add(
                egui::Slider::new(&mut self.params.scale_percent, 10.0..=200.0)
                    .text("相对缩放 (源短边%)")
                    .suffix("%")
            );
            if resp.changed() {
                self.params.absolute_width_px = None;
                dirty = true;
            }

            ui.horizontal(|ui| {
                ui.label("绝对宽度 (px):");
                let mut abs_w = self.params.absolute_width_px.unwrap_or(0);
                let r = ui.add(
                    egui::DragValue::new(&mut abs_w).speed(1).range(0..=10000)
                );
                if r.changed() {
                    self.params.absolute_width_px = if abs_w > 0 { Some(abs_w) } else { None };
                    dirty = true;
                }
                if ui.button("清除").clicked() {
                    self.params.absolute_width_px = None;
                    dirty = true;
                }
            });

            ui.label(RichText::new(self.current_watermark_size_display()).small().color(Color32::from_rgb(0x40, 0x9E, 0xFF)));

            ui.add_space(6.0);
            if ui
                .add_sized([ui.available_width(), 28.0], egui::Button::new("⟲ 重置所有参数"))
                .clicked()
            {
                self.reset_params();
            }

            if dirty && self.auto_preview {
                self.mark_preview_dirty();
            }
        });
    }

    fn render_preview_ctrls_ui(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("👁 预览控制", |ui| {
            ui.horizontal(|ui| {
                if ui.button("🔄 刷新预览").clicked() {
                    self.mark_preview_dirty();
                }
                let r = ui.checkbox(&mut self.auto_preview, "自动实时预览");
                if r.changed() {
                    self.status_message = if self.auto_preview { "自动预览：开".into() } else { "自动预览：关（需手动刷新）".into() };
                }
            });
        });
    }

    fn render_export_ui(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("🚀 导出设置", |ui| {
            ui.label(RichText::new("命名冲突策略").strong());
            for strategy in NameConflictStrategy::ALL {
                let r = ui.radio(self.name_strategy == strategy, strategy.label());
                if r.clicked() { self.name_strategy = strategy; }
            }

            ui.add_space(6.0);
            ui.label(RichText::new("导出格式").strong());
            egui::ComboBox::from_id_source("export_fmt")
                .selected_text(self.export_format.label())
                .show_ui(ui, |ui| {
                    for f in ExportFormat::ALL {
                        let r = ui.selectable_label(self.export_format == f, f.label());
                        if r.clicked() { self.export_format = f; }
                    }
                });

            if self.export_format == ExportFormat::Jpeg
                || matches!(self.export_format, ExportFormat::SameAsSource)
            {
                let resp = ui.add(
                    egui::Slider::new(&mut self.jpeg_quality, 1..=100).text("JPEG 质量").suffix("%")
                );
                if resp.changed() { /* no preview needed */ }
            }

            ui.add_space(12.0);
            let can_click = !self.export_in_progress;
            let btn = egui::Button::new(if self.export_in_progress { "⏳ 导出中..." } else { "🚀 开始批量导出" });
            let resp = if can_click {
                ui.add_sized([ui.available_width(), 44.0], btn.fill(Color32::from_rgb(0x2E, 0x7D, 0x32)))
            } else {
                ui.add_sized([ui.available_width(), 44.0], btn)
            };
            if resp.clicked() && can_click {
                self.start_export();
            }

            if self.export_in_progress {
                ui.add_space(6.0);
                if let Some(p) = &self.last_progress {
                    let pct = if p.total > 0 { p.current as f32 / p.total as f32 } else { 0.0 };
                    ui.add(egui::ProgressBar::new(pct).text(format!("正在处理 {}/{}  {}", p.current, p.total, p.current_file)));
                    ui.label(format!("成功: {}  跳过: {}  失败: {}", p.success, p.skipped, p.failed));
                }
                if ui.button("⚠ 取消导出").clicked() {
                    self.cancel_flag.store(true, Ordering::SeqCst);
                }
            }
        });
    }

    fn render_preview_panel_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("👁 预览区域");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let total = self.source_images.len();
                let mut nav = false;
                if ui.button("▶ 下一张").clicked() && total > 0 {
                    self.current_preview_idx = (self.current_preview_idx + 1) % total;
                    nav = true;
                }
                if ui.button("◀ 上一张").clicked() && total > 0 {
                    self.current_preview_idx = if self.current_preview_idx == 0 { total - 1 } else { self.current_preview_idx - 1 };
                    nav = true;
                }
                if ui.button("🔄 刷新预览").clicked() {
                    self.mark_preview_dirty();
                }
                let r = ui.checkbox(&mut self.auto_preview, "自动预览");
                if r.changed() {}
                if nav { self.mark_preview_dirty(); }
            });
        });
        ui.separator();

        if self.source_images.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(120.0);
                ui.label(RichText::new("请先从左侧选择「源图片」开始预览").heading().weak());
                ui.add_space(12.0);
                ui.label(RichText::new("提示: 支持多选 JPG / PNG / BMP / WEBP 格式").small().weak());
            });
            return;
        }

        let idx = self.current_preview_idx;
        let info = &self.source_images[idx];
        let name = info.path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "?".into());
        let total = self.source_images.len();
        ui.label(format!(
            "正在预览：{}/{} — {}  {}",
            idx + 1,
            total,
            name,
            if total > 1 { "（当前图预览，导出处理全部）" } else { "" }
        ));
        if let Some((w, h)) = info.dimensions {
            ui.label(RichText::new(format!("原图尺寸：{} × {} px", w, h)).small().weak());
        }
        ui.add_space(6.0);

        ScrollArea::both().show(ui, |ui| {
            self.render_preview(ui);
        });

        ui.separator();
        ui.label(RichText::new(&self.status_message).small());
    }

    fn render_report_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_report_dialog {
            return;
        }
        let mut should_close = false;
        let show = self.show_report_dialog;
        let mut open = show;
        egui::Window::new("📋 导出结果报告")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .default_size([480.0, 360.0])
            .show(ctx, |ui| {
                if let Some(p) = &self.last_progress {
                    ui.heading(if p.is_cancelled { "⚠ 导出已取消" } else { "✅ 导出完成" });
                    ui.label(format!("总处理：{} 张", p.total));
                    ui.label(format!(
                        "成功：{} 张  |  跳过：{} 张  |  失败：{} 张",
                        p.success, p.skipped, p.failed
                    ));
                    if p.is_cancelled {
                        ui.colored_label(Color32::from_rgb(0xFF, 0x98, 0x00), format!("已完成 {}/{} 张，用户中途取消", p.current, p.total));
                    }
                    if !p.failed_items.is_empty() {
                        ui.separator();
                        ui.label(RichText::new("失败详情:").strong());
                        ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                            for (name, err) in &p.failed_items {
                                ui.label(format!("× {}: {}", name, err));
                            }
                        });
                    }
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("📂 打开导出文件夹").clicked() {
                            let dir = self.output_dir.clone().unwrap_or_else(|| {
                                self.source_images
                                    .first()
                                    .and_then(|i| i.path.parent().map(|p| p.join("watermarked_output")))
                                    .unwrap_or_else(|| PathBuf::from("./watermarked_output"))
                            });
                            let _ = open_path(&dir);
                        }
                        if ui.button("关闭").clicked() {
                            should_close = true;
                        }
                    });
                }
            });
        if !open || should_close {
            self.show_report_dialog = false;
        }
    }

    fn render_toasts(&self, ctx: &egui::Context) {
        egui::Area::new(egui::Id::new("toasts"))
            .anchor(egui::Align2::RIGHT_BOTTOM, [-12.0, -12.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    for t in &self.toasts {
                        let alpha = (t.remaining / 3.5).clamp(0.2, 1.0);
                        let color = Color32::from_rgba_unmultiplied(
                            t.color.r(), t.color.g(), t.color.b(), (alpha * 220.0) as u8,
                        );
                        let frame = egui::Frame::popup(ui.style())
                            .fill(color)
                            .stroke(egui::Stroke::new(1.0, Color32::WHITE));
                        frame.show(ui, |ui| {
                            ui.label(RichText::new(&t.message).color(Color32::WHITE));
                        });
                    }
                });
            });
    }
}

fn open_path(path: &Path) -> Result<()> {
    let path = path.to_path_buf();
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer").arg(path).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(path).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
    }
    Ok(())
}

fn install_cjk_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    #[cfg(target_os = "windows")]
    let font_candidates: &[&str] = &[
        r"C:\Windows\Fonts\msyh.ttc",
        r"C:\Windows\Fonts\msyh.ttf",
        r"C:\Windows\Fonts\msyhbd.ttc",
        r"C:\Windows\Fonts\simhei.ttf",
        r"C:\Windows\Fonts\simsun.ttc",
    ];

    #[cfg(target_os = "macos")]
    let font_candidates: &[&str] = &[
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/STHeiti Light.ttc",
        "/Library/Fonts/Arial Unicode.ttf",
    ];

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    let font_candidates: &[&str] = &[
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ];

    let mut loaded: Vec<String> = Vec::new();
    for (i, path) in font_candidates.iter().enumerate() {
        if let Ok(bytes) = std::fs::read(path) {
            let name = format!("cjk_font_{}", i);
            fonts
                .font_data
                .insert(name.clone(), egui::FontData::from_owned(bytes));
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, name.clone());
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .push(name.clone());
            loaded.push(path.to_string());
            break;
        }
    }

    if loaded.is_empty() {
        eprintln!("[warn] 未找到任何 CJK 字体，中文可能显示为方框");
    }

    ctx.set_fonts(fonts);

    let mut style = (*ctx.style()).clone();
    use std::collections::BTreeMap;
    use egui::TextStyle;
    let mut text_styles: BTreeMap<TextStyle, egui::FontId> = BTreeMap::new();
    text_styles.insert(TextStyle::Heading, egui::FontId::proportional(20.0));
    text_styles.insert(TextStyle::Body, egui::FontId::proportional(14.0));
    text_styles.insert(TextStyle::Button, egui::FontId::proportional(14.0));
    text_styles.insert(TextStyle::Small, egui::FontId::proportional(11.5));
    text_styles.insert(TextStyle::Monospace, egui::FontId::monospace(13.0));
    style.text_styles = text_styles;
    ctx.set_style(style);
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 840.0])
            .with_min_inner_size([960.0, 620.0])
            .with_title("批量图片水印工具 (Rust + egui)"),
        ..Default::default()
    };
    eframe::run_native(
        "批量图片水印工具",
        options,
        Box::new(|cc| {
            install_cjk_fonts(&cc.egui_ctx);
            Ok(Box::<WatermarkApp>::default())
        }),
    )
}