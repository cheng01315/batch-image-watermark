//! 界面国际化（简体中文 / English）+ 系统语言探测。
//!
//! 用法：
//! - 取当前语言的字符串表：`let t = self.lang.t();` 然后 `t.control_panel`
//! - 带占位符的文案用 `tf!` 宏填充：`tf!(t.selected_count, "total" => 3, "valid" => 2)`

use serde::{Deserialize, Serialize};

/// 界面语言
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lang {
    Zh,
    En,
}

impl Default for Lang {
    fn default() -> Self {
        Lang::Zh
    }
}

impl Lang {
    /// 切换器里显示的短名
    pub fn short_label(self) -> &'static str {
        match self {
            Lang::Zh => "中文",
            Lang::En => "English",
        }
    }

    /// 当前语言对应的字符串表
    pub fn t(self) -> &'static Tr {
        match self {
            Lang::Zh => &ZH,
            Lang::En => &EN,
        }
    }
}

/// 环境变量强制指定界面语言（可选）：`WATERMARK_TOOL_LANG=zh` / `=en`
///
/// 优先级：环境变量 > 配置文件里存的选择 > 系统语言自动判定
pub fn lang_from_env() -> Option<Lang> {
    let v = std::env::var("WATERMARK_TOOL_LANG").ok()?.trim().to_lowercase();
    if v.starts_with("zh") || v == "cn" {
        Some(Lang::Zh)
    } else if v.starts_with("en") {
        Some(Lang::En)
    } else {
        None
    }
}

/// 探测系统语言：中文系统 → 中文界面；其它语言 → 英文界面。
pub fn detect_system_lang() -> Lang {
    if let Some(lang) = lang_from_env() {
        return lang;
    }

    #[cfg(windows)]
    {
        // GetUserDefaultUILanguage 返回 Windows 用户界面语言的 LANGID，
        // 低 10 位是主语言 ID，0x04 即中文（含简繁、中国香港/中国台湾等变体）。
        #[link(name = "kernel32")]
        extern "system" {
            fn GetUserDefaultUILanguage() -> u16;
        }
        let langid = unsafe { GetUserDefaultUILanguage() };
        if (langid & 0x03FF) == 0x04 {
            Lang::Zh
        } else {
            Lang::En
        }
    }

    #[cfg(not(windows))]
    {
        for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(v) = std::env::var(key) {
                let v = v.to_lowercase();
                if v.starts_with("zh") {
                    return Lang::Zh;
                }
            }
        }
        Lang::En
    }
}

/// 模板填充宏：把 `{key}` 占位符替换为对应值。
///
/// `tf!(t.selected_count, "total" => 3, "valid" => 2)`
macro_rules! tf {
    ($tpl:expr) => {{
        $tpl.to_string()
    }};
    ($tpl:expr, $( $k:literal => $v:expr ),* $(,)?) => {{
        let mut s: String = $tpl.to_string();
        $(
            s = s.replace(concat!("{", $k, "}"), &$v.to_string());
        )*
        s
    }};
}
pub(crate) use tf;

/// 界面文案表。ZH / EN 两份字段必须一一对应。
pub struct Tr {
    // ---------- 窗口 / 通用 ----------
    pub app_title: &'static str,
    pub control_panel: &'static str,
    pub lang_tooltip: &'static str,
    pub ready: &'static str,

    // ---------- 源图片 ----------
    pub sec_source: &'static str,
    pub btn_pick_images: &'static str,
    pub btn_append: &'static str,
    pub filter_images: &'static str,
    pub selected_count: &'static str,
    pub btn_clear: &'static str,
    pub corrupt_mark: &'static str,
    pub file_item: &'static str,
    pub file_item_bad: &'static str,

    // ---------- 水印图片 ----------
    pub sec_watermark: &'static str,
    pub btn_pick_watermark: &'static str,
    pub filter_png: &'static str,
    pub btn_remove: &'static str,
    pub btn_reselect: &'static str,
    pub wm_current: &'static str,
    pub wm_size: &'static str,
    pub wm_none: &'static str,
    pub wm_alpha_tip: &'static str,
    pub wm_load_fail: &'static str,

    // ---------- 导出目录 ----------
    pub sec_output: &'static str,
    pub btn_pick_folder: &'static str,
    pub output_default: &'static str,

