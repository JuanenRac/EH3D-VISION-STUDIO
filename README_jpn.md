<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-VISION-STUDIO banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-VISION-STUDIO
<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | 🇯🇵 <b>日本語</b> | <a href="README_por.md">🇵🇹 Português</a></p>

### カメラ画像の基準マーク：マーク、スケール、基板の補正

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%246.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.6-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-functional-4CAF50.svg" alt="Maturity">
</p>

---

**正直さの確認 - 現在実際に動くもの：** Rust ライブラリとコマンドライン プログラム（`eh3d-vision`）で、コンピューター上で実行される 246 件のテストがあります：画像から丸い基準マークを見つけ、画素をミリメートルに変換し、基板を設計どおりの位置に合わせる回転と移動量を計算します。ウィンドウ（`--features gui` でビルドする `eh3d-vision-gui` プログラム）があり、画像を開き、マークを座標つきで表示し、スケールを設定し、グリッドからレンズの歪みを求め、設計と位置合わせして ISOCAM 用のオプションを書き出しますが、ウィンドウはまだ画面で確認していません。機械では試しておらず、実際のファイルの集まりでも確認していません。

---

> **安全とセキュリティの状況：** このプログラムは機械を制御せず、安全システムでもありません。出力は、機械で何かを実行する前に人が確認する必要があります。ここにあるものは実機で動かしたことがありません。


## 📋 ひと目で

| | |
|---|---|
| バージョン | `0.0.6` |
| 成熟度 | 機能する |
| テスト | 246 |
| 技術スタック | Rust |
| 実機での動作 | 未試験：ここにあるものは機械で動かしたことがありません |

