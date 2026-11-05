<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-VISION-STUDIO banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-VISION-STUDIO
<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | 🇨🇳 <b>中文</b> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### 摄像头图像中的基准点：标记、比例和电路板修正

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.2-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**诚实检查 - 当前真正能运行的内容：** 一个 Rust 库和一个命令行程序（`eh3d-vision`），有 128 个在电脑上运行的测试：在图像中找到圆形基准点，把像素转换为毫米，并计算使电路板回到设计位置所需的旋转和平移。有一个窗口（用 `--features gui` 构建，程序为 `eh3d-vision-gui`），可打开图像、显示带坐标的标记、设定比例、用网格拟合镜头畸变、与设计对齐并导出 ISOCAM 选项，但窗口尚未在屏幕上看过；没有在机器上试过，也没有用大量真实文件验证过。

---

> **安全状态：** 本程序不控制任何机器，也不是安全系统。它的输出在机器上运行之前必须由人检查；这里的任何内容都没有在真实硬件上运行过。


## 📋 速览

| | |
|---|---|
| 版本 | `0.0.2` |
| 成熟度 | 脚手架 |
| 测试 | 128 |
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
| 测试 | 🟡 | 128 个测试通过（`cargo test`），包含正常情况、边界情况和损坏的输入。 |
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
* **没有 unsafe 代码：** 库和程序中都有 `#![forbid(unsafe_code)]`。
* **给 ISOCAM 用的标记：** `align --isocam-args` 把每一对标记打印成 `eh3d-isocam` 的选项 `--fiducial 设计X,设计Y,测量X,测量Y`。
* **窗口：** 用 `--features gui` 构建为程序 `eh3d-vision-gui`（在 Windows 上不带控制台窗口打开）：打开 PNG 或 PGM 图像，支持缩放和平移，用可编辑的数值查找标记，带坐标绘制，用已知大小的标记设定比例，用标记网格拟合镜头畸变，与 CSV 文件中的设计对齐（标记按位置与其配对，所以顺序无关），仅在对齐通过时才导出 ISOCAM 的 `--fiducial` 选项。支持八种语言，无需网络。
* **配置文件：** `vision.cfg` 保存每个调校过的数值（标记的样子、靶标的网格、可接受对齐的限度、读取的最大文件）；其中的错误会连同行号被拒绝，`examples/vision.cfg` 列出了全部。

## 🔒 安全与防护

| 威胁 | 防御 |
|---|---|
| 损坏、被截断或为空的文件 | 不会 panic：测试在每个位置截断有效输入并输入乱码 |
| 没有意义的数字（负数、非有限值、过大的数） | 以指出该值的错误拒绝；不会被舍入成看似合理的值 |
| 不安全的内存使用 | 带 `forbid(unsafe_code)` 的纯安全 Rust |

**没有防御的内容：** 无错误产生的结果不等于经过验证的结果。这里没有任何内容了解机器的极限，程序也没有经过独立审查。请私下报告漏洞（见 [SECURITY.md](SECURITY.md)）。

## 🔌 接口

库为 `eh3d_vision_studio`（`use eh3d_vision_studio::...`），命令行为 `eh3d-vision`。命令如下；每个命令把结果打印到标准输出，把错误以非零退出码报告到标准错误。没有网络访问。

```text
eh3d-vision find IMAGE [--light] [--min-diam PX] [--max-diam PX] [--known-mm D]   the round marks found (--known-mm gives millimetres)
eh3d-vision align DESIGN.csv MEASURED.csv [--scale] [--isocam-args] [--max-rms 0.1] [--max-stretch 0.01]
eh3d-vision-gui [IMAGE] [DESIGN.csv]   the window (a build with --features gui); on Windows it opens with no console
```

## ⚙️ 配置

调校的数值保存在简单的 `键 = 值` 文件中，而不是代码里：程序启动所在文件夹或其旁边的 `vision.cfg`（变量 `EH3D_VISION_CONFIG` 可指定另一个文件）。它设定窗口语言、标记是深色还是浅色、阈值、直径范围、标记的填充率和伸长率、用来设定比例的标记大小、网格的列数和行数、对齐是否可以改变尺寸、对齐必须满足的限度以及读取的最大文件。`examples/vision.cfg` 列出每个键及其含义和程序启动时的值；文件不存在就使用这些值，有错误的文件会连同行号被拒绝。命令行中同样的内容以选项给出（见接口）。

## 🚀 快速开始

需要稳定版 Rust 工具链（1.88 或更新）。需要把 EH3D-COMMON 仓库克隆到本仓库旁边：图像读取器通过路径从中引入。

```bash
cargo build --release
cargo test
cargo run --release -- align examples/design.csv examples/measured.csv
cargo run --release --features gui --bin eh3d-vision-gui   # the window
```

## 🧪 测试

测试套件有 128 个测试（`cargo test`，CI 在 Windows 和 Linux 上运行）。它们检查正常情况、边界情况和损坏的输入：空文件、乱码和被截断的文件绝不会 panic。CI 还运行 `cargo fmt --check`、把警告当作错误的 `cargo clippy` 以及 `python -I tools/ci_validate.py`。

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
│   ├── config.rs                                配置文件：每个调校过的数值
│   ├── fiducial.rs                              图像中的圆形标记：阈值、斑点和检查
│   ├── grid.rs                                  由标记网格得出的镜头畸变
│   ├── points.rs                                设计的点文件
│   ├── scale.rs                                 像素到毫米，以及镜头的径向畸变
│   ├── browse.rs                                打开文件窗口的文件列表
│   ├── gui.rs                                   窗口（egui，gui 特性）：绘制状态并传递用户的操作
│   ├── lang.rs                                  窗口的文字（八种语言）
│   ├── viewport.rs                              图像的缩放和平移（纯数值）
│   └── window.rs                                无需屏幕的窗口状态：图像、标记、比例、镜头和对齐
├── tests/
│   ├── readme.rs                                运行 README 和 run 脚本所承诺内容的测试
│   ├── strict.rs                                命令行测试：它不认识的选项和词
│   └── vision.rs                                库和命令行的测试
├── examples/
│   ├── design.csv                               试用的示例
│   ├── marks.pgm                                试用的示例
│   ├── measured.csv                             试用的示例
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
* 它读取图像文件而不是实时摄像头，只能找圆形标记（不能找十字或方块），只在测试绘制的图像上试过，没有用真实电路板的照片试过。

**下一步**

* 读取机器的实时摄像头（EH3D-NODE-BUNGARD-AUX），识别十字和方形，用真实电路板的照片试验，并在窗口中加入新标记的训练。
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
