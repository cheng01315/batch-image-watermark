<div align="center">

# 🌊 Watermark Tool

**批量图片水印工具** — 基于 Rust + egui 的桌面端图形化应用，支持自定义水印位置、平铺模式、透明度、批量导出，并内置中英双语界面。

[![Rust](https://img.shields.io/badge/Rust-1.75%2B-000?logo=rust&logoColor=fff)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-Windows-blue)](#)

> 📖 **English version / 英文版**：[README_EN.md](./README_EN.md)

</div>

---

## 🖼️ 软件预览
![项目示例图片](https://raw.githubusercontent.com/cheng01315/batch-image-watermark/main/others/image.jpg)

## ✨ 功能特性

### 🎯 水印布局引擎
- **单水印模式**：9 种预设锚点（左上 / 上中 / 右上 / 左中 / 中心 / 右中 / 左下 / 下中 / 右下），配合边距和 XY 方向微调
- **平铺水印模式**：3 种平铺策略
  - 标准网格（Grid）
  - 砖墙交错（Brick）
  - 对角线无缝（Diagonal）
- **可调平铺间距**：X / Y 方向独立设置间距百分比
- **任意角度旋转**：支持水印旋转 ±180°

### 🎨 水印效果控制
- **透明度（Opacity）**：0% ~ 100% 无级调节，Alpha 通道正确混合
- **缩放控制**：基于原图短边百分比缩放，或强制指定绝对像素宽度
- **重采样滤镜**：使用高质量 Lanczos3 算法进行缩放，避免模糊锯齿

### 📦 批量导出引擎
- **多线程并发**：基于标准库 `std::thread` + `mpsc` 通道实现，导出在独立线程执行，UI 不卡顿
- **实时进度条**：显示当前完成数 / 总数 / 百分比进度
- **命名冲突策略**：
  - 自动重命名（追加 `_wm` 后缀，冲突时再加序号）
  - 覆盖同名文件
  - 跳过同名文件
- **格式转换**：保持原图格式 / 统一输出 JPEG / 统一输出 PNG
- **JPEG 质量调节**：1 ~ 100 可配置

### 🌐 中英双语界面
- **自动识别系统语言**：中文系统（Windows UI 语言为中文）启动时默认中文；其它语言默认英文
- **手动切换**：控制面板右上角一键切换「中文 / English」，选择会被持久化
- **环境变量覆盖**（可选）：`WATERMARK_TOOL_LANG=zh` 或 `=en` 可强制指定语言

### 💾 参数持久化
- 配置自动保存到系统用户配置目录（Windows：`%APPDATA%` 下的应用专属子目录 `config.json`，由 `directories` crate 自动定位）
- 下次启动自动恢复上次所有参数与语言选择
- 支持 PNG / JPEG / BMP / WebP 四种输入格式

### 🖼️ 预览与交互
- 左侧控制面板 + 右侧预览画布的经典布局，控制面板分区默认展开
- 水印参数修改后**实时预览**（自动缩放适配窗口）
- 系统原生文件/文件夹选择对话框（`rfd`）
- Toast 风格消息提示（成功 / 警告 / 错误）
- 导出完成后弹出结果报告（成功数、失败数、跳过数、文件列表）

---

## 🚀 快速使用

### 方式一：下载即用（推荐）
直接下载仓库根目录的 `watermark-tool.exe`，双击运行即可，无需安装任何运行库。

### 方式二：从源码构建
见下方 🛠️ 构建指南章节。

---

## 🎮 使用流程

```
┌─────────────────────────────────────────────────────────────┐
│  ① 选择水印图片 ────►  选择 PNG/JPG/BMP/WebP 作为水印图案  │
│  ② 选择输入图片 ─────► 多选源图片，可追加或清空            │
│  ③ 调整水印参数 ─────► 位置 / 平铺 / 透明度 / 缩放 / 旋转 │
│  ④ 预览效果 ────────► 右侧画布实时查看合成效果             │
│  ⑤ 选择输出目录 ────► 设置导出格式、质量、命名策略        │
│  ⑥ 开始批量导出 ────► 实时进度条 + 完成报告               │
└─────────────────────────────────────────────────────────────┘
```

---

## 🛠️ 构建指南

### 前置要求
- Rust 工具链 1.75+（推荐通过 [rustup](https://rustup.rs/) 安装）

### Debug 构建（开发调试）
```powershell
cd watermark-tool
cargo build
.\target\debug\watermark-tool.exe
```

### Release 构建（发布优化）
```powershell
cargo build --release
.\target\release\watermark-tool.exe
```

Release 配置已在 `Cargo.toml` 中开启以下优化：
| 选项 | 值 | 效果 |
|------|----|------|
| `opt-level` | 3 | 最高级别优化 |
| `lto` | fat | 全程序链接时优化，显著减小体积 |
| `codegen-units` | 1 | 单编译单元，最大化 LTO 效果 |
| `strip` | true | 移除调试符号与符号表 |
| `panic` | abort | 移除 panic 展开代码，更小更快 |

最终单文件 EXE 体积约 **5 ~ 8 MB**，无外部 DLL 依赖。

---

## 🧱 技术栈

| 类别 | 库 | 版本 | 用途 |
|------|----|------|------|
| GUI 框架 | eframe + egui | 0.28 | 即时模式跨平台桌面 UI |
| 图像编解码 | image | 0.25 | PNG/JPEG/BMP/WebP 读写 |
| 文件对话框 | rfd | 0.14 | 原生系统文件/目录选择 |
| 序列化 | serde + serde_json | 1.x | 参数持久化 |
| 用户目录 | directories | 6.x | 跨平台用户配置目录 |
| 错误处理 | anyhow | 1.x | 便捷错误管理（`Context` / `Result`） |
| 国际化 | 自实现（i18n.rs） | — | 中英双语文案与系统语言探测（无第三方依赖） |

---

## 📁 项目结构

```
watermark-tool/
├── Cargo.toml            # 项目配置与依赖声明
├── Cargo.lock            # 依赖版本锁定
├── README.md             # 本文档（简体中文）
├── README_EN.md          # 英文版文档
├── watermark-tool.exe    # 预编译 Windows 可执行文件
├── others/
│   └── image.jpg         # README 预览图
└── src/
    ├── main.rs           # 主程序（约 1800 行）：数据结构 / 合成核心 / 导出引擎 / UI 层
    └── i18n.rs           # 国际化模块：ZH / EN 文案表 + 系统语言探测
```

### src/main.rs 内部分层（从上至下）
1. **数据结构定义**：锚点、布局模式、平铺策略、命名冲突、导出格式、参数结构体
2. **配置持久化**：`load_config()` / `save_config()`（写入 `config.json`）
3. **水印合成核心**：
   - `resize_watermark()` — 水印缩放（Lanczos3）
   - `apply_opacity()` — 透明度混合
   - `rotate_image()` — 旋转
   - `alpha_composite()` — Alpha 通道叠加
   - `compute_single_position()` — 单图位置计算
   - `compose_watermark()` — 单水印 / 平铺总入口
   - `determine_output_format()` / `resolve_conflict_path()` — 导出格式与命名
4. **批量导出引擎**：`process_single_image()` + `start_export()`（独立线程 `thread::spawn` + `mpsc` 进度上报）
5. **UI 层**：`WatermarkApp` 及 `eframe::App` 实现（左侧面板 / 预览画布 / 弹窗 / Toast / 语言切换）
6. **主函数**：`eframe::run_native` 启动原生窗口

---

## 📌 支持的图片格式

| 格式 | 读取 | 导出 | 备注 |
|------|------|------|------|
| PNG | ✅ | ✅ | 支持透明通道 |
| JPEG | ✅ | ✅ | 可调压缩质量 |
| BMP | ✅ | ❌（输出需转 JPEG/PNG） |  |
| WebP | ✅ | ❌（输出需转 JPEG/PNG） |  |

---

## ⚠️ 注意事项

1. **水印图片建议使用带透明通道的 PNG**，合成效果最佳
2. 平铺模式下间距过小会导致水印重叠（预览可观察实际效果）
3. 批量处理大量大尺寸图片时会占用较多内存，建议分批处理
4. Release 构建首次编译较慢（全程序 LTO 优化），请耐心等待

---

## 📄 开源协议

本项目基于 [LICENSE](./LICENSE) 文件中的协议发布。