**移動先：** [概要](#-概要) · [アーキテクチャ](#-アーキテクチャ) · [クイックスタート](#-クイックスタート) · [テスト](#-テスト) · [構成](#-リポジトリの構成) · [関連](#-関連プロジェクト)

## 🚦 プロジェクトの状況

各行はこのリポジトリの一部分で、信号は今日どこまで進んでいるかを示します。計画ではなく、リポジトリが示せるもの（テスト、ビルド、実行）に基づいて書かれています。

🟢 完了・検証済み · 🟡 動作する（既知の制限あり） · 🟠 一部のみ、または未検証 · 🔴 未実装または動作しない

| 領域 | 状態 | ここでの意味 |
|---|---|---|
| コア | 🟡 | 画像から丸い基準マークを見つけ、画素をミリメートルに変換し、基板を設計どおりの位置に合わせる回転と移動量を計算します。コンピューター上で通常入力と破損入力でテスト済み。実際のファイルの集まりでは未確認。 |
| コマンドライン | 🟡 | `eh3d-vision` はライブラリと同じ関数を実行して結果を表示します。エラーはメッセージと 0 以外の終了コードで返ります。 |
| テスト | 🟡 | 246 件のテストが通ります（`cargo test`）。通常ケース、境界ケース、破損入力に加え、ノイズ、誤った露出、無効または欠落したフレーム、マークに見えるほこり、途切れた画像ファイルを加え、再取得される基板上の模擬カメラを含みます。 |
| セキュリティ | 🟡 | 安全な Rust のみ（`forbid(unsafe_code)`）。受け取ったファイルでパニックせず、不正な値は理由付きのエラーになります。独立したレビューはありません。 |
| ウィンドウ | 🟠 | 作成済み（egui、`--features gui`、プログラム `eh3d-vision-gui`）：画像を開く／ドロップする、マークを座標つきで見る、大きさが既知のマークでスケールを設定する、グリッドからレンズの歪みを求める、CSV の設計と位置合わせする、ISOCAM 用オプションを書き出す。ロジックはテスト済みですが、まだビルドも画面での確認もしていません。 |
| 実機 | 🔴 | 未試験：ここにあるものは機械で実行したことがありません。 |
| ドキュメント | 🟡 | 8 言語の README、変更履歴、`docs/` から描いた 2 枚の図。 |

## 🎯 概要

* **マーク**（`src/fiducial.rs`）：大津の閾値で画像を暗部と明部に分け、連結した斑点を測り、期待する大きさの円板（面積、外接円の充填率、細長さ）であるものをマークとして、良いものから返します。画像の端で切れた斑点は中心が信頼できないため除外し、マークは明地に暗、暗地に明のどちらも扱えます。
* **スケールとレンズ**（`src/scale.rs`）：1 画素のミリメートルは大きさが既知のマークから求め、機械の軸は上向き、画像は下向きです。レンズの放射歪みはマークの格子から最小二乗法で求めて補正します。
* **基板**（`src/align.rs`）：設計の 2 つ以上の基準点とカメラが見つけた同じ点から、回転、移動、（許可すれば）大きさを最小二乗法で求め、残差をミリメートルで示します。残差が大きすぎる（マークの取り違え）、大きさがずれている（基板の伸び）場合は受け入れません。

## 🧩 EH3D での役割

EH3D-VISION-STUDIO は EH3D Fabrication Suite のプログラムの一つです。ライブラリとコマンドラインとして動作し、`--features gui` でビルドすると、スイートの他のプログラムと共通の外観のウィンドウとしても動作します。単純なファイルを読み書きするため、単独でも他のプログラムの間でも使えます。

## 🏛️ アーキテクチャ

<p align="center">
  <img src="images/architecture.svg" alt="EH3D-VISION-STUDIO のアーキテクチャ：誰が使い、何を持ち、何に依存するか" width="100%">
</p>


このリポジトリの各部分の関係：誰が使い、何を持ち、何に依存するか。図は `docs/architecture.json` から描かれ、コードと照合されます。

## ✨ 機能

* **ライブラリとコマンドライン：** `eh3d-vision` の各コマンドはライブラリの関数なので、スイートの他のプログラムがプロセスを起動せずに呼び出せます。
* **厳密なコマンドライン：** 書き間違えたオプション（`--powr`）や余分な語は無視されず、それを名指しするエラーになります。そうでないと、プログラムは既定値で出力を作ってしまいます。
* **破損入力のテスト：** 空のファイル、ゴミデータ、巨大な数値、途中で切れたファイルも与え、どれもパニックしてはいけません。
* **推測しない：** 意味のない値は、どの値かと理由を示すエラーになり、見かけだけ正しい結果にはなりません。
* **読み込むファイルのサイズ上限：** コマンドラインは、画像や点のファイルが `vision.cfg` の `max_file_mib`（既定は 64 MiB）以下のときだけ読み込みます。それより大きいファイルはコード `file_too_large` とファイル名で、読めないファイルは `file_unreadable` で、テキストでない点のファイルは `file_not_text` で拒否され、何も読み込まれません。
* **unsafe コードなし：** ライブラリとプログラムに `#![forbid(unsafe_code)]`。
* **ISOCAM 用のマーク：** `align --isocam-args` は各マークの組を `eh3d-isocam` のオプション `--fiducial 設計X,設計Y,測定X,測定Y` として出力します。
* **ウィンドウ：** `--features gui` でプログラム `eh3d-vision-gui` としてビルドします（Windows ではコンソールなしで開きます）。PNG または PGM の画像をズームと移動つきで開き、編集できる値でマークを見つけ、座標つきで描画し、大きさが既知のマークでスケールを設定し、マークのグリッドからレンズの歪みを求め、CSV ファイルの設計と位置合わせし（マークは位置で対応付けるので順序は関係ありません）、位置合わせが合格のときだけ ISOCAM のオプション `--fiducial` を書き出します。8 言語に対応し、ネットワークは不要です。
* **設定ファイル：** `vision.cfg` に調整されるすべての値（マークの見た目、ターゲットのグリッド、合格する位置合わせの限度、読み込む最大ファイル）を書きます。間違いは行番号つきで拒否され、`examples/vision.cfg` がすべてを一覧にしています。
* **1 画素未満の精度の中心と信頼度：** `find --refine`（または `vision.cfg` の `refine_subpixel`）は各マークの縁のグレーの画素に重みを付けるので、中心が 1 画素未満の精度で求まり、マークが基板からほとんど目立たないと下がる信頼度を表示します。
* **ムラのある照明：** `--flatten PX`（または `flatten_window`）は検索の前に画像を各画素の周りの明るさで割るので、基板の一方で弱くなるランプや影が暗い側のマークを隠すことはありません。
* **十字形のマーク：** `eh3d-vision crosses 画像` は腕が画像の軸に沿ったプラス記号を見つけます（正方形の枠、同じ幅の 2 本の腕、空いた四隅）。円板と正方形は除外され、`--turned` を付けると任意の角度の十字を見つけます（下記参照）。
* **ターゲットのピッチからのスケール：** `eh3d-vision calibrate 画像 --pitch-mm D` はグリッドの隣り合うマークの距離の中央値を取り、正面から見ていないターゲットを拒否し、`vision.cfg` 用の `mm_per_px = ...` の行を表示します。
* **他のプログラム向けの部品、単純なファイルで：** `eh3d-vision measure 画像 --mm-per-px V` は各部品の中心（画像の中心からの mm、x は右、y は上）、回転（-90 を除く 90 度まで）、長さ、幅、面積を CSV（`index,x_mm,y_mm,angle_deg,length_mm,width_mm,area_mm2`）または JSON（`--format json`：`format` は `eh3d-vision-parts`、`version` は 1、`unit`、`image_width_px`、`image_height_px`、`mm_per_px`、`parts`）で出力し、読むプログラムとは他に何も共有しません。`--output ファイル` は `--replace` なしでは既存のファイルにも、リンクにも書き込みません。
* **フォルダをまとめて：** `eh3d-vision batch フォルダ --report ファイル.csv` はサイズ上限内（最大 `batch_max_files` 枚）のすべての PNG と PGM を読み、画像ごとに 1 行（`file,status,marks,mean_score,min_score,mean_diameter_px,detail`）を書きます。状態は `ok`、`no_marks`、`picture_unreadable`、または拒否のコードです。表計算が数式として実行しそうなファイル名は、先頭に引用符を付けて書かれます。
* **任意の角度の正方形と十字：** `eh3d-vision squares 画像` は任意の角度に回った正方形のマークを見つけ（中心、辺、機械から見て反時計回りの角度、-45 を除き 45 度まで）、`eh3d-vision crosses 画像 --turned` は腕が軸に沿っていない十字を見つけます。正方形の最小の充填率と十字の最小スコア（`square_min_fill`、`cross_min_score`）は `vision.cfg` にあります。
* **2 つの係数を持つレンズ、合わなければ拒否：** `eh3d-vision lens 画像` はマークのグリッドから放射歪みの `k1` と `k2` を求め、`vision.cfg` 用に `lens_k1 = ...` と `lens_k2 = ...` の行を出力します。補正後もマークが格子から `lens_max_rms` ピクセルより離れる当てはめ（`lens_residual_too_large`）や、0.5 を超える係数（`lens_out_of_range`）は拒否され、`find --undistort` はそれらで位置を補正します。
* **部品ごとの判定：** `eh3d-vision inspect 画像 --package 名前 --table packages.csv` は画像の部品を計測し、それぞれをパッケージの本体寸法（CSV ファイルの `package`、`body_long_mm`、`body_short_mm`、必要なら `tolerance_mm`。`examples/packages.csv` にいくつか入っています）と比較して、`pass` か `fail` を理由（`length`、`width`、`angle`）とともに CSV または JSON で出し、部品が不合格なら終了コードは 1 です。許容差と最大の回転（`inspect_tolerance`、`inspect_max_angle`）は `vision.cfg` にあります。
* **フォルダの HTML レポート：** `eh3d-vision batch フォルダ --html report.html` は単独で完結する 1 ページ（スクリプト、リンク、他のファイルなし）を書き出します。画像の表、見つかった各マークに円を付けた小さな絵、警告（使用できなかった画像、マークがない画像、`report_low_score` を下回るスコアの画像）が入り、言語は `--lang` または `vision.cfg` に従います。
* **ウィンドウの新しいツール：** サイドパネルに、各マークの信頼度付きのサブピクセル中心、照明のウィンドウ、任意の角度の正方形と十字（画像上に描画）、ミリメートルの部品（`<画像>-parts.csv` として保存）、フォルダのレポート（`eh3d-vision-report.html`。`replace_output` がなければ既存ファイルに上書きしません）、ターゲットの間隔からのスケールが加わり、文言は 8 言語、値は `vision.cfg` にあります。画面ではまだ確認していません。
* **大きな部品の撮影計画：** `eh3d-vision shots --package 名前 --fov-w MM --fov-h MM`（または `--body-long` と `--body-short`）は、視野より大きな部品の全体が見えるカメラの位置を、部品の中心からのミリメートルで CSV に並べます。隣り合う画像は `shots_overlap` だけ重なり、`shots_margin` の余白を含みます。カメラは不要です。

## 🔒 セキュリティと安全

| 脅威 | 対策 |
|---|---|
| 破損・途中切れ・空のファイル | パニックなし：テストは有効な入力をあらゆる位置で切り、ゴミデータも与えます |
| 意味のない数値（負、無限、極端に大きい） | 値を示すエラーで拒否され、もっともらしい値に丸められることはありません |
| 別のファイルに上書きしてしまうレポートや計測ファイル | ファイルがない場所にしか書きません。上書きには `--replace` が必要で、リンク越しには決して書きません |
| 安全でないメモリ使用 | `forbid(unsafe_code)` の安全な Rust のみ |

**防御していないこと：** エラーなく作られた結果は、検証された結果ではありません。ここには機械の限界を知るものはなく、プログラムは独立したレビューを受けていません。脆弱性は非公開で報告してください（[SECURITY.md](SECURITY.md) 参照）。

## 🔌 インターフェース

ライブラリは `eh3d_vision_studio`（`use eh3d_vision_studio::...`）、コマンドラインは `eh3d-vision` です。コマンドは次のとおりで、結果は標準出力に表示され、エラーは標準エラー出力に 0 以外の終了コードで報告されます。ネットワークアクセスはありません。

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

## ⚙️ 設定

調整される値はコードではなく、単純な `キー = 値` のファイルに書きます。プログラムを起動したフォルダー、またはプログラムの隣の `vision.cfg` です（変数 `EH3D_VISION_CONFIG` で別のファイルを指定できます）。ウィンドウの言語、マークが暗いか明るいか、しきい値、直径の範囲、マークの充填率と細長さ、スケールを決めるマークの大きさ、グリッドの列数と行数、位置合わせでサイズの変更を許すか、位置合わせが満たすべき限度、読み込む最大ファイルを設定します。`examples/vision.cfg` はすべてのキーを意味と起動時の値つきで一覧にしています。ファイルがなければその値が使われ、間違いのあるファイルは行番号つきで拒否されます。コマンドラインでは同じことをオプションで指定します（インターフェースを参照）。また、その他の調整値も保持します。信頼度（`refine_subpixel`、`full_contrast`）、照明のウィンドウ（`flatten_window`）、スケールと部品の大きさ（`mm_per_px`、`part_min_area`、`part_max_area`、`max_parts`）、十字の範囲（`cross_min_size`、`cross_max_size`、`cross_max_elongation`）、ターゲットの許容差（`pitch_tolerance`）、フォルダ内の画像の最大数（`batch_max_files`）です。 正方形の最小の充填率と、任意の角度の十字の最小スコアは `square_min_fill` と `cross_min_score` です。 レンズは `lens_k1` と `lens_k2`（`lens` が求める値）で、補正後にグリッドが格子から離れてよい最大値は `lens_max_rms` です。 `inspect` が読むパッケージの表は `package_table`、部品がパッケージから外れてよい最大値と回転の最大値は `inspect_tolerance` と `inspect_max_angle` です。 HTML レポートが画像に警告を出すスコアのしきい値は `report_low_score` です。 スケールを決めるターゲットのマーク間隔は `pitch_mm`、書き出すファイルが既存のファイルに上書きしてよいかは `replace_output`（既定は no）です。 `shots` の計画で隣り合う画像の重なりと部品の周りの余白は `shots_overlap` と `shots_margin` です。

## 🚀 クイックスタート

安定版の Rust ツールチェーン（1.88 以降）が必要です。EH3D-COMMON リポジトリをこのリポジトリの隣にクローンしておく必要があります。画像リーダーはそこからパスで取り込みます。

```bash
cargo build --release
cargo test
cargo run --release -- align examples/design.csv examples/measured.csv
cargo run --release --features gui --bin eh3d-vision-gui   # the window
cargo run --release --features gui --bin eh3d-vision-gui -- --shot screen.png --open examples/marks.pgm --open examples/design.csv   # a picture of the window
```

手動で開かずにウィンドウの画像を撮るには、`--shot FILE.png` を付けて実行します（`--size WxH`, `--theme forge|dark|light`, `--wait N`, `--lang CODE`, `--open FILE`, `--replace`, `--mark N`, `--align` も使えます）。指定のサイズで開き、PNG を保存し、設定は保持せず、自動で閉じます（終了コードは、ファイルを書き込めたら 0、書けなければ 1、引数が誤りなら 2）。 `--mark N` で N 番目（1 から）のマークがスケールを決め、`--align` で設計がマークに位置合わせされるため、画像には位置合わせの線が表示されます。設計が各マークを置く位置から、見つかったマークまでの線で、距離はミリメートルで表示されます（`vision.cfg` の `show_residuals` で無効にできます）。`examples/design_pair.csv` は `examples/marks.pgm` の 2 つのマーク用の設計です。

## 🧪 テスト

テストは 246 件あります（`cargo test`、CI では Windows と Linux で実行）。通常ケース、境界ケース、破損入力を確認し、空のファイル、ゴミデータ、途中で切れたファイルでもパニックしません。CI は `cargo fmt --check`、警告をエラーにした `cargo clippy`、`python -I tools/ci_validate.py` も実行します。

## 🏗️ ビルドとリリース

バージョンは `Cargo.toml` と `eh3d.project.json` にあり、各バージョンは [CHANGELOG.md](CHANGELOG.md) に項目があります。`build.bat` / `build.sh` でビルド、`test.bat` / `test.sh` でテスト、`run.bat` / `run.sh` でサンプル実行します。リリースは GitHub で公開されます。

### ダウンロード

各バージョンには、Windows（64 ビットと 32 ビット）、Linux（64 ビット）、macOS（Apple silicon と Intel）向けの、ウィンドウ付きのプログラムが `<名前>-<バージョン>-<システム>.zip`（Windows）または `.tar.gz` として含まれ、プログラム、ライセンス、README、アイコン、サンプルが入っています。ファイルは GitHub Actions でビルドされ、`SHA256SUMS` に記載され、EH3D の鍵で署名されています。ダウンロードを確認するには、隣にある `.sha256` ファイルとハッシュを比較し（`sha256sum -c`）、隣にある `.sig` ファイルで署名を確認します（`ssh-keygen -Y verify`）。

## 📂 リポジトリの構成

<p align="center">
  <img src="images/repository-structure.svg" alt="EH3D-VISION-STUDIO のリポジトリ構成" width="100%">
</p>

<details>
<summary>ツリーのテキスト版</summary>

```text
EH3D-VISION-STUDIO/
├── src/
│   ├── lib.rs                                   ライブラリ：各コマンドが使うもの
│   ├── main.rs                                  コマンドライン
│   ├── gui_main.rs                              独立したプログラムとしてのウィンドウ（gui 機能）
│   ├── cli_args.rs                              コマンドライン引数：小さなツールすべてで同じファイル
│   ├── align.rs                                 基板の回転・シフト・サイズと設計との対応付け
│   ├── batch.rs                                 フォルダ内の画像をまとめて処理し、CSV レポートを出力
│   ├── config.rs                                設定ファイル：調整されたすべての値
│   ├── cross.rs                                 十字形の基準マーク
│   ├── fiducial.rs                              画像の円形マーク：しきい値・領域・検査
│   ├── flatten.rs                               画像のムラのある照明を取り除く
│   ├── grid.rs                                  マークのグリッドから求めるレンズの歪み
│   ├── inspect.rs                               部品を表のパッケージ本体寸法と比較して判定する
│   ├── lens.rs                                  マークのグリッドから求めるレンズの 2 係数（合わなければ拒否）
│   ├── points.rs                                設計の点のファイル
│   ├── report.rs                                フォルダのレポートを 8 言語の単独 HTML ページとして出力
│   ├── limit.rs                                 サイズ上限付きのファイル読み込みと、拒否ごとのコード
│   ├── measure.rs                               部品をミリメートルで計測し、CSV と JSON を出力
│   ├── scale.rs                                 ピクセルからミリメートルへ、レンズの放射歪み
│   ├── square.rs                                四角いマークと任意の角度に回った十字
│   ├── tools.rs                                 サイドパネルの画像ツール：図形・部品・フォルダレポート・間隔によるスケール
│   ├── shot.rs                                  ウィンドウの画像モード（--shot）：画面なしで読み取り・検証される引数
│   ├── shots.rs                                 視野より大きな部品を覆うカメラ位置を、寸法だけから求める
│   ├── browse.rs                                ファイルを開くウィンドウのファイル一覧
│   ├── gui.rs                                   ウィンドウ（egui、gui 機能）：状態を描き、操作を伝える
│   ├── lang.rs                                  ウィンドウの文言（8言語）
│   ├── viewport.rs                              画像の拡大縮小と移動（単純な数値）
│   └── window.rs                                画面なしのウィンドウの状態：画像・マーク・スケール・レンズ・位置合わせ
├── tests/
│   ├── readme.rs                                READMEとrunスクリプトが約束する内容を実行するテスト
│   ├── strict.rs                                コマンドラインのテスト：知らないオプションと語
│   ├── read_limit.rs                            コマンドラインが読むファイルのサイズ上限のテスト
│   ├── exchange.rs                              画像を読んでファイルを書くコマンドのテスト
│   ├── shot_sample.rs                           サンプルが画像用ウィンドウで開くことのテスト
│   └── vision.rs                                ライブラリとコマンドラインのテスト
├── examples/
│   ├── design.csv                               試すためのサンプル
│   ├── design_pair.csv                          marks.pgm の 2 つのマーク用の設計
│   ├── marks.pgm                                試すためのサンプル
│   ├── measured.csv                             試すためのサンプル
│   ├── packages.csv                             inspect 用のパッケージ本体寸法の表
│   └── vision.cfg                               すべての値とその意味を記した設定ファイル
├── docs/
│   ├── architecture.json                        アーキテクチャ図のデータ
│   └── structure.json                           構成図のデータ
├── tools/                                       CI と README のチェック、図のツール
├── images/                                      バナーとアーキテクチャ・リポジトリ構成の図
├── .github/                                     CI とリリースのワークフロー、Issue と PR のテンプレート
├── eh3d.project.json                            マニフェスト：バージョン、役割、成熟度、ビルドコマンド
├── Cargo.toml Cargo.lock                        ワークスペースと固定された依存関係
├── rust-toolchain.toml                          固定されたRustツールチェーン
├── CHANGELOG.md                                 各バージョンの変更点
├── SECURITY.md SUPPORT.md                       脆弱性の報告方法と問い合わせ先
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   貢献方法、行動規範、ライセンス
```

</details>

## 🗺️ ロードマップと既知の制限

**現時点で分かっている制限**

* ウィンドウはまだ画面で確認していません。マークは位置で設計と対応付けるため、大きく回転した基板や欠けたマークは推測せずに拒否されます。
* 機械でも実際のファイルの集まりでも試しておらず、独立したレビューも受けていません。
* 読むのは画像ファイルでライブカメラではなく、見つけるのは丸いマーク、四角いマーク、十字形のマークだけで、テストが描いた画像で試しただけで、実際の基板の写真では試していません。部品の回転はその画素から長方形として測るため、長方形でない部品には長軸の回転が与えられます。

**次のステップ**

* 機械のライブカメラ（EH3D-NODE-BUNGARD-AUX）を読み取り、実際の基板の写真で試し、新しいマークの学習をウィンドウに加える。
* ウィンドウを画面で確認して仕上げる。
* 実際のファイルとスイートの他のプログラムで試す。

## 🌍 言語

README とテキストは、英語、スペイン語、フランス語、イタリア語、ドイツ語、中国語、日本語、ヨーロッパポルトガル語で提供されています。

## 🔗 関連プロジェクト

* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON)：共有クレート（画像リーダーを含む）
* [EH3D-PHOENIX-PNP](https://github.com/JuanenRac/EH3D-PHOENIX-PNP): ピック＆プレース実装
* [EH3D-ISOCAM](https://github.com/JuanenRac/EH3D-ISOCAM): 銅箔の絶縁ミリング
* [EH3D-PHOENIX-CNC](https://github.com/JuanenRac/EH3D-PHOENIX-CNC): 機械制御プログラム
* [EH3D-NODE-BUNGARD-AUX](https://github.com/JuanenRac/EH3D-NODE-BUNGARD-AUX): カメラ付きドリルヘッドのノード
* [EH3D-SUITE-LAUNCHER](https://github.com/JuanenRac/EH3D-SUITE-LAUNCHER): スイートのプログラムを見つけて起動するハブ

## 📚 ドキュメント

* [変更履歴](CHANGELOG.md)
* [アーキテクチャと構成のデータ](docs/architecture.json)：2 枚の図を描くためのデータ。

## 🤝 コントリビュートとコミュニティ

* プルリクエストの前に `cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test`、`python -I tools/ci_validate.py` を実行し、すべて通してください。[CONTRIBUTING.md](CONTRIBUTING.md) と[行動規範](CODE_OF_CONDUCT.md)を参照。
* README は 8 言語あります。文章の変更は 8 ファイルすべてで行い、2 枚の図は `tools/make_architecture.py` と `tools/make_structure.py` が `docs/architecture.json` と `docs/structure.json` から描きます。
* 脆弱性は非公開で報告してください：[SECURITY.md](SECURITY.md)。質問・提案・報告：electrohobby3d@gmail.com（[SUPPORT.md](SUPPORT.md)）。

## 👤 作者

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 ライセンス

ソフトウェア：GPL-3.0-or-later。[LICENSE](LICENSE) を参照。スイートのハードウェア設計には CERN OHL-S を使います。