    // ---------- 布局 ----------
    pub sec_layout: &'static str,
    pub mode_single: &'static str,
    pub mode_tiled: &'static str,
    pub anchor_title: &'static str,
    pub anchor_tl: &'static str,
    pub anchor_tc: &'static str,
    pub anchor_tr: &'static str,
    pub anchor_ml: &'static str,
    pub anchor_c: &'static str,
    pub anchor_mr: &'static str,
    pub anchor_bl: &'static str,
    pub anchor_bc: &'static str,
    pub anchor_br: &'static str,
    pub margin: &'static str,
    pub offset_x: &'static str,
    pub offset_y: &'static str,
    pub rotation: &'static str,
    pub tile_submode: &'static str,
    pub tile_grid: &'static str,
    pub tile_brick: &'static str,
    pub tile_diagonal: &'static str,
    pub tile_spacing_x: &'static str,
    pub tile_spacing_y: &'static str,
    pub tile_spacing_hint: &'static str,

    // ---------- 样式 ----------
    pub sec_style: &'static str,
    pub opacity: &'static str,
    pub scaling_title: &'static str,
    pub scale_relative: &'static str,
    pub abs_width: &'static str,
    pub wm_size_current: &'static str,
    pub wm_size_original: &'static str,
    pub wm_not_selected: &'static str,
    pub btn_reset: &'static str,
    pub reset_done: &'static str,

    // ---------- 预览 ----------
    pub sec_preview_ctrl: &'static str,
    pub btn_refresh: &'static str,
    pub auto_preview: &'static str,
    pub status_auto_on: &'static str,
    pub status_auto_off: &'static str,
    pub sec_preview: &'static str,
    pub btn_next: &'static str,
    pub btn_prev: &'static str,
    pub empty_hint: &'static str,
    pub empty_tip: &'static str,
    pub previewing: &'static str,
    pub preview_all_note: &'static str,
    pub source_dims: &'static str,
    pub err_corrupt: &'static str,
    pub err_unknown: &'static str,
    pub err_preview_load: &'static str,

    // ---------- 导出 ----------
    pub sec_export: &'static str,
    pub name_conflict: &'static str,
    pub name_rename: &'static str,
    pub name_overwrite: &'static str,
    pub name_skip: &'static str,
    pub export_format: &'static str,
    pub fmt_same: &'static str,
    pub jpeg_quality: &'static str,
    pub btn_export: &'static str,
    pub btn_exporting: &'static str,
    pub progress_processing: &'static str,
    pub progress_stats: &'static str,
    pub btn_cancel_export: &'static str,

    // ---------- 结果报告 ----------
    pub report_title: &'static str,
    pub report_cancelled: &'static str,
    pub report_done: &'static str,
    pub report_total: &'static str,
    pub report_stats: &'static str,
    pub report_cancel_note: &'static str,
    pub failed_details: &'static str,
    pub btn_open_folder: &'static str,
    pub btn_close: &'static str,

    // ---------- 提示 / 校验 ----------
    pub err_no_source: &'static str,
    pub err_no_watermark: &'static str,
    pub err_no_valid_source: &'static str,
    pub err_create_dir: &'static str,
    pub err_dir_unwritable: &'static str,
    pub err_open_source: &'static str,
    pub err_output_name: &'static str,
    pub err_file_exists: &'static str,
}

