# Batch Image Watermark Tool

批量图片自动水印添加工具，基于 Rust + egui + image 库构建，提供高性能、跨平台的桌面 GUI 应用。

## ✨ 功能特性

- **🖼 多格式支持**：支持 JPEG、PNG、BMP、WebP 格式的源图片读写
- **💧 灵活水印**：支持单个水印与平铺水印两种布局模式
- **📍 9 点锚定**：单个水印支持 9 种预设锚点位置（四角、四边中点、中心）
- **🔄 旋转与缩放**：水印自由旋转角度，支持相对比例缩放与绝对像素尺寸
- **🎛 平铺图案**：网格、砖墙交错、对角线无缝 3 种平铺排列方式
- **👁 实时预览**：所见即所得，参数调整即时预览效果
- **⚡ 并行导出**：基于 Rayon 的数据并行，批量处理速度快
- **💾 配置持久化**：自动保存用户参数，下次启动自动恢复
- **🛡 文件冲突策略**：重命名（加 `_wm` 后缀）、覆盖、跳过 3 种模式
- **📊 导出进度报告**：实时显示处理进度、成功/失败/跳过统计

## 🛠 技术栈

| 组件 | 说明 |
|------|------|
| **Rust** | 主语言，Edition 2021 |
| **egui / eframe** | 即时模式 GUI 框架（winit + wgpu 后端） |
| **image** | 图像编解码与基础处理 |
| **imageproc** | 图像处理扩展（alpha 混合等） |
| **rfd** | 跨平台原生文件/目录选择对话框 |
| **serde / serde_json** | 参数 JSON 序列化持久化 |
| **directories** | 跨平台用户配置目录定位 |
| **rayon** | 数据并行计算 |
| **walkdir** | 目录递归遍历 |
| **anyhow / thiserror** | 错误处理 |

## 📦 环境与构建

### 前置要求

- Rust 工具链（`rustc` ≥ 1.70，`cargo`）
- 系统需支持 wgpu（Vulkan / Metal / DX12）

### 开发运行

```powershell
cargo run
```

### Release 构建（已优化体积）

```powershell
cargo build --release
```

Release 配置已启用：
- `opt-level = 3` 最高优化等级
- `lto = "fat"` 全程序链接时优化
- `codegen-units = 1` 单代码单元
- `strip = true` 剥离调试符号
- `panic = "abort"` 移除 panic 展开代码

生成的可执行文件位于：`target/release/watermark-tool.exe`

## 🚀 使用流程

1. **选择源图片**：点击「📁 选择图片（多选）」批量导入，或「➕ 追加」补充
2. **选择水印图片**：点击「选择 PNG 水印」，推荐使用带透明背景的 PNG
3. **调整布局参数**：
   - **单个水印**：选择锚点位置、边距、偏移、旋转角度
   - **平铺水印**：选择平铺图案（网格/砖墙/对角线）、间距
4. **样式调整**：不透明度、缩放比例（或绝对宽度 px）
5. **设置导出**：输出目录、导出格式、JPEG 质量、文件名冲突策略
6. **实时预览**：右侧预览区即时查看效果
7. **开始导出**：点击「🚀 开始批量导出」，处理完成后显示结果报告

## ⚙ 布局模式说明

### 单个水印（Single）

| 参数 | 说明 |
|------|------|
| 锚点 | 9 种位置（左上、上中、右上、左中、中心、右中、左下、下中、右下） |
| 边距 | 锚点离图片边缘的像素距离 |
| 偏移 X/Y | 基于锚点的额外微调位移 |
| 旋转角度 | 顺时针角度，支持双线性插值旋转 |

### 平铺水印（Tiled）

| 图案 | 说明 |
|------|------|
| 标准网格 | 规则行列对齐 |
| 砖墙交错 | 奇数行水平偏移半个步长 |
| 对角线无缝 | 每行逐行偏移，形成无缝对角线纹理 |

间距以水印尺寸的百分比（%）控制，支持 X/Y 方向独立设置。

## 💾 配置文件位置

用户参数自动保存至系统配置目录：

- **Windows**：`%APPDATA%\watermark\tool\config\config.json`
- **macOS**：`~/Library/Application Support/com.watermark.tool/config.json`
- **Linux**：`~/.config/watermark-tool/config.json`

## 📁 项目结构

```
batch-image-watermark/
├── src/
│   └── main.rs              # 主程序入口（数据结构 + 水印合成核心 + UI）
├── Cargo.toml               # 项目清单与依赖
├── Cargo.lock               # 依赖锁定版本
├── .gitignore
└── watermark-tool.exe       # 预构建的 Windows 可执行文件
```

## 🔧 核心数据结构

- `WatermarkParams`：水印参数（布局、锚点、边距、旋转、缩放、透明度等）
- `PersistedConfig`：持久化配置（含参数、上次输出目录、导出格式、JPEG 质量、命名策略）
- `SourceImageInfo`：源图片元信息（路径、尺寸、大小、有效性）
- `ExportProgress`：导出进度状态（当前/总数、成功/跳过/失败计数、失败列表）

## 📄 License

MIT License
