<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-VISION-STUDIO banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-VISION-STUDIO
<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | 🇨🇳 <b>中文</b> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### 摄像头图像中的基准点：标记、比例和电路板修正

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%246.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.6-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-functional-4CAF50.svg" alt="Maturity">
</p>

---

**诚实检查 - 当前真正能运行的内容：** 一个 Rust 库和一个命令行程序（`eh3d-vision`），有 256 个在电脑上运行的测试：在图像中找到圆形基准点，把像素转换为毫米，并计算使电路板回到设计位置所需的旋转和平移。有一个窗口（用 `--features gui` 构建，程序为 `eh3d-vision-gui`），可打开图像、显示带坐标的标记、设定比例、用网格拟合镜头畸变、与设计对齐并导出 ISOCAM 选项，但窗口尚未在屏幕上看过；没有在机器上试过，也没有用大量真实文件验证过。

---

> **安全状态：** 本程序不控制任何机器，也不是安全系统。它的输出在机器上运行之前必须由人检查；这里的任何内容都没有在真实硬件上运行过。


## 📋 速览

| | |
|---|---|
| 版本 | `0.0.6` |
| 成熟度 | 可用 |
| 测试 | 256 |
| 技术栈 | Rust |
| 真实硬件 | 未试过：这里的任何内容都没有在机器上运行过 |