pub static ZH: Tr = Tr {
    app_title: "批量图片水印工具 (Rust + egui)",
    control_panel: "控制面板",
    lang_tooltip: "界面语言",
    ready: "就绪",

    sec_source: "源图片",
    btn_pick_images: "选择图片 (多选)",
    btn_append: "追加",
    filter_images: "图片文件",
    selected_count: "已选：{total} 张（有效 {valid}，损坏 {bad}）",
    btn_clear: "清空",
    corrupt_mark: "损坏",
    file_item: "[{i}] {name}  {dims}  {size}",
    file_item_bad: "× [{i}] {name}  ({mark})",

    sec_watermark: "水印图片",
    btn_pick_watermark: "选择 PNG 水印",
    filter_png: "PNG (推荐透明背景)",
    btn_remove: "取消",
    btn_reselect: "重选",
    wm_current: "当前：{name}",
    wm_size: "尺寸：{w} × {h} px",
    wm_none: "未选择水印图片",
    wm_alpha_tip: "提示: 建议使用带透明背景的 PNG 以获得最佳效果",
    wm_load_fail: "无法读取水印图片",

    sec_output: "导出目录",
    btn_pick_folder: "选择目录",
    output_default: "默认：源目录/watermarked_output",

    sec_layout: "布局模式",
    mode_single: "① 单个水印",
    mode_tiled: "② 平铺水印",
    anchor_title: "9 宫格锚点",
    anchor_tl: "↖ 左上",
    anchor_tc: "↑ 上中",
    anchor_tr: "↗ 右上",
    anchor_ml: "← 左中",
    anchor_c: "⊙ 中心",
    anchor_mr: "→ 右中",
    anchor_bl: "↙ 左下",
    anchor_bc: "↓ 下中",
    anchor_br: "↘ 右下",
    margin: "边缘边距 (px)",
    offset_x: "X 偏移: ",
    offset_y: " Y 偏移: ",
    rotation: "旋转角度 (°)",
    tile_submode: "平铺子模式",
    tile_grid: "标准网格",
    tile_brick: "砖墙交错",
    tile_diagonal: "对角线无缝",
    tile_spacing_x: "水平间距 (%)",
    tile_spacing_y: "垂直间距 (%)",
    tile_spacing_hint: "0% = 无缝紧贴，100% = 间距 = 水印尺寸",

    sec_style: "样式参数（透明度 / 缩放）",
    opacity: "透明度",
    scaling_title: "缩放控制",
    scale_relative: "相对缩放 (源短边%)",
    abs_width: "绝对宽度 (px):",
    wm_size_current: "当前尺寸：{w} × {h} px",
    wm_size_original: "原始尺寸：{w} × {h} px",
    wm_not_selected: "未选择水印",
    btn_reset: "重置所有参数",
    reset_done: "已重置所有参数",

    sec_preview_ctrl: "预览控制",
    btn_refresh: "刷新预览",
    auto_preview: "自动实时预览",
    status_auto_on: "自动预览：开",
    status_auto_off: "自动预览：关（需手动刷新）",
    sec_preview: "预览区域",
    btn_next: "下一张",
    btn_prev: "上一张",
    empty_hint: "请先从左侧选择「源图片」开始预览",
    empty_tip: "提示: 支持多选 JPG / PNG / BMP / WEBP 格式",
    previewing: "正在预览：{i}/{total} — {name}  {note}",
    preview_all_note: "（当前图预览，导出处理全部）",
    source_dims: "原图尺寸：{w} × {h} px",
    err_corrupt: "❌ 当前图片损坏: {msg}",
    err_unknown: "未知错误",
    err_preview_load: "❌ 预览加载失败: {e}",

    sec_export: "导出设置",
    name_conflict: "命名冲突策略",
    name_rename: "重命名（加 _wm 后缀）",
    name_overwrite: "覆盖同名文件",
    name_skip: "跳过同名文件",
    export_format: "导出格式",
    fmt_same: "与源图一致",
    jpeg_quality: "JPEG 质量",
    btn_export: "开始批量导出",
    btn_exporting: "导出中...",
    progress_processing: "正在处理 {cur}/{total}  {file}",
    progress_stats: "成功: {ok}  跳过: {skip}  失败: {fail}",
    btn_cancel_export: "取消导出",

    report_title: "导出结果报告",
    report_cancelled: "⚠ 导出已取消",
    report_done: "✅ 导出完成",
    report_total: "总处理：{total} 张",
    report_stats: "成功：{ok} 张  |  跳过：{skip} 张  |  失败：{fail} 张",
    report_cancel_note: "已完成 {cur}/{total} 张，用户中途取消",
    failed_details: "失败详情:",
    btn_open_folder: "打开导出文件夹",
    btn_close: "关闭",

    err_no_source: "请先选择源图片",
    err_no_watermark: "请先选择水印图片",
    err_no_valid_source: "没有有效的源图片",
    err_create_dir: "无法创建输出目录: {e}",
    err_dir_unwritable: "输出目录不存在或无写入权限",
    err_open_source: "无法打开源图: {path}",
    err_output_name: "无法生成输出文件名",
    err_file_exists: "文件已存在: {name}",
};