**跳转到：** [概述](#-概述) · [架构](#-架构) · [快速开始](#-快速开始) · [测试](#-测试) · [结构](#-仓库结构) · [相关项目](#-相关项目)

## 🚦 项目状态

每一行是本仓库的一个部分，灯号表示它今天做到了哪一步。内容依据仓库能够展示的证据（测试、构建、运行），而不是计划。

🟢 已完成并验证 · 🟡 可用，有已知限制 · 🟠 部分完成或尚未验证 · 🔴 未完成或不可用

| 领域 | 状态 | 在这里的含义 |
|---|---|---|
| 核心 | 🟡 | 在图像中找到圆形基准点，把像素转换为毫米，并计算使电路板回到设计位置所需的旋转和平移；已在电脑上用正常和损坏的输入测试，未用大量真实文件验证。 |
| 命令行 | 🟡 | `eh3d-vision` 运行与库相同的函数并打印结果；每个错误都以消息和非零退出码返回。 |
| 测试 | 🟡 | 256 个测试通过（`cargo test`），包含正常情况、边界情况和损坏的输入，以及位于电路板上方的模拟相机：它会加入噪声、曝光错误、失效或缺失的画面、看起来像标记的灰尘或被截断的图像文件，并会被再次请求。 |
| 安全 | 🟡 | 纯安全 Rust（`forbid(unsafe_code)`）：处理收到的文件时不会 panic，错误的值会返回带原因的错误；没有独立审查。 |
| 窗口 | 🟠 | 已编写（egui，`--features gui`，程序 `eh3d-vision-gui`）：打开或拖入图像，查看带坐标的标记，用已知大小的标记设定比例，用网格拟合镜头畸变，与 CSV 设计对齐并导出 ISOCAM 选项；其逻辑已测试，但尚未构建，也未在屏幕上看过。 |
| 真实硬件 | 🔴 | 未试过：这里的任何内容都没有在机器上运行过。 |
| 文档 | 🟡 | 八种语言的 README、更新日志，以及由 `docs/` 绘制的两张图。 |

## 🎯 概述

* **标记**（`src/fiducial.rs`）：用大津阈值把图像分为暗和亮，测量相连的斑点，符合预期大小的圆盘（面积、对外接圆的填充率和细长程度）即为标记，最好的排在前面；被图像边缘截断的斑点因中心不可信而被排除，标记可以是浅底深色或深底浅色。
* **比例与镜头**（`src/scale.rs`）：每个像素的毫米数来自已知大小的标记，机器的轴向上而图像向下，镜头的径向畸变用最小二乘法从标记网格中求出并校正。
* **电路板**（`src/align.rs`）：根据设计中的两个或更多基准点和摄像头找到的对应点，用最小二乘法求出旋转、平移和（若允许）尺寸，并给出以毫米为单位的残差；残差过大（认错了标记）或尺寸偏差（电路板被拉伸）的变换不被接受。

## 🧩 在 EH3D 中的角色

EH3D-VISION-STUDIO 是 EH3D Fabrication Suite 的一个程序。它以库和命令行的形式工作，用 `--features gui` 构建时还是一个与套件其他程序外观一致的窗口。它读写简单的文件，因此可以单独使用，也可以用在其他程序之间。

## 🏛️ 架构

<p align="center">
  <img src="images/architecture.svg" alt="EH3D-VISION-STUDIO 的架构：谁在使用、包含什么、依赖什么" width="100%">
</p>


本仓库各部分如何配合：谁使用它、它包含什么、它依赖什么。图由 `docs/architecture.json` 绘制并与代码核对。

## ✨ 功能

* **库加命令行：** `eh3d-vision` 的每个命令都是库中的函数，套件中的其他程序无需启动进程即可调用。
* **严格的命令行：** 写错的选项（`--powr`）或多余的词会产生指明它的错误，而不是被忽略；否则程序会用默认值生成输出。
* **损坏输入的测试：** 测试还会输入空文件、乱码、巨大数字和被截断的文件，任何情况都不允许 panic。
* **不猜测：** 没有意义的值是一个说明哪个值、为什么的错误，绝不是只是看起来正确的结果。
* **读取文件的大小上限：** 命令行只在图片或点文件不超过 `vision.cfg` 中的 `max_file_mib`（默认 64 MiB）时才读取；更大的文件以代码 `file_too_large` 和文件名拒绝，无法读取的以 `file_unreadable` 拒绝，不是文本的点文件以 `file_not_text` 拒绝，且不加载任何内容。
* **没有 unsafe 代码：** 库和程序中都有 `#![forbid(unsafe_code)]`。
* **给 ISOCAM 用的标记：** `align --isocam-args` 把每一对标记打印成 `eh3d-isocam` 的选项 `--fiducial 设计X,设计Y,测量X,测量Y`。
* **窗口：** 用 `--features gui` 构建为程序 `eh3d-vision-gui`（在 Windows 上不带控制台窗口打开）：打开 PNG 或 PGM 图像，支持缩放和平移，用可编辑的数值查找标记，带坐标绘制，用已知大小的标记设定比例，用标记网格拟合镜头畸变，与 CSV 文件中的设计对齐（标记按位置与其配对，所以顺序无关），仅在对齐通过时才导出 ISOCAM 的 `--fiducial` 选项。支持八种语言，无需网络。
* **配置文件：** `vision.cfg` 保存每个调校过的数值（标记的样子、靶标的网格、可接受对齐的限度、读取的最大文件）；其中的错误会连同行号被拒绝，`examples/vision.cfg` 列出了全部。
* **亚像素级中心与置信度：** `find --refine`（或 `vision.cfg` 中的 `refine_subpixel`）对每个标记边缘的灰色像素加权，使中心精确到像素的几分之一，并输出置信度；标记与电路板几乎没有反差时置信度会降低。
* **不均匀光照：** `--flatten PX`（或 `flatten_window`）在搜索前用每个像素周围的亮度去除图像，这样板子一侧渐暗的灯光或阴影就不会遮住暗侧的标记。
* **十字形标记：** `eh3d-vision crosses 图像` 查找臂沿图像坐标轴的加号（方形外框、两条等宽的臂、四角为空）；圆盘和正方形会被排除，加上 `--turned` 则可查找任意角度的十字（见下）。
* **由靶标间距得到比例：** `eh3d-vision calibrate 图像 --pitch-mm D` 取网格中相邻标记距离的中位数，拒绝不是正对拍摄的靶标，并输出可写入 `vision.cfg` 的 `mm_per_px = ...` 行。
* **给其他程序的零件，使用简单文件：** `eh3d-vision measure 图像 --mm-per-px V` 给出每个零件的中心（相对图像中心的毫米数，x 向右，y 向上）、角度（-90 不含到 90 度）、长度、宽度和面积，格式为 CSV（`index,x_mm,y_mm,angle_deg,length_mm,width_mm,area_mm2`）或 JSON（`--format json`：`format` 为 `eh3d-vision-parts`，`version` 为 1，另有 `unit`、`image_width_px`、`image_height_px`、`mm_per_px`、`parts`）；与读取它们的程序不共享其他任何东西。没有 `--replace` 时，`--output 文件` 绝不会覆盖已有文件，也不会写入链接。
* **一次处理整个文件夹：** `eh3d-vision batch 文件夹 --report 文件.csv` 在大小限制内（最多 `batch_max_files` 张）读取每张 PNG 和 PGM 图片，每张写一行（`file,status,marks,mean_score,min_score,mean_diameter_px,detail`），状态为 `ok`、`no_marks`、`picture_unreadable` 或拒绝代码；电子表格会当作公式执行的文件名，前面会加上一个引号。
* **任意角度的方形与十字：** `eh3d-vision squares 图像` 查找旋转任意角度的方形标记（中心、边长，以及机器所见的逆时针角度，从 -45 不含到 45 度），`eh3d-vision crosses 图像 --turned` 查找臂不沿坐标轴的十字；方形的最小填充率和十字的最低得分（`square_min_fill`、`cross_min_score`）写在 `vision.cfg` 中。
* **两个系数的镜头，不合适则拒绝：** `eh3d-vision lens 图像` 从标记网格求出径向畸变的 `k1` 和 `k2`，并输出供 `vision.cfg` 使用的 `lens_k1 = ...` 和 `lens_k2 = ...` 两行；若拟合后标记仍离其晶格超过 `lens_max_rms` 像素（`lens_residual_too_large`）或某个系数超过 0.5（`lens_out_of_range`）则拒绝，`find --undistort` 用它们校正位置。
* **对每个零件给出判定：** `eh3d-vision inspect 图像 --package 名称 --table packages.csv` 测量图像中的零件，并把每个零件与其封装的本体尺寸比较（CSV 文件中的 `package`、`body_long_mm`、`body_short_mm`，需要时加 `tolerance_mm`；`examples/packages.csv` 里有一些）：以 CSV 或 JSON 给出 `pass` 或 `fail` 及原因（`length`、`width`、`angle`），有零件不合格时退出码为 1；公差和最大旋转角（`inspect_tolerance`、`inspect_max_angle`）写在 `vision.cfg` 中。
* **文件夹的 HTML 报告：** `eh3d-vision batch 文件夹 --html report.html` 写出一个自包含的页面（没有脚本、没有链接、没有其他文件）：图像表格、每张图像的小图（每个找到的标记画一个圆圈）和警告（无法使用的图像、没有标记的图像或得分低于 `report_low_score` 的图像），语言由 `--lang` 或 `vision.cfg` 决定。
* **窗口中的新工具：** 侧边栏新增了带每个标记置信度的亚像素中心、光照窗口、任意角度的方形与十字（画在图像上）、以毫米计的零件（保存为 `<图像>-parts.csv`）、文件夹报告（在独立线程中读取，带进度条和取消按钮；`eh3d-vision-report.html`，没有 `replace_output` 就不会覆盖已有文件）以及由标靶间距得到的比例，文字有八种语言，数值在 `vision.cfg` 中；尚未在屏幕上看过。
* **大型零件的拍摄规划：** `eh3d-vision shots --package 名称 --fov-w MM --fov-h MM`（或 `--body-long` 与 `--body-short`）以 CSV 列出相机位置（相对零件中心的毫米数），使超过视场的零件完全可见；相邻图像重叠 `shots_overlap`，并留有 `shots_margin` 的边距；不需要相机。

## 🔒 安全与防护

| 威胁 | 防御 |
|---|---|
| 损坏、被截断或为空的文件 | 不会 panic：测试在每个位置截断有效输入并输入乱码 |
| 没有意义的数字（负数、非有限值、过大的数） | 以指出该值的错误拒绝；不会被舍入成看似合理的值 |
| 会覆盖其他文件的报告或测量文件 | 只在没有文件的位置写入；覆盖需要 `--replace`，并且绝不会通过链接写入 |
| 不安全的内存使用 | 带 `forbid(unsafe_code)` 的纯安全 Rust |

**没有防御的内容：** 无错误产生的结果不等于经过验证的结果。这里没有任何内容了解机器的极限，程序也没有经过独立审查。请私下报告漏洞（见 [SECURITY.md](SECURITY.md)）。

## 🔌 接口

库为 `eh3d_vision_studio`（`use eh3d_vision_studio::...`），命令行为 `eh3d-vision`。命令如下；每个命令把结果打印到标准输出，把错误以非零退出码报告到标准错误。没有网络访问。

```text
eh3d-vision find IMAGE [--light] [--min-diam PX] [--max-diam PX] [--known-mm D] [--refine] [--undistort] [--flatten PX]   the round marks found (--known-mm gives millimetres, --refine the centre to a fraction of a pixel with a confidence, --undistort straightens the places with lens_k1 and lens_k2 of vision.cfg, --flatten takes uneven light out first)
eh3d-vision align DESIGN.csv MEASURED.csv [--scale] [--pair-nearest] [--isocam-args] [--max-rms 0.1] [--max-stretch 0.01]
eh3d-vision crosses IMAGE [--turned] [--light] [--min-size PX] [--max-size PX] [--flatten PX]   the cross-shaped marks found (plus signs; with --turned the arms may have any angle and the angle is printed)
eh3d-vision inspect IMAGE --package NAME [--table FILE] [--mm-per-px V] [--tolerance MM] [--max-angle DEG] [--light] [--flatten PX] [--format csv|json] [--output FILE] [--replace]   each part of a picture compared with the body of its package from a table (CSV with package, body_long_mm, body_short_mm), one verdict per part; exit 1 when a part fails
eh3d-vision shots (--package NAME [--table FILE] | --body-long MM --body-short MM) --fov-w MM --fov-h MM [--overlap F] [--margin MM] [--output FILE] [--replace]   the places of the camera that cover a part bigger than its field (millimetres from the centre of the part), row after row, as CSV; no camera is needed
eh3d-vision lens IMAGE [--columns C] [--rows R] [--max-rms PX] [--light] [--flatten PX]   the two radial factors of the lens from a grid of marks (columns and rows as grid_columns and grid_rows), refused when the marks stay farther than --max-rms pixels from their lattice
eh3d-vision squares IMAGE [--light] [--min-size PX] [--max-size PX] [--flatten PX]   the square marks found, turned by any angle (centre, side and angle)
eh3d-vision calibrate IMAGE --pitch-mm D [--light] [--tolerance 0.05] [--flatten PX]   the millimetres of a pixel from a target whose neighbouring marks are D millimetres apart
eh3d-vision measure IMAGE [--mm-per-px V] [--light] [--flatten PX] [--format csv|json] [--output FILE] [--replace]   the parts of a picture: centre, turn, length, width and area in millimetres, as a CSV or JSON file
eh3d-vision batch FOLDER [--light] [--flatten PX] [--report FILE.csv] [--html FILE.html] [--lang CODE] [--replace]   every PNG and PGM picture of a folder: marks, scores and diameters, or the code of what went wrong, one line each; --html writes a page that stands alone, with a drawing of each picture and the warnings, in the language of --lang or of vision.cfg
eh3d-vision-gui [IMAGE] [DESIGN.csv]   the window (a build with --features gui); on Windows it opens with no console
```

## ⚙️ 配置

调校的数值保存在简单的 `键 = 值` 文件中，而不是代码里：程序启动所在文件夹或其旁边的 `vision.cfg`（变量 `EH3D_VISION_CONFIG` 可指定另一个文件）。它设定窗口语言、标记是深色还是浅色、阈值、直径范围、标记的填充率和伸长率、用来设定比例的标记大小、网格的列数和行数、对齐是否可以改变尺寸、对齐必须满足的限度以及读取的最大文件。`examples/vision.cfg` 列出每个键及其含义和程序启动时的值；文件不存在就使用这些值，有错误的文件会连同行号被拒绝。命令行中同样的内容以选项给出（见接口）。它还保存其他调好的值：置信度（`refine_subpixel`、`full_contrast`）、光照窗口（`flatten_window`）、比例与零件大小（`mm_per_px`、`part_min_area`、`part_max_area`、`max_parts`）、十字的范围（`cross_min_size`、`cross_max_size`、`cross_max_elongation`）、靶标的容差（`pitch_tolerance`）以及文件夹中图片的最大数量（`batch_max_files`）。 方形的最小填充率和任意角度十字的最低得分是 `square_min_fill` 和 `cross_min_score`。 方形边长的范围以及其外框可比宽度长多少是 `square_min_size`、`square_max_size` 和 `square_max_elongation`，与十字的分开。 镜头由 `lens_k1` 和 `lens_k2`（由 `lens` 求出）表示，校正后网格离其晶格的最大允许距离是 `lens_max_rms`。 `inspect` 读取的封装表是 `package_table`，零件与其封装的最大允许偏差和最大旋转角是 `inspect_tolerance` 和 `inspect_max_angle`。 HTML 报告对某张图像发出警告的得分下限是 `report_low_score`。 设定比例的标靶标记之间的距离是 `pitch_mm`，`replace_output` 决定写出的文件是否可以覆盖已存在的文件（默认不可以）。 `shots` 规划中相邻图像的重叠量和零件周围的边距是 `shots_overlap` 和 `shots_margin`。

## 🚀 快速开始

需要稳定版 Rust 工具链（1.88 或更新）。需要把 EH3D-COMMON 仓库克隆到本仓库旁边：图像读取器通过路径从中引入。

```bash
cargo build --release
cargo test
cargo run --release -- align examples/design.csv examples/measured.csv
cargo run --release --features gui --bin eh3d-vision-gui   # the window
cargo run --release --features gui --bin eh3d-vision-gui -- --shot screen.png --open examples/marks.pgm --open examples/design.csv   # a picture of the window
```

要在不手动打开窗口的情况下截取窗口图像，请使用 `--shot FILE.png` 运行（还可用 `--size WxH`, `--theme forge|dark|light`, `--wait N`, `--lang CODE`, `--open FILE`, `--replace`, `--mark N`, `--align`）：它按该尺寸打开，保存 PNG，不保留任何设置并自动关闭（写入文件成功时退出码为 0，否则为 1，参数错误时为 2）。 使用 `--mark N` 时，第 N 个标记（从 1 起）确定比例；使用 `--align` 时，设计会与标记对齐，因此图像会显示对齐线：从设计放置每个标记的位置到找到的标记，并标出以毫米为单位的距离（可在 `vision.cfg` 中用 `show_residuals` 关闭）。`examples/design_pair.csv` 是 `examples/marks.pgm` 中两个标记的设计。

## 🧪 测试

测试套件有 256 个测试（`cargo test`，CI 在 Windows 和 Linux 上运行）。它们检查正常情况、边界情况和损坏的输入：空文件、乱码和被截断的文件绝不会 panic。CI 还运行 `cargo fmt --check`、把警告当作错误的 `cargo clippy` 以及 `python -I tools/ci_validate.py`。

## 🏗️ 构建与发布

版本号位于 `Cargo.toml` 和 `eh3d.project.json`；每个版本在 [CHANGELOG.md](CHANGELOG.md) 中都有条目。`build.bat` / `build.sh` 构建程序，`test.bat` / `test.sh` 运行测试，`run.bat` / `run.sh` 用示例运行。版本发布在 GitHub 上。

### 下载

每个版本都提供带窗口的程序，适用于 Windows（64 位和 32 位）、Linux（64 位）和 macOS（Apple silicon 和 Intel），文件名为 `<名称>-<版本>-<系统>.zip`（Windows）或 `.tar.gz`，内含程序、许可证、自述文件、图标和示例。这些文件由 GitHub Actions 构建，列在 `SHA256SUMS` 中，并使用 EH3D 的密钥签名。要检查下载的文件，请将其哈希与旁边的 `.sha256` 文件比较（`sha256sum -c`），并用旁边的 `.sig` 文件检查签名（`ssh-keygen -Y verify`）。

## 📂 仓库结构

<p align="center">
  <img src="images/repository-structure.svg" alt="EH3D-VISION-STUDIO 的仓库结构" width="100%">
</p>

<details>
<summary>目录树的文本版</summary>

```text
EH3D-VISION-STUDIO/
├── src/
│   ├── lib.rs                                   库：每个命令所使用的部分
│   ├── main.rs                                  命令行
│   ├── gui_main.rs                              作为独立程序的窗口（gui 特性）
│   ├── cli_args.rs                              命令行参数：每个小工具中的同一个文件
│   ├── align.rs                                 电路板的旋转、平移和尺寸及其与设计的配对
│   ├── batch.rs                                 一次处理整个文件夹的图片，并生成 CSV 报告
│   ├── config.rs                                配置文件：每个调校过的数值
│   ├── cross.rs                                 十字形基准标记
│   ├── fiducial.rs                              图像中的圆形标记：阈值、斑点和检查
│   ├── flatten.rs                               去除图片中不均匀的光照
│   ├── grid.rs                                  由标记网格得出的镜头畸变
│   ├── inspect.rs                               把每个零件与表中封装的本体尺寸比较并给出判定
│   ├── job.rs                                   在独立线程中读取文件夹报告，带进度与取消
│   ├── lens.rs                                  由标记网格求镜头的两个系数，不合适则拒绝
│   ├── points.rs                                设计的点文件
│   ├── report.rs                                把文件夹报告输出为八种语言的自包含 HTML 页面
│   ├── limit.rs                                 带大小上限的文件读取，每种拒绝都有代码
│   ├── measure.rs                               以毫米测量零件，输出 CSV 和 JSON 文件
│   ├── scale.rs                                 像素到毫米，以及镜头的径向畸变
│   ├── square.rs                                方形标记与任意角度旋转的十字
│   ├── tools.rs                                 侧边栏中针对图像的工具：形状、零件、文件夹报告、由间距求比例
│   ├── shot.rs                                  窗口的截图模式（--shot）：无需屏幕即可读取和检查其参数
│   ├── shots.rs                                 仅由尺寸求出覆盖超出视场的零件所需的相机位置
│   ├── browse.rs                                打开文件窗口的文件列表
│   ├── gui.rs                                   窗口（egui，gui 特性）：绘制状态并传递用户的操作
│   ├── lang.rs                                  窗口的文字（八种语言）
│   ├── viewport.rs                              图像的缩放和平移（纯数值）
│   └── window.rs                                无需屏幕的窗口状态：图像、标记、比例、镜头和对齐
├── tests/
│   ├── readme.rs                                运行 README 和 run 脚本所承诺内容的测试
│   ├── strict.rs                                命令行测试：它不认识的选项和词
│   ├── read_limit.rs                            命令行所读文件的大小上限测试
│   ├── exchange.rs                              读取图片并写出文件的命令的测试
│   ├── shot_sample.rs                           示例能在截图窗口中打开的测试
│   └── vision.rs                                库和命令行的测试
├── examples/
│   ├── design.csv                               试用的示例
│   ├── design_pair.csv                          marks.pgm 中两个标记的设计
│   ├── marks.pgm                                试用的示例
│   ├── measured.csv                             试用的示例
│   ├── packages.csv                             供 inspect 使用的封装本体尺寸表
│   └── vision.cfg                               列出每个数值及其含义的配置文件
├── docs/
│   ├── architecture.json                        架构图数据
│   └── structure.json                           结构图数据
├── tools/                                       CI 和 README 检查及绘图工具
├── images/                                      横幅图以及架构和仓库结构图片
├── .github/                                     CI 与发布工作流、Issue 和 PR 模板
├── eh3d.project.json                            清单：版本、角色、成熟度和构建命令
├── Cargo.toml Cargo.lock                        工作区及其锁定的依赖
├── rust-toolchain.toml                          固定的 Rust 工具链
├── CHANGELOG.md                                 每个版本的变更
├── SECURITY.md SUPPORT.md                       如何报告漏洞及在哪里提问
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   参与贡献、行为准则和许可证
```

</details>

## 🗺️ 路线图与已知限制

**目前已知的限制**

* 窗口尚未在屏幕上看过；标记按位置与设计配对，因此旋转很大的电路板或缺失的标记会被拒绝，而不是被猜测。
* 没有在机器上试过，也没有用大量真实文件验证过，也没有独立审查。
* 它读取图像文件而不是实时摄像头，只能找圆形、方形和十字形标记，只在测试绘制的图像上试过，没有用真实电路板的照片试过。零件的角度按矩形由其像素测得，因此不是矩形的零件得到的是其长轴的角度。

**下一步**

* 读取机器的实时摄像头（EH3D-NODE-BUNGARD-AUX），用真实电路板的照片试验，并在窗口中加入新标记的训练。
* 在屏幕上查看窗口并完善它。
* 用真实文件和套件的其他程序进行试用。

## 🌍 语言

README 和文本提供英语、西班牙语、法语、意大利语、德语、汉语、日语和欧洲葡萄牙语版本。

## 🔗 相关项目

* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON)：共享 crate，其中包括图像读取器
* [EH3D-PHOENIX-PNP](https://github.com/JuanenRac/EH3D-PHOENIX-PNP): 贴片装配
* [EH3D-ISOCAM](https://github.com/JuanenRac/EH3D-ISOCAM): 铜箔隔离铣削
* [EH3D-PHOENIX-CNC](https://github.com/JuanenRac/EH3D-PHOENIX-CNC): 机器控制程序
* [EH3D-NODE-BUNGARD-AUX](https://github.com/JuanenRac/EH3D-NODE-BUNGARD-AUX): 带摄像头的钻头节点
* [EH3D-SUITE-LAUNCHER](https://github.com/JuanenRac/EH3D-SUITE-LAUNCHER): 查找并启动套件程序的中心

## 📚 文档

* [更新日志](CHANGELOG.md)
* [架构与结构数据](docs/architecture.json)：绘制两张图所用的数据。

## 🤝 参与贡献与社区

* 提交拉取请求前请运行 `cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test` 和 `python -I tools/ci_validate.py`：必须全部通过。见 [CONTRIBUTING.md](CONTRIBUTING.md) 和[行为准则](CODE_OF_CONDUCT.md)。
* README 有八种语言：修改文字要在八个文件中同时进行，两张图由 `tools/make_architecture.py` 和 `tools/make_structure.py` 根据 `docs/architecture.json` 和 `docs/structure.json` 绘制。
* 请私下报告漏洞：[SECURITY.md](SECURITY.md)。问题、想法和报告：electrohobby3d@gmail.com（[SUPPORT.md](SUPPORT.md)）。

## 👤 作者

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 许可证

软件：GPL-3.0-or-later。见 [LICENSE](LICENSE)。套件的硬件设计使用 CERN OHL-S。