pub static EN: Tr = Tr {
    app_title: "Batch Image Watermark Tool (Rust + egui)",
    control_panel: "Control Panel",
    lang_tooltip: "Interface language",
    ready: "Ready",

    sec_source: "Source Images",
    btn_pick_images: "Select Images (Multi)",
    btn_append: "Add More",
    filter_images: "Image Files",
    selected_count: "Selected: {total} (valid {valid}, corrupt {bad})",
    btn_clear: "Clear",
    corrupt_mark: "corrupt",
    file_item: "[{i}] {name}  {dims}  {size}",
    file_item_bad: "× [{i}] {name}  (corrupt)",

    sec_watermark: "Watermark Image",
    btn_pick_watermark: "Select PNG Watermark",
    filter_png: "PNG (transparent background recommended)",
    btn_remove: "Remove",
    btn_reselect: "Reselect",
    wm_current: "Current: {name}",
    wm_size: "Size: {w} × {h} px",
    wm_none: "No watermark image selected",
    wm_alpha_tip: "Tip: a PNG with a transparent background gives the best result",
    wm_load_fail: "Failed to read the watermark image",

    sec_output: "Output Folder",
    btn_pick_folder: "Select Folder",
    output_default: "Default: <source folder>/watermarked_output",

    sec_layout: "Layout Mode",
    mode_single: "① Single",
    mode_tiled: "② Tiled",
    anchor_title: "Anchor (9-grid)",
    anchor_tl: "↖ Top Left",
    anchor_tc: "↑ Top Center",
    anchor_tr: "↗ Top Right",
    anchor_ml: "← Middle Left",
    anchor_c: "⊙ Center",
    anchor_mr: "→ Middle Right",
    anchor_bl: "↙ Bottom Left",
    anchor_bc: "↓ Bottom Center",
    anchor_br: "↘ Bottom Right",
    margin: "Margin (px)",
    offset_x: "Offset X: ",
    offset_y: " Offset Y: ",
    rotation: "Rotation (°)",
    tile_submode: "Tile Pattern",
    tile_grid: "Grid",
    tile_brick: "Brick",
    tile_diagonal: "Diagonal",
    tile_spacing_x: "Horizontal spacing (%)",
    tile_spacing_y: "Vertical spacing (%)",
    tile_spacing_hint: "0% = seamless, 100% = gap equals watermark size",

    sec_style: "Style (Opacity / Scale)",
    opacity: "Opacity",
    scaling_title: "Scaling",
    scale_relative: "Scale (% of short edge)",
    abs_width: "Absolute width (px):",
    wm_size_current: "Current size: {w} × {h} px",
    wm_size_original: "Original size: {w} × {h} px",
    wm_not_selected: "No watermark",
    btn_reset: "Reset All Parameters",
    reset_done: "All parameters reset",

    sec_preview_ctrl: "Preview Control",
    btn_refresh: "Refresh Preview",
    auto_preview: "Auto real-time preview",
    status_auto_on: "Auto preview: ON",
    status_auto_off: "Auto preview: OFF (refresh manually)",
    sec_preview: "Preview",
    btn_next: "Next",
    btn_prev: "Prev",
    empty_hint: "Select source images on the left to start previewing",
    empty_tip: "Tip: multi-select supported — JPG / PNG / BMP / WEBP",
    previewing: "Previewing: {i}/{total} — {name}  {note}",
    preview_all_note: "(showing current image; export processes all)",
    source_dims: "Source size: {w} × {h} px",
    err_corrupt: "❌ Corrupted image: {msg}",
    err_unknown: "unknown error",
    err_preview_load: "❌ Failed to load preview: {e}",

    sec_export: "Export Settings",
    name_conflict: "Name Conflict Strategy",
    name_rename: "Rename (add _wm suffix)",
    name_overwrite: "Overwrite existing",
    name_skip: "Skip existing",
    export_format: "Export Format",
    fmt_same: "Same as source",
    jpeg_quality: "JPEG quality",
    btn_export: "Start Batch Export",
    btn_exporting: "Exporting...",
    progress_processing: "Processing {cur}/{total}  {file}",
    progress_stats: "Done: {ok}  Skipped: {skip}  Failed: {fail}",
    btn_cancel_export: "Cancel export",

    report_title: "Export Report",
    report_cancelled: "⚠ Export Cancelled",
    report_done: "✅ Export Finished",
    report_total: "Total: {total} image(s)",
    report_stats: "Success: {ok}  |  Skipped: {skip}  |  Failed: {fail}",
    report_cancel_note: "Cancelled by user after {cur}/{total}",
    failed_details: "Failure details:",
    btn_open_folder: "Open Output Folder",
    btn_close: "Close",

    err_no_source: "Please select source images first",
    err_no_watermark: "Please select a watermark image first",
    err_no_valid_source: "No valid source images",
    err_create_dir: "Cannot create output folder: {e}",
    err_dir_unwritable: "Output folder is missing or not writable",
    err_open_source: "Cannot open source image: {path}",
    err_output_name: "Cannot generate output filename",
    err_file_exists: "File already exists: {name}",
};
