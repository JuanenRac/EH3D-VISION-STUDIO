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
  <img src="https://img.shields.io/badge/Version-0.1.4-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-functional-4CAF50.svg" alt="Maturity">
</p>

---

**正直さの確認 - 現在実際に動くもの：** Rust ライブラリとコマンドライン プログラム（`eh3d-vision`）で、コンピューター上で実行される 970 件のテストがあります：画像から丸い基準マークを見つけ、画素をミリメートルに変換し、基板を設計どおりの位置に合わせる回転と移動量を計算します。ウィンドウ（`--features gui` でビルドする `eh3d-vision-gui` プログラム）があり、画像を開き、マークを座標つきで表示し、スケールを設定し、グリッドからレンズの歪みを求め、設計と位置合わせして ISOCAM 用のオプションを書き出しますが、ウィンドウはまだ画面で確認していません。機械では試しておらず、実際のファイルの集まりでも確認していません。

---

> **安全とセキュリティの状況：** このプログラムは機械を制御せず、安全システムでもありません。出力は、機械で何かを実行する前に人が確認する必要があります。ここにあるものは実機で動かしたことがありません。


## 📋 ひと目で

| | |
|---|---|
| バージョン | `0.1.4` |
| 成熟度 | 機能する |
| テスト | 970 |
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
| テスト | 🟡 | 970 件のテストが通ります（`cargo test`）。通常ケース、境界ケース、破損入力に加え、ノイズ、誤った露出、無効または欠落したフレーム、マークに見えるほこり、途切れた画像ファイルを加え、再取得される基板上の模擬カメラを含みます。 |
| セキュリティ | 🟡 | 安全な Rust のみ（`forbid(unsafe_code)`；unsafe はカメラのバックエンドを専用機能でビルドしたときの小さな部分だけ）。受け取ったファイルでパニックせず、不正な値は理由付きのエラーになります。独立したレビューはありません。 |
| ウィンドウ | 🟠 | 作成済み（egui、`--features gui`、プログラム `eh3d-vision-gui`）：画像を開く／ドロップする、マークを座標つきで見る、大きさが既知のマークでスケールを設定する、グリッドからレンズの歪みを求める、CSV の設計と位置合わせする、ISOCAM 用オプションを書き出す。ロジックはテスト済みですが、まだビルドも画面での確認もしていません。 |
| ライブカメラ | 🟠 | カメラ層、校正ボード、基板の位置合わせは書かれ、模擬カメラと模擬ステージで試験済みです。Windows、V4L2、libcamera のバックエンドは骨組みで、実際のカメラ、レンズ、基板では何も動かしていません。 |
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

## 🖼️ スクリーンショット

<p align="center">
  <img src="images/screens/vision-studio-01-window.png" alt="サンプル画像とサンプル設計を開いたウィンドウ" width="100%">
</p>

<p align="center"><em>サンプル画像とサンプル設計を開いたウィンドウ。見つかった2つのマークがそれぞれ中心と番号つきで表示され、左側に検索の設定があります。</em></p>

## ✨ 機能

* **ライブラリとコマンドライン：** `eh3d-vision` の各コマンドはライブラリの関数なので、スイートの他のプログラムがプロセスを起動せずに呼び出せます。
* **厳密なコマンドライン：** 書き間違えたオプション（`--powr`）や余分な語は無視されず、それを名指しするエラーになります。そうでないと、プログラムは既定値で出力を作ってしまいます。
* **破損入力のテスト：** 空のファイル、ゴミデータ、巨大な数値、途中で切れたファイルも与え、どれもパニックしてはいけません。
* **推測しない：** 意味のない値は、どの値かと理由を示すエラーになり、見かけだけ正しい結果にはなりません。
* **読み込むファイルのサイズ上限：** コマンドラインは、画像や点のファイルが `vision.cfg` の `max_file_mib`（既定は 64 MiB）以下のときだけ読み込みます。それより大きいファイルはコード `file_too_large` とファイル名で、読めないファイルは `file_unreadable` で、テキストでない点のファイルは `file_not_text` で拒否され、何も読み込まれません。
* **既定のビルドに unsafe コードなし：** ライブラリとプログラムに `#![forbid(unsafe_code)]`。ライブカメラのバックエンドだけが、専用の機能でビルドしたときに小さな unsafe 部分（数回の呼び出しで、構造体のサイズはビルド時に検査）を持ちます。
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
* **1 種類または 2 種類の大きさのリングマーク：** `eh3d-vision rings 画像` は穴のある丸いマーク（ドーナツ）を見つけます。各マークは、中心、外径と内径、形のスコア、信頼度（リングが背景から目立たないときに下げたスコア、`full_contrast`）を持ち、リングが 2 種類の大きさのときは小か大かも示します（`--split PX` または `ring_split` で境目の直径を指定、なければ直径が `ring_size_gap` 以上の間隔で 2 群に分かれる所で決めます）。円盤、切れ目のあるリング、画像の端で切れたリング、丸くないもの、穴が小さすぎるか大きすぎるもの（`ring_min_ratio`、`ring_max_ratio`）は除かれます。ウィンドウにはその専用の節があり、外側と内側の円を大きさごとの色で描きます。
* **ピックした部品の保持位置：** `eh3d-vision offset 画像 --package 名前` は、下から見た部品の画像（部品は 1 つ）を `measure` と同じに測り、`inspect` と同じにパッケージ本体と大きさを比べ、ノズル軸からのずれ（`--nozzle-x`、`--nozzle-y` または `nozzle_x_mm`、`nozzle_y_mm`。画像の中心からのミリメートル、x は右、y は上）、本来の保持向きからの回転（`--nominal-angle`。縦横が同じ部品は 4 分の 1 回転以内、それ以外は半回転以内の近い方向）、そして部品をノズルの望む位置に戻す補正（同じ距離と逆向きの回転）を出します。CSV または JSON で、`--replace` なしには既存ファイルに上書きしません。部品の大きさが違う、`--max-offset`（`pick_max_offset_mm`）より遠くで保持されている、または `--max-turn`（`pick_max_turn_deg`）を超えて回っているときは、ファイルを書いた後に終了コード 1 になります。部品がない画像や複数ある画像は拒否されます（`no_parts`、`parts_count`）。画像上の幾何だけで、カメラ・ノズル・機械は使っていません。
* **トレイ用の補正表とそのウィンドウパネル：** `eh3d-vision offset 画像 --package 名前 --each` は画像内のすべての部品（トレイ）を一つずつ個別に扱い、列 `part`（部品の番号、1 から）に続けて `offset` の列を並べた表を、部品ごとに 1 行で書き出します。部品が 1 つでも不合格なら終了コードは 1 です。ウィンドウにも同じパネル（`nozzle_x_mm`、`nozzle_y_mm`、`nominal_angle_deg`、`pick_max_offset_mm`、`pick_max_turn_deg`）があり、開いている画像の表を計算し、各部品の補正をウィンドウの言語で一覧し、`<画像>-offsets.csv` として保存します。`nominal_angle_deg` は正しく保持された部品の回転で、`vision.cfg` にもあります。
* **鏡像のカメラとノズル軸：** `--mirror`（`pick_mirror`）は部品を左右反転して見るカメラ用で、各部品の x と回転の符号が変わります。`eh3d-vision nozzle 画像 画像... --package 名前` は同じノズルでピックした部品の画像（各画像に部品 1 つ、2 枚以上）を受け取り、`vision.cfg` 用の `nozzle_x_mm` と `nozzle_y_mm` の行（部品の平均位置。測定時に使っていた軸には依存しません）を書き出し、エラー出力に部品のばらつき（x と y の標準偏差、最も遠い部品、平均回転）を示します。
* **画像は測定に十分か：** `eh3d-vision quality 画像` は画像のシャープネス（ラプラシアンの分散）、コントラスト（グレーレベルの標準偏差）、平均グレーレベルを CSV または JSON で出力し、`flat`、`dark`、`bright`、または `--min-sharpness` が 0 より大きいときは `blurry` として拒否します（ファイル書き出し後に終了コード 1）。下限は `--min-contrast`、`--min-mean`、`--max-mean`、`--min-sharpness`、または `vision.cfg` のキーです。`quality_check = yes` のとき、`offset`、`nozzle`、ウィンドウは不良な画像（`poor_picture`）から部品を測る前に拒否し、ウィンドウには画像を確認して数値を表示するパネルがあります。値はカメラとレンズに依存するため設定ファイルにあり、シャープネスは最初はオフです。実際のカメラでは試していません。
* **2 つの係数を持つレンズ、合わなければ拒否：** `eh3d-vision lens 画像` はマークのグリッドから放射歪みの `k1` と `k2` を求め、`vision.cfg` 用に `lens_k1 = ...` と `lens_k2 = ...` の行を出力します。補正後もマークが格子から `lens_max_rms` ピクセルより離れる当てはめ（`lens_residual_too_large`）や、0.5 を超える係数（`lens_out_of_range`）は拒否され、`find --undistort` はそれらで位置を補正します。
* **部品ごとの判定：** `eh3d-vision inspect 画像 --package 名前 --table packages.csv` は画像の部品を計測し、それぞれをパッケージの本体寸法（CSV ファイルの `package`、`body_long_mm`、`body_short_mm`、必要なら `tolerance_mm`。`examples/packages.csv` にいくつか入っています）と比較して、`pass` か `fail` を理由（`length`、`width`、`angle`）とともに CSV または JSON で出し、部品が不合格なら終了コードは 1 です。許容差と最大の回転（`inspect_tolerance`、`inspect_max_angle`）は `vision.cfg` にあります。
* **フォルダの HTML レポート：** `eh3d-vision batch フォルダ --html report.html` は単独で完結する 1 ページ（スクリプト、リンク、他のファイルなし）を書き出します。画像の表、見つかった各マークに円を付けた小さな絵、警告（使用できなかった画像、マークがない画像、`report_low_score` を下回るスコアの画像）が入り、言語は `--lang` または `vision.cfg` に従います。
* **ウィンドウの新しいツール：** サイドパネルに、各マークの信頼度付きのサブピクセル中心、照明のウィンドウ、任意の角度の正方形と十字（画像上に描画）、ミリメートルの部品（`<画像>-parts.csv` として保存）、フォルダのレポート（スレッドで読み込み、進捗バーと中止ボタンつき。`eh3d-vision-report.html`。`replace_output` がなければ既存ファイルに上書きしません）、ターゲットの間隔からのスケールが加わり、文言は 8 言語、値は `vision.cfg` にあります。画面ではまだ確認していません。
* **ウィンドウのレンズ（2 つの係数）：** グリッドのボタンは `eh3d-vision lens` と同じく `k1` と `k2` を求めます（パネルの欄 `lens_max_rms` を超えると拒否）。ウィンドウは、求めたレンズがグリッドの格子を置く位置を描き、見つかったマークと比べられます（`show_lens_grid`）。グリッドなしで `vision.cfg` の `lens_k1` と `lens_k2` を使うことも、`<画像>-lens.cfg` として保存することもできます。
* **ウィンドウでの部品とパッケージの照合、大きな部品の撮影計画：** サイドパネルはパッケージ表（`package_table`）を読み、測定した部品をパッケージ `inspect_package` と照合して（部品ごとに緑か赤の行、不合格の理由つき。`inspect_tolerance` と `inspect_max_angle` はパネルの欄）、判定を `eh3d-vision inspect` と同じ `<画像>-inspection.csv` として保存します。また、視野より大きな部品に必要な撮影を計画し（視野はミリメートルにした画像、部品は `shots_part_width` x `shots_part_height` かパッケージの本体、`shots_overlap` と `shots_margin`）、カメラの移動先を一覧にして、`eh3d-vision shots` と同じ `<画像>-shots.csv` として保存します。コマンドラインは、`--package` も部品サイズも与えないとき、この 2 つの新しいキーを使います。
* **大きな部品の撮影計画：** `eh3d-vision shots --package 名前 --fov-w MM --fov-h MM`（または `--body-long` と `--body-short`）は、視野より大きな部品の全体が見えるカメラの位置を、部品の中心からのミリメートルで CSV に並べます。隣り合う画像は `shots_overlap` だけ重なり、`shots_margin` の余白を含みます。カメラは不要です。
* **相関によるマーク検出：** `eh3d-vision correlate 画像 --diameter PX` は、その直径の円板を画像の上でずらし、各位置で正規化相関を計算します（光が明るくなってもコントラストが下がっても変わりません）。そのため、ムラのある光でしきい値では見失うマークも見つかります。位置は 1 ピクセル未満の精度でスコア付きで、良い順に示され、より良いものから直径以内にあるものは出ません。直径（`correlate_diameter`、`correlate_min_score`）が必要で、しきい値による検索より遅いので、時間がかかりすぎる画像は拒否されます（`correlate_too_big`）。
* **遠近補正：** `eh3d-vision rectify 画像 --corners X1,Y1,...,X4,Y4 --output FILE.pgm` は、画像内の長方形の 4 つの角から、真上から見たように画像を描き直します（双線形サンプリングによるホモグラフィ）。凸四角形にならない角は拒否され（`bad_corners`）、最大の辺は `rectify_max_side` です。
* **パッドの列：** `eh3d-vision pads 画像 --mm-per-px V` は画像の部品をパッドとみなし、ピッチ（最良近似直線に沿った距離の中央値）、そのばらつき、列からのパッドのずれ、平均サイズを CSV または JSON で出します。判定は `pads_tolerance` に対して理由 `pitch`、`row`、`expected`（`--expect-pitch`）付きの `fail` で、終了コードは 1 です。
* **前と後：** `eh3d-vision diff 前 後` は同じ大きさの 2 枚の画像を比較します。グレー値が `diff_threshold` を超えて異なる画素、その割合、平均と最大の差、それらを囲む枠、`diff_min_area` 画素以上の領域を出します。`--mask FILE.pgm` は変化の画像を書き出し、`diff_max_changed` を超えて変わった場合の終了コードは 1 です。
* **部品がどちら向きか：** `eh3d-vision orientation 画像 --mm-per-px V` は部品（基板と異なる部分）とその上のマーク（ダイオードの帯、1 番ピンの点：内側で本体と異なる部分。基板と同じ色でも可）を見つけ、部品の中心からマークへの向き（x 軸から反時計回りの度数、y は上）、最も近い 4 分の 1 回転とそこからのずれ、`--expect-deg` があれば望む向きからのずれを出します。部品は（ファイルを書いた後に終了コード 1 で）`no_mark`、`centred`、`mark_big`、`wrong_way`、向きの指定がなければ `skewed` で拒否されます。
* **パッドのフットプリント：** `eh3d-vision footprint 画像 --pad-length MM --pad-width MM` はすべてのパッドを測り、ミリメートルの許容差（`--tolerance`、`footprint_tolerance_mm`）で設計と一つずつ照らして判定します。上の行から下へ、左から右へ、パッドごとに 1 行で、差と理由 `length`、`width` を出します。`--count` はパッドの数も確認し（`count`）、パッドが外れていれば終了コードは 1 です。
* **画像のどこが鮮明かの地図：** `eh3d-vision sharpmap 画像 --cells N` は画像を格子に切り、各セルの鮮明さ（ラプラシアンの分散）、中央値、最も弱いセル、x 方向と y 方向の鮮明さの傾き、弱いセル（中央値の `--min-share` 未満）を出します。ほとんどコントラストのないセルは `empty` で判定しません。`--heat ファイル.pgm` は地図の画像（ぼやけた所が暗い）を書き、弱いセルがあれば終了コードは 1 です（`uneven`）。
* **正方形の盤からのスケールとレポート：** `eh3d-vision board 画像 --pitch-mm D --square-mm S` は正方形を見つけ、盤の回転と各軸の隣との距離を求め、x 方向・y 方向・辺からの 1 ピクセルのミリメートル、ピクセルがどれだけ正方形か（`aspect`）、距離がどれだけ違うか（`spread`）を出します。`--report ファイル.txt` は人向けの説明と、`vision.cfg` 用の行 `mm_per_px = ...` を書きます。ピクセルが正方形でない（`aspect`）、距離が違う（`spread`）、辺とピッチが合わない（`disagree`）場合は拒否されます（終了コード 1）。
* **ウィンドウにパネルを 4 つ追加：** サイドパネルに、相関による検索（見つかった目印を位置とスコアつきで一覧）、見つかった 4 つの目印からの画像補正（`<画像>-rectified.pgm` として保存）、測定した部品からのパッド列の判定（`<画像>-pads.csv` として保存）、開いている画像と別の画像の比較（`<画像>-diff.csv` と `<画像>-changes.pgm` として保存）があります。値はパネルの欄（`correlate_diameter`、`correlate_min_score`、`rectify_max_side`、`pads_tolerance`、`diff_threshold`、`diff_min_area`、`diff_max_changed`）で、文言は 8 言語です。ロジックはテスト済みですが、ウィンドウは画面で確認していません。
* **機械のサーバー：** `eh3d-vision server health|config|discover|check|detect|send` は、スイートの共有クライアント（EH3D-COMMON の `eh3d-serverclient`）を通して、スイートの他のプログラムと同じキー（`server_address`、`server_token`、`server_machine`、`server_timeout_s`、`server_retries`、`server_mdns`）を使います。`config` はサーバーがその機械のために保管する `vision.cfg` の値（形式 `text/x-eh3d-vision-config` の最新のアーティファクト `vision-<machine>.cfg`。ファイルの行または JSON オブジェクトとして書かれています。全部か何もしないか。接続のキーはサーバーから受け取りません）を読み、ファイル用の行として書き出します。`send quality|offset|pads|diff|orientation|footprint|sharpmap|board` は、そのコマンドの JSON 結果をアーティファクト `vision-<kind>.json` としてサーバーに保管します（`POST /api/v1/artifacts`、機械と種類はパラメーターに入ります）。再試行と接続の状態は共有クライアントのもので（届いたかもしれない要求は 2 度送りません）、サーバーの拒否はコードにより 8 言語で表示でき、トークンは表示されず、ローカルでないアドレスへ平文 HTTP で送られるトークンには警告が出ます。サーバーには `vision` プラグインがあるため、結果はその規約でも保管できます（以下の項目）。メモリ上のサーバー（`server::SimServer`）とループバックのソケットに対してテストしており、実際のサーバーではありません。
* **サーバーの規約での結果:** `eh3d-vision server send fiducials|focus|blobs 画像` は、EH3D-SERVER の `vision` プラグインとカメラ越しに見るプログラムが共有する形で結果を保管します。形式 `application/json`、スキーマ `eh3d.vision.result/1` のファイル `vision-<種類>.json` で、画像のサイズと SHA-256、その種類の検出結果（`fiducials` `{x, y, radius_px, score}`、`focus` `{score, sharp}`、`blobs` `{x, y, area_px}`。`barcode` は検査はしますが作りません。ここにはコードを読むものがないためです）、単位（`px`）、証拠（`--evidence camera|simulator` がなければ `picture`。ファイルがカメラ由来かどうかはプログラムには分かりません）を持ちます。`server check ファイル.json` は文書が規約に沿っているかを答え、沿っていない部分を示します。沿っていない文書は送る前に拒否されます。ピント確認の点数は、鮮鋭度が `focus_reference` のとき 50 です。
* **サーバーの模擬ステーション:** `eh3d-vision server detect fiducials|barcode|focus|blobs 画像 [--seed N] [--artifact]` は、サーバーの `vision` プラグイン（`POST /api/v1/work/jobs`、開始して終了まで読み取り）に、画像のサイズとハッシュとシードから作り出す結果を求めます。毎回同じで（`evidence: simulator`）、ジョブのビューに入るか、アーティファクトとして保管されます。`station::simulate` はメモリ上のサーバーが応答に使うプラグインの移植で、ジョブ・結果・アーティファクトの流れ全体をネットワークなしでここでテストします。実際のサーバーに対しては実行していません。
* **ネットワーク上のサーバー検索（本物のスイッチ）:** `eh3d-vision server discover` は、ローカルネットワークで `_eh3d._tcp` に応答するサーバーの `ホスト:ポート` を一覧にします。`server_mdns` がオフ（既定）の間は何もしません。共有クライアントのリゾルバーを使うには `--features mdns` でビルドする必要があります（リリース版はそうなっています）。見つかるのは人が選ぶ候補で、ネットワーク上のどの機器でも応答できるため、自動では決して接続しません。
* **1つの見本から学習するマーク、大きく回転した基板、安全なサーバー一覧：** `eh3d-vision train 画像 --rect X,Y,W,H --output ファイル` はマークの周りを切り出した長方形の平均をマークのファイルとして保存し（`--replace` がない限り既存のファイルには書き込みません）、`eh3d-vision locate --mark ファイル 画像` は相関で再び見つけます。必要なら一定の刻みで回転させ、位置・角度・スコアを返します（`vision.cfg` の `train_*` キー）。`eh3d-vision pair 設計.csv 測定.csv --any-turn` は基板の回転に関係なく、欠けたマークや余分なマークがあってもマークを設計と対応付け、二通りが同じくらい説明できる場合は拒否します（`pair_*` キー）。`discovery` はネットワークで応答したものを、形式の正しい少数のローカル候補にし（`find_server_*` キー）、`server discover` とウィンドウはその候補だけを一覧にします。コマンドは `vision.cfg` か `--lang` の言語で答え、`--json` で JSON を出し、終了コード 0（完了）、1（実行できなかった）、2（実行したが検証に合格しなかった：見本が一致しない、隣と区別できないマーク、マークが見つからない、対応付けできないマーク）で終わります。すべて描いた画像とそれを描いた真値で試しています。見て、動かし、もう一度見るループは今も位置で対応付けており、ウィンドウはまだマークを学習しません。
* **モーションJPEGフレームの読み取り、印刷する点のカード、画像の照明、テープの穴：** `eh3d-vision jpeg フレーム` はベースラインJPEGのフレームを `mjpeg_max_bytes` と `mjpeg_max_pixels` の範囲で明るさまで読み取ります。途切れたフレーム、あり得ない場所のマーカー、あり得ないテーブル、何十億画素ものサイズは、変わらないコード（`jpeg_truncated`、`jpeg_bad_marker`、`jpeg_bad_table`、`jpeg_bad_size`、`jpeg_unsupported`、`jpeg_bad_data`）で拒否され、パニックにはなりません。`eh3d-vision calib-card` は 100 % で印刷する丸い点のカードをミリメートル単位で描き、毎回同じで、ページに収まらなければ拒否します（`card_*` キー）。`eh3d-vision light-check 画像` は画像の照明が良好か（平均、白飛びの割合、ゾーン、傾き。`light_*` キー）と、良くない理由を示し、良くなければ終了コード 2 です。`eh3d-vision tape-locate 画像` は部品テープの穴、テープのピッチとの比較、ポケットを見つけます（`tape_*` キー）。すべて固定シードで描いた画像とフレームで試しており、カメラは使わず、カードは印刷せず、テープは撮影していません。
* **サブピクセルのエッジと不確かさ:** `eh3d-vision edge 画像 --from X,Y --to X,Y` は、エッジをまたぐ線に沿って画像を読み（双一次、`edge_step_px` 間隔）、線の両端を 2 つの平坦部とみなして、プロファイルが両者の中間レベルを横切る位置を求め、線に沿った位置と画像内の位置、コントラスト、遷移の幅、不確かさ（1 標準偏差：平坦部のノイズに 8 ビットの丸めを加えたものを、交差点の傾きで割ったもの）を出します。不確かさが `edge_max_uncertainty_px` を超えると終了コードは 1 で、エッジのない線は `edge_flat` です。1 ピクセルより鋭いエッジは最大 0.05 px 傾くことがあります。
* **ヒストグラムと自動しきい値:** `eh3d-vision histogram 画像 --bins N` は、グレーレベルを 8〜256 のグループにして、大津（マーク検索が使うもの）、isodata、三角形法の切れ目、大津の 2 グループが説明する分散の割合、黒と白のピクセルの割合、3 つの切れ目が一致するか（8 レベル以内）を出します。一致しない場合は、照明か `vision.cfg` の `threshold` を見直してください。
* **許容マスクによる基準画像との照合:** `eh3d-vision refcheck 基準 画像 --tolerance マスク.pgm` は、同じサイズの 2 枚の画像を、許容差を場所ごとに変えて比較します。マスクの画素が 0 なら一般の `diff_threshold` のまま、1〜254 ならその分のグレーレベルだけ許容を増やし、255 は見ません。許容範囲外の画素、超えた量、領域の数（`diff_min_area`）と場所を出し、`--map ファイル.pgm` はそれを画像として書き出します。`diff_max_changed` を超えると終了コードは 1 です。
* **レンズの残差（マークごと）:** `eh3d-vision lens 画像 --residuals` は、格子の各マークについて、補正前と補正後に本来の位置からどれだけずれているか、最も遠いマーク、中心からの距離の 3 つの帯、外側の帯と内側の帯の比を書き出します。比が 1 をはるかに超える場合、2 つの放射項ではそのレンズに足りない（またはターゲットが平らでない）ことを示します。
* **ウィンドウの追加機能:** サイドパネルには、サーバーの設定（`eh3d-view` のスイート共通パネル：アドレス、トークン、機械、再試行、ネットワーク検索のスイッチ、ログインとセッション）、隅の接続状態、サーバー検索（スイッチがオンのときだけ、別スレッドで、選べる候補付き）、3 つの切れ目付きのヒストグラム、4 つの数値で決める線に沿ったエッジ、許容マスク付きの基準画像との照合、そして各パネルの見出しの下に畳まれたヘルプ行（8 言語）があります。ウィンドウはまだ画面で確認していません。
* **固定シードのテスト：** `tests/fuzz.rs` は、時計を読まない生成器で、任意の大きさの画像、任意の位置の角、任意の大きさのパッド、任意の位置の正方形、壊れた設定ファイル、JSON、Base64、HTTP 応答を与えます。パニックしてはならず、出てくるものは意味を持たなければなりません。ウィンドウのパネルも同様にランダムな画像を通します。 `tests/fuzz_camera.rs` は、任意の画像の任意の線に沿ったエッジ、ヒストグラム、任意のマスクによる基準画像との照合、任意のマークの残差、そして切り詰め・改変・でたらめな内容にした規約の文書について、同じことを行います。
* **トレイトの背後にあるライブカメラ（模擬カメラでのみ試験済み、`src/capture/`）：** カメラは名前と変わらないキー（`vendor:product:serial@port`）で一覧され、形式（YUYV、モーションJPEG、RGB、グレー）で開かれ、カウンタと元のバイト列付きのフレームを返します。13個のプロパティにはそれぞれ範囲があり、無い場合はカメラがそう答えます。USBバスの負荷（`capture_usb_mbps`）、黒・固着・欠落・途切れたフレーム、照明のちらつきを検出し、新しいフレームは秒数ではなく枚数で待ち、ログは関数に渡されます。模擬カメラはわざと故障します。Windows、V4L2、libcamera のバックエンドは機能 `mfoundation`、`v4l2`、`libcamera`（既定では無効）の背後にある骨組みで、実機では試していません。モーションJPEGのフレームは検査しますが、まだデコードしません。
* **番号付きマーカーの校正ボード（`src/charuco/`）：** `eh3d-vision charuco card` は白いマスにマーカーを持つチェス盤をミリメートル単位で描き、100 % で印刷できます。`eh3d-vision charuco calibrate IMAGE` はマーカーを読み、どの回転でも角を画素の小数精度で見つけ、画素からミリメートルへの変換を3通り（ホモグラフィ、アフィン、相似）で当てはめます。角に残る誤差が大きいとき（`calib_max_residual_px`）、方法ごとの角の位置が離れすぎるとき（`calib_max_method_gap`）、または1行の両端の外側の角の間を定規で測った距離（`calib_check_mm`）と合わないときは拒否されます。定規の値が無い場合は `calib_allow_unchecked` で許可しない限り使われません。唯一のオフセットは `camera_offset_x_mm`、`camera_offset_y_mm`、`camera_offset_deg` のもので、模擬フレームから作った校正は実際のカメラには使えません。
* **見て、動かし、また見て基板を正しい位置に置く（`src/boardaim.rs`）：** 変わらない姿勢からフィデューシャルを見て、回転とずれを求め、ステージに逆向きの移動を指示します（1回あたり `align_max_step_mm` と `align_max_step_deg` を、合計で `align_max_total_mm` を超えません）。移動後に撮られたフレームを待ってからもう一度見て、基板が `align_tolerance_mm` と `align_tolerance_deg` の内側に入るまで続けます。何かおかしいときは最も安全な状態（通常のステップ、半分のステップ、保持、中止）の方向にしか進みません。誤差が増えるとステップを半分にし、さらに増えれば止め、マークが無い、黒や同じフレーム、上限到達、ステージの失敗でも移動を止めます。模擬ステージでのみ実行しています。

## 🔒 セキュリティと安全

| 脅威 | 対策 |
|---|---|
| 破損・途中切れ・空のファイル | パニックなし：テストは有効な入力をあらゆる位置で切り、ゴミデータも与えます |
| 意味のない数値（負、無限、極端に大きい） | 値を示すエラーで拒否され、もっともらしい値に丸められることはありません |
| 別のファイルに上書きしてしまうレポートや計測ファイル | ファイルがない場所にしか書きません。上書きには `--replace` が必要で、リンク越しには決して書きません |
| 安全でないメモリ使用 | `forbid(unsafe_code)` の安全な Rust のみ；unsafe はカメラのバックエンドを専用機能でビルドしたときの小さな部分だけ |
| トークンが出てはならない所に表示・送信される | トークンは表示されず、結果にも入りません。サーバーが送る値は全部か何もかで、接続のキー（`server_*`）はサーバーから受け取りません。ローカルでないアドレスへ平文 HTTP で送られるトークンには警告が出ます |

**防御していないこと：** エラーなく作られた結果は、検証された結果ではありません。ここには機械の限界を知るものはなく、プログラムは独立したレビューを受けていません。脆弱性は非公開で報告してください（[SECURITY.md](SECURITY.md) 参照）。

## 🔌 インターフェース

ライブラリは `eh3d_vision_studio`（`use eh3d_vision_studio::...`）、コマンドラインは `eh3d-vision` です。コマンドは次のとおりで、結果は標準出力に表示され、エラーは標準エラー出力に 0 以外の終了コードで報告されます。ネットワークを使うのは `server` コマンドだけです（`vision.cfg` に書かれた機械のサーバーへの平文 HTTP）。ほかのコマンドはすべてローカルのファイルで動きます。

```text
eh3d-vision find IMAGE [--light] [--min-diam PX] [--max-diam PX] [--known-mm D] [--refine] [--undistort] [--flatten PX]   the round marks found (--known-mm gives millimetres, --refine the centre to a fraction of a pixel with a confidence, --undistort straightens the places with lens_k1 and lens_k2 of vision.cfg, --flatten takes uneven light out first)
eh3d-vision align DESIGN.csv MEASURED.csv [--scale] [--pair-nearest] [--isocam-args] [--max-rms 0.1] [--max-stretch 0.01]
eh3d-vision crosses IMAGE [--turned] [--light] [--min-size PX] [--max-size PX] [--flatten PX]   the cross-shaped marks found (plus signs; with --turned the arms may have any angle and the angle is printed)
eh3d-vision inspect IMAGE --package NAME [--table FILE] [--mm-per-px V] [--tolerance MM] [--max-angle DEG] [--light] [--flatten PX] [--format csv|json] [--output FILE] [--replace]   each part of a picture compared with the body of its package from a table (CSV with package, body_long_mm, body_short_mm), one verdict per part; exit 1 when a part fails
eh3d-vision shots (--package NAME [--table FILE] | --body-long MM --body-short MM) --fov-w MM --fov-h MM [--overlap F] [--margin MM] [--output FILE] [--replace]   the places of the camera that cover a part bigger than its field (millimetres from the centre of the part), row after row, as CSV; no camera is needed
eh3d-vision lens IMAGE [--columns C] [--rows R] [--max-rms PX] [--light] [--flatten PX] [--residuals] [--format csv|json] [--output FILE] [--replace]   the two radial factors of the lens from a grid of marks (columns and rows as grid_columns and grid_rows), refused when the marks stay farther than --max-rms pixels from their lattice; --residuals writes, mark by mark, how far each lies from its place before and after the correction, the farthest mark, three bands of distance from the centre and the ratio of the outer band to the inner one (a ratio well above 1: two radial terms are not enough)
eh3d-vision squares IMAGE [--light] [--min-size PX] [--max-size PX] [--flatten PX]   the square marks found, turned by any angle (centre, side and angle)
eh3d-vision rings IMAGE [--light] [--min-size PX] [--max-size PX] [--split PX] [--flatten PX]   the ring marks found (donuts: a round mark with a hole), with the outer and inner diameter, the size class when the rings are of two sizes (--split gives the diameter that tells small from large, else it is found from the diameters), the score of the shape and a confidence
eh3d-vision offset IMAGE --package NAME [--table FILE] [--mm-per-px V] [--nozzle-x MM] [--nozzle-y MM] [--nominal-angle DEG] [--tolerance MM] [--max-offset MM] [--max-turn DEG] [--mirror] [--each] [--light] [--flatten PX] [--format csv|json] [--output FILE] [--replace]   where a picked part is held, from a picture of it taken from below (one part in it): its offset from the axis of the nozzle and its turn from the way it should be held, the correction that puts it right, its size against the body of the package; --mirror when the camera sees the part turned over left to right; --each takes every part of the picture (a tray) and writes a table of corrections, one row for each; exit 1 when a part is the wrong size, too far off or turned too much
eh3d-vision nozzle IMAGE IMAGE... --package NAME [--table FILE] [--mm-per-px V] [--mirror] [--light] [--flatten PX] [--output FILE] [--replace]   where the axis of the nozzle is, from pictures of parts picked with it (one part in each, at least two pictures): the lines nozzle_x_mm and nozzle_y_mm for vision.cfg, with how spread the parts are
eh3d-vision quality IMAGE [--min-contrast V] [--min-mean V] [--max-mean V] [--min-sharpness V] [--format csv|json] [--output FILE] [--replace]   whether a picture is good enough to measure from: its sharpness, contrast and mean grey level, and why it is refused (flat, dark, bright or blurry); exit 1 when it is refused
eh3d-vision calibrate IMAGE --pitch-mm D [--light] [--tolerance 0.05] [--flatten PX]   the millimetres of a pixel from a target whose neighbouring marks are D millimetres apart
eh3d-vision measure IMAGE [--mm-per-px V] [--light] [--flatten PX] [--format csv|json] [--output FILE] [--replace]   the parts of a picture: centre, turn, length, width and area in millimetres, as a CSV or JSON file
eh3d-vision batch FOLDER [--light] [--flatten PX] [--report FILE.csv] [--html FILE.html] [--lang CODE] [--replace]   every PNG and PGM picture of a folder: marks, scores and diameters, or the code of what went wrong, one line each; --html writes a page that stands alone, with a drawing of each picture and the warnings, in the language of --lang or of vision.cfg
eh3d-vision correlate IMAGE [--diameter PX] [--min-score S] [--light] [--flatten PX]   the round marks found by sliding a disc of that diameter over the picture and working out the correlation at every place (a mark that the search by threshold loses on uneven light), with the place to a fraction of a pixel and the score
eh3d-vision rectify IMAGE --corners X1,Y1,X2,Y2,X3,Y3,X4,Y4 --output FILE.pgm [--size WxH] [--fill V] [--replace]   the picture drawn again as seen from straight above, from the four corners of a rectangle in it (top left, top right, bottom right, bottom left, in pixels)
eh3d-vision pads IMAGE [--mm-per-px V] [--expect-pitch MM] [--tolerance F] [--light] [--flatten PX] [--format csv|json] [--output FILE] [--replace]   a row of pads: the pitch, how much it varies, how far the pads lie from the row and their size; exit 1 when the pitch varies, a pad is off the row or the pitch is not the one asked for
eh3d-vision diff BEFORE AFTER [--threshold V] [--min-area PX] [--max-changed F] [--mask FILE.pgm] [--format csv|json] [--output FILE] [--replace]   two pictures of the same size compared: how many pixels changed, how much, where and in how many regions; exit 1 when more than --max-changed of the pixels changed
eh3d-vision orientation IMAGE [--expect-deg D] [--tolerance DEG] [--body-contrast V] [--mark-contrast V] [--min-mark-px N] [--mm-per-px V] [--format csv|json] [--output FILE] [--replace]   which way a part with a mark on it (the band of a diode, the dot of pin 1) is held: the direction from the middle of the part to the mark, the quarter of a turn it is nearest to and how far from it, and how far from the wanted direction; exit 1 when there is no mark, it is in the middle, or it points off the wanted direction (or off a quarter of a turn when none is asked for)
eh3d-vision footprint IMAGE --pad-length MM --pad-width MM [--tolerance MM] [--count N] [--mm-per-px V] [--light] [--flatten PX] [--format csv|json] [--output FILE] [--replace]   each pad of a picture judged on its own against the size of the design, with a tolerance in millimetres: one line per pad with the differences; exit 1 when a pad is off or the count is not the one asked for
eh3d-vision sharpmap IMAGE [--cells N] [--min-share F] [--heat FILE.pgm] [--format csv|json] [--output FILE] [--replace]   a map of where the picture is sharp: a grid of cells with the sharpness of each, the weak ones, the tilt along x and y and (with --heat) a picture of the map; exit 1 when a cell is much less sharp than the median
eh3d-vision board IMAGE [--square-mm D] [--pitch-mm D] [--max-aspect F] [--max-spread F] [--light] [--flatten PX] [--format csv|json] [--output FILE] [--report FILE.txt] [--replace]   the millimetres of a pixel from a board of squares (D is the side or the distance between centres), along x, along y and from the side, with a report; exit 1 when the pixels are not square, the distances between neighbours differ or the side and the pitch do not agree
eh3d-vision edge IMAGE --from X,Y --to X,Y [--step PX] [--max-uncertainty PX] [--flatten PX] [--format csv|json] [--output FILE] [--replace]   where an edge is, to a fraction of a pixel, along a line laid across it (from a point on one side to a point on the other, both inside the picture), with the uncertainty of the position (one standard deviation, from the noise of the two flat grounds and the slope of the edge), the contrast and the width of the transition; exit 1 when the uncertainty is above the limit
eh3d-vision histogram IMAGE [--bins N] [--flatten PX] [--format csv|json] [--output FILE] [--replace]   the histogram of the grey levels in 8 to 256 groups and the thresholds that Otsu, isodata and the triangle method pick, how much of the variance two groups explain and whether the three agree (the value for `threshold` of vision.cfg when the automatic one is not good)
eh3d-vision refcheck REFERENCE PICTURE [--tolerance MASK.pgm] [--threshold V] [--min-area PX] [--max-changed F] [--map FILE.pgm] [--format csv|json] [--output FILE] [--replace]   a picture compared with a reference of the same size through a mask of tolerance (a mask pixel at 0 keeps the general threshold, 1 to 254 allows that many levels more, 255 is not looked at): the pixels outside the tolerance, how much they pass it, in how many regions and where; --map writes them as a picture; exit 1 when the share is above the limit
eh3d-vision charuco card [--lang CODE] [--output FILE.svg] [--replace]   the board of calibration (squares and markers, calib_squares_x, calib_squares_y, calib_square_mm and calib_marker_seed of vision.cfg) as a drawing in millimetres to print at 100 %, with the guide in the language
eh3d-vision charuco calibrate IMAGE [--check-mm D] [--allow-unchecked] [--output FILE.cal] [--replace]   the calibration of the camera from a picture of that board: the markers are read, the corners found, the map from pixels to millimetres fitted three ways and refused when the residual, the gap between the methods or the distance D measured with a ruler between the two outer corners of a row (calib_check_mm) do not hold; without D the calibration is refused unless --allow-unchecked or calib_allow_unchecked
eh3d-vision train IMAGE --rect X,Y,W,H [--rect X,Y,W,H ...] --output FILE [--replace|--overwrite|--create] [--flatten PX] [--json]   a mark that is none of the known shapes, trained from one or more rectangles cut around it (all of the same size, from train_min_side to train_max_side): their average is kept in a file of a mark (the first line is EH3DMARK 1); a file that exists is never written over unless --replace (--overwrite says the same, --create asks for a new file, the default); exit 2 when the examples do not look like each other, have no contrast, or the mark looks like itself a few pixels away (it could not be told from its neighbours)
eh3d-vision locate --mark FILE IMAGE [--turn DEG] [--min-score S] [--flatten PX] [--json]   the places of the picture that look like a trained mark, with the angle and the score of each (--turn lets the search turn the mark up to DEG each way, 180 is every angle; train_turn_max_deg of vision.cfg when it is not given); exit 2 when the mark is not found, is ambiguous or is found more times than max_parts
eh3d-vision pair DESIGN.csv MEASURED.csv [--any-turn] [--max-turn DEG] [--tolerance MM] [--min-marks N] [--json]   which mark found goes with which mark of the design: by place by default (the same number of marks, the board nearly where it should be), or with --any-turn whatever the turn of the board, up to --max-turn (pair_max_turn_deg of vision.cfg), with a mark missing or an extra one told apart, and the turn, the shift and what is left over; exit 2 when the pairing is ambiguous (a design that looks the same when turned: narrow --max-turn), none is found or fewer marks than --min-marks are paired
eh3d-vision light-check IMAGE [--cells N] [--max-saturated F] [--max-uneven F] [--max-tilt F] [--format text|json|csv] [--output FILE] [--replace]   whether the light of a picture is good to measure from: its mean, the share burnt out, how much the zones differ and the tilt from one side to the other, and why not when it is not (light_* of vision.cfg); exit 2 when the light is not good
eh3d-vision tape-locate IMAGE [--mm-per-px V] [--light] [--pitch-mm MM] [--hole-mm MM] [--format text|json] [--output FILE] [--replace]   the holes along the edge of a tape of parts, the pitch measured against the pitch of the tape, how far the holes lie from their line, the pockets and the one nearest to the centre of the picture, where a part would be picked (tape_* of vision.cfg); exit 2 when there are too few holes, one is missing or off the line, or the pitch or the size is not right
eh3d-vision calib-card [--columns N] [--rows N] [--pitch-mm MM] [--lang CODE] [--output FILE.svg] [--replace]   the card of calibration of round dots (card_* of vision.cfg) as a drawing in millimetres to print at 100 %, with the larger dot at the top left and a bar to measure; a card that does not fit its page is refused, never drawn smaller
eh3d-vision jpeg FRAME [--output FILE.pgm] [--replace] [--json]   the brightness of the picture of a baseline JPEG frame, read within the limits mjpeg_max_bytes and mjpeg_max_pixels; a frame that is cut or wrong is refused with a code that does not change (jpeg_truncated, jpeg_bad_marker, jpeg_bad_table, jpeg_bad_size, jpeg_unsupported, jpeg_bad_data)
eh3d-vision server health|config|discover|check|detect|send KIND ... [--address HOST:PORT] [--machine ID]   the EH3D server of the machine (server_address, server_token, server_machine and server_mdns of vision.cfg): health asks whether it answers, config reads the values of the configuration file that the server keeps for the machine (written as lines for vision.cfg with --output, never over a file without --replace), discover lists the servers that answer on the local network (only with server_mdns = on and a program built with the feature mdns; they are candidates for the person to choose, never connected to by themselves), check FILE tells whether a result follows the convention of the server (schema, kind, picture, findings, units, evidence), detect fiducials|barcode|focus|blobs IMAGE [--seed N] [--artifact] asks the simulated station of the server for a result about the picture (evidence simulator), send quality|offset|pads|diff|orientation|footprint|sharpmap|board takes the arguments of the command of that name and keeps its result as JSON on the server, and send fiducials|focus|blobs IMAGE [--evidence camera|picture|simulator] [--max-findings N] keeps the result in the convention of the server with its schema
eh3d-vision-gui [IMAGE] [DESIGN.csv]   the window (a build with --features gui); on Windows it opens with no console
```

コマンド `train`、`locate`、`pair`、`light-check`、`tape-locate` は、完了すれば終了コード 0、実行できなければ 1（誤ったオプション、読み書きできないファイル、超過した上限）、実行したが検証に合格しなければ 2 で終わります。

## ⚙️ 設定

調整される値はコードではなく、単純な `キー = 値` のファイルに書きます。プログラムを起動したフォルダー、またはプログラムの隣の `vision.cfg` です（変数 `EH3D_VISION_CONFIG` で別のファイルを指定できます）。ウィンドウの言語、マークが暗いか明るいか、しきい値、直径の範囲、マークの充填率と細長さ、スケールを決めるマークの大きさ、グリッドの列数と行数、位置合わせでサイズの変更を許すか、位置合わせが満たすべき限度、読み込む最大ファイルを設定します。`examples/vision.cfg` はすべてのキーを意味と起動時の値つきで一覧にしています。ファイルがなければその値が使われ、間違いのあるファイルは行番号つきで拒否されます。コマンドラインでは同じことをオプションで指定します（インターフェースを参照）。また、その他の調整値も保持します。信頼度（`refine_subpixel`、`full_contrast`）、照明のウィンドウ（`flatten_window`）、スケールと部品の大きさ（`mm_per_px`、`part_min_area`、`part_max_area`、`max_parts`）、十字の範囲（`cross_min_size`、`cross_max_size`、`cross_max_elongation`）、ターゲットの許容差（`pitch_tolerance`）、フォルダ内の画像の最大数（`batch_max_files`）です。 正方形の最小の充填率と、任意の角度の十字の最小スコアは `square_min_fill` と `cross_min_score` です。 正方形の辺の範囲と、その枠が幅よりどれだけ長くてよいかは、十字のものとは別に `square_min_size`、`square_max_size`、`square_max_elongation` です。 レンズは `lens_k1` と `lens_k2`（`lens` が求める値）で、補正後にグリッドが格子から離れてよい最大値は `lens_max_rms` です。 `inspect` が読むパッケージの表は `package_table`、部品がパッケージから外れてよい最大値と回転の最大値は `inspect_tolerance` と `inspect_max_angle` です。 HTML レポートが画像に警告を出すスコアのしきい値は `report_low_score` です。 スケールを決めるターゲットのマーク間隔は `pitch_mm`、書き出すファイルが既存のファイルに上書きしてよいかは `replace_output`（既定は no）です。 `shots` の計画で隣り合う画像の重なりと部品の周りの余白は `shots_overlap` と `shots_margin` です。 ウィンドウがレンズの格子を描くかどうかは `show_lens_grid` です。 `inspect` とウィンドウが部品を照合するパッケージは `inspect_package`、`shots` とウィンドウが撮影を計画する部品のサイズは `shots_part_width` と `shots_part_height` です（0 と 0 ならパッケージの本体）。 リングマークには、外径の範囲（`ring_min_diameter`、`ring_max_diameter`）、内径と外径の比の範囲（`ring_min_ratio`、`ring_max_ratio`）、帯の最小充填率（`ring_min_fill`）、細長さ（`ring_max_elongation`）、2 種類の大きさを分ける直径（`ring_split`、0 で自動）、大きさの間隔（`ring_size_gap`）があります。 ピックした部品の画像でのノズル軸は `nozzle_x_mm` と `nozzle_y_mm`、保持してよい最大距離と最大回転は `pick_max_offset_mm` と `pick_max_turn_deg` です（0 は確認しない）。 正しく保持された部品の回転は `nominal_angle_deg`、カメラが部品を鏡像で見るかどうかは `pick_mirror`、画像のチェックは `quality_check` と下限 `quality_min_contrast`、`quality_min_mean`、`quality_max_mean`、`quality_min_sharpness`（0：確認しない）です。 サーバーは `server_address`、`server_token`、`server_machine`、`server_timeout_s`、`server_retries` です（アドレスが空ならサーバーなし）。相関による検索は `correlate_diameter` と `correlate_min_score`、補正した画像の最大の辺は `rectify_max_side`、パッド列の許容差は `pads_tolerance`、2 枚の画像の比較は `diff_threshold`、`diff_min_area`、`diff_max_changed` です。サーバーをローカルネットワークで探すかどうかは `server_mdns` です（既定はオフ）。マーク付き部品の読み取りは `orientation_body_contrast`、`orientation_mark_contrast`、`orientation_min_mark_px`、`orientation_tolerance_deg`、パッドの許容差は `footprint_tolerance_mm`、鮮明さの地図は `sharpmap_cells`、`sharpmap_min_share`、`sharpmap_min_contrast`、盤からのスケールは `board_square_mm`（中心間の距離は `pitch_mm`）、`board_max_aspect`、`board_max_spread` です。トークンを含む `vision.cfg` は非公開にしてください。 ピント確認の結果で 50 点にあたる鮮鋭度は `focus_reference`、エッジの線に沿った間隔と位置の不確かさの上限は `edge_step_px` と `edge_max_uncertainty_px` です。ライブカメラ、校正ボード、基板の位置合わせには専用のキー（`capture_*`、`calib_*`、`camera_offset_*`、`align_*`）があります。どれにも範囲があり、範囲外の値は拒否され、キーが関わるエラーはそのキーを名指しします。 モーションJPEGフレームの読み取り、点のカード、照明のチェック、テープの穴の検索には、それぞれ専用のキー（`mjpeg_*`、`card_*`、`light_*`、`tape_*`）があります。どれにも範囲があり、範囲外の値はキー名を示して拒否されます。

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

テストは 970 件あります（`cargo test`、CI では Windows と Linux で実行）。通常ケース、境界ケース、破損入力を確認し、空のファイル、ゴミデータ、途中で切れたファイルでもパニックしません。CI は `cargo fmt --check`、警告をエラーにした `cargo clippy`、`python -I tools/ci_validate.py` も実行します。

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
│   ├── board.rs                                 正方形の盤からのスケール：x 方向・y 方向・辺から、レポートつき
│   ├── boardaim.rs                              設計どおりに基板を置く：見て、動かし、また見る。常に最も安全な状態へ
│   ├── capture/                                 ライブカメラ：トレイト、形式、プロパティ、USB帯域、ストリームの状態、模擬カメラ
│   ├── charuco/                                 番号付きマーカーの校正ボード：角、校正、ファイル、印刷用カード
│   ├── config.rs                                設定ファイル：調整されたすべての値
│   ├── correlate.rs                             目的の直径の円板との相関で見つける丸いマーク
│   ├── diff.rs                                  同じ場所の2枚の画像の比較：何が、どこで、どれだけ変わったか
│   ├── discovery.rs                             ネットワークの応答を安全な候補の短い一覧にする処理と、台本どおりに答えるリゾルバー
│   ├── cross.rs                                 十字形の基準マーク
│   ├── edge.rs                                  線に沿ったサブピクセルのエッジと不確かさ
│   ├── fiducial.rs                              画像の円形マーク：しきい値・領域・検査
│   ├── flatten.rs                               画像のムラのある照明を取り除く
│   ├── footprint.rs                             各パッドを設計寸法と照らして判定（許容差はミリメートル）
│   ├── grid.rs                                  マークのグリッドから求めるレンズの歪み
│   ├── inspect.rs                               部品を表のパッケージ本体寸法と比較して判定する
│   ├── job.rs                                   スレッドで読み込むフォルダレポート：進捗表示と中止つき
│   ├── lens.rs                                  マークのグリッドから求めるレンズの 2 係数（合わなければ拒否）
│   ├── points.rs                                設計の点のファイル
│   ├── quality.rs                               測定に使える画像かどうか：シャープネス・コントラスト・平均レベル
│   ├── reference.rs                             許容マスクによる基準画像との照合
│   ├── histogram.rs                             ヒストグラムと大津・isodata・三角形法の切れ目
│   ├── report.rs                                フォルダのレポートを 8 言語の単独 HTML ページとして出力
│   ├── ring.rs                                  リングマーク：穴のある丸いマーク、1 種または 2 種の大きさ、信頼度つき
│   ├── limit.rs                                 サイズ上限付きのファイル読み込みと、拒否ごとのコード
│   ├── measure.rs                               部品をミリメートルで計測し、CSV と JSON を出力
│   ├── offset.rs                                ピックした部品の保持位置と補正、トレイの補正表、ノズル軸
│   ├── orientation.rs                           マーク付き部品の向き（マークの方向から）
│   ├── pads.rs                                  パッドの列：ピッチ、そのばらつき、列からのパッドのずれ
│   ├── panels.rs                                ウィンドウのパネル：レンズ・パッケージ・撮影計画・ピックのずれ・画像チェック
│   ├── perspective.rs                           遠近補正：4つの角から、真上から見たように描き直した画像
│   ├── scale.rs                                 ピクセルからミリメートルへ、レンズの放射歪み
│   ├── server.rs                                共有クライアントによる EH3D サーバーへの経路と、テスト用のメモリ上のサーバー
│   ├── convention.rs                            サーバー規約の画像結果：作成、検査、SHA-256
│   ├── station.rs                               サーバーの模擬ビジョンステーション：ジョブ、応答、移植
│   ├── sharpmap.rs                              画像の鮮明さの地図：セルの格子、弱いセル、傾き
│   ├── square.rs                                四角いマークと任意の角度に回った十字
│   ├── tools.rs                                 サイドパネルの画像ツール：図形・部品・フォルダレポート・間隔によるスケール
│   ├── train.rs                                 見本から学習したマークを、回転した角度も含めて相関で再発見する
│   ├── turnpair.rs                              基板がどんな角度に回転していても、マークの欠けや余分があっても設計と対応付ける
│   ├── dotcard.rs                               100 % で印刷する丸い点のカード（ミリメートル単位の図面）
│   ├── light.rs                                 画像の照明が良好か：平均、白飛び、ゾーン、傾き
│   ├── mjpeg.rs                                 バイトを決して信用しないモーションJPEGフレームの読み取り
│   ├── tape.rs                                  部品テープの穴とポケット、およびテープのピッチとの比較
│   ├── verbs.rs                                 コマンド train、locate、pair、light-check、tape-locate、calib-card、jpeg
│   ├── shot.rs                                  ウィンドウの画像モード（--shot）：画面なしで読み取り・検証される引数
│   ├── shots.rs                                 視野より大きな部品を覆うカメラ位置を、寸法だけから求める
│   ├── browse.rs                                ファイルを開くウィンドウのファイル一覧
│   ├── checks.rs                                ウィンドウのパネル：相関、補正、パッド列、画像の比較
│   ├── readings.rs                              ウィンドウのパネル：ヒストグラム、エッジ、基準、サーバー設定
│   ├── gui.rs                                   ウィンドウ（egui、gui 機能）：状態を描き、操作を伝える
│   ├── lang.rs                                  ウィンドウの文言（8言語）
│   ├── viewport.rs                              画像の拡大縮小と移動（単純な数値）
│   └── window.rs                                画面なしのウィンドウの状態：画像・マーク・スケール・レンズ・位置合わせ
├── tests/
│   ├── readme.rs                                READMEとrunスクリプトが約束する内容を実行するテスト
│   ├── strict.rs                                コマンドラインのテスト：知らないオプションと語
│   ├── read_limit.rs                            コマンドラインが読むファイルのサイズ上限のテスト
│   ├── exchange.rs                              画像を読んでファイルを書くコマンドのテスト
│   ├── commands.rs                              correlate・rectify・pads・diff・server コマンドのテスト
│   ├── analysis.rs                              コマンド orientation、footprint、sharpmap、board のテスト
│   ├── capture.rs                               模擬カメラでのライブカメラ・校正・位置合わせとコマンド
│   ├── fuzz.rs                                  固定シードのランダム入力によるテスト：パニックしてはならない
│   ├── camera.rs                                プログラムとして実行する新コマンド：edge、histogram、refcheck、残差、server
│   ├── fuzz_camera.rs                           新しい測定と規約に対する固定シードのランダム入力
│   ├── fuzz_capture.rs                          フレーム読み取り・校正ファイル・位置合わせの固定シードのランダム入力
│   ├── training.rs                              学習したマークが描いた位置と角度で見つかること、およびそのファイルと矩形
│   ├── pairing.rs                               任意角度旋转的基板的标记按真值配对，并用随机点测试
│   ├── mjpeg.rs                                 符号化フレームの読み戻し、あらゆる位置での切断、バイト変更、巨大サイズ、ノイズ
│   ├── newcli.rs                                新しいコマンド：テキスト、JSON、上書きしないファイル、終了コード、ヘルプの例
│   ├── lightfuzz.rs                             あらゆる画像と設定に対する照明・テープ・カード（固定シード）
│   ├── discovery.rs                             敵対的または大量のネットワーク応答でも、安全でない候補は決して作られない
│   ├── shot_sample.rs                           サンプルが画像用ウィンドウで開くことのテスト
│   ├── vision.rs                                ライブラリとコマンドラインのテスト
│   └── reqs.rs                                  要件一覧のテスト
├── examples/
│   ├── design.csv                               試すためのサンプル
│   ├── design_pair.csv                          marks.pgm の 2 つのマーク用の設計
│   ├── marks.pgm                                試すためのサンプル
│   ├── measured.csv                             試すためのサンプル
│   ├── packages.csv                             inspect 用のパッケージ本体寸法の表
│   └── vision.cfg                               すべての値とその意味を記した設定ファイル
├── docs/
│   ├── architecture.json                        アーキテクチャ図のデータ
│   ├── NOT_YET.md                               このバージョンに含めない良いアイデアとその理由、再検討の時期
│   ├── REQUIREMENTS.md                          要件の一覧：それぞれの状態、証拠、テスト
│   ├── SCOPE.md                                 目的、行わないこと、決して判断しないこと
│   ├── STATUS.md                                存在するもの、足りないもの、未決のこと
│   └── structure.json                           構成図のデータ
├── tools/                                       CI と README のチェック、図のツール
├── images/                                      バナーとアーキテクチャ・リポジトリ構成の図
├── .github/                                     CI とリリースのワークフロー、Issue と PR のテンプレート
├── eh3d.project.json                            マニフェスト：バージョン、役割、成熟度、ビルドコマンド
├── Cargo.toml Cargo.lock                        ワークスペースと固定された依存関係
├── rust-toolchain.toml                          固定されたRustツールチェーン
├── CHANGELOG.md                                 各バージョンの変更点
├── AGENTS.md                                    リポジトリを変更する人のための規則
├── SECURITY.md SUPPORT.md                       脆弱性の報告方法と問い合わせ先
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   貢献方法、行動規範、ライセンス
```

</details>

## 🗺️ ロードマップと既知の制限

**現時点で分かっている制限**

* ウィンドウはまだ画面で確認していません。`align` と、見て・動かし・もう一度見るループはマークを位置で設計と対応付けるため、そこでは大きく回転した基板や欠けたマークは拒否されます（`pair --any-turn` は対応付けできます）。
* 機械でも実際のファイルの集まりでも試しておらず、独立したレビューも受けていません。
* 読むのは画像ファイルで、ライブカメラは模擬でのみ動き（Windows、V4L2、libcamera のバックエンドは実機で試していない骨組み）、見つけるのは丸いマーク、四角いマーク、十字形のマークだけで、テストが描いた画像で試しただけで、実際の基板の写真では試していません。部品の回転はその画素から長方形として測るため、長方形でない部品には長軸の回転が与えられます。
* サーバーにはビジョン用のプラグインがないため、プログラムはストアのアーティファクト（結果は `POST /api/v1/artifacts`、機械の設定は `GET /api/v1/artifacts` と `GET /api/v1/artifacts/{id}`）を、独自の名前と形式（`vision-<kind>.json`、`vision-<machine>.cfg`）で使います。現時点ではメモリ上のサーバーとループバックのソケットに対してのみテストしており、mDNS によるサーバー検索（`server_mdns`）はリゾルバーを組み込んでいないスイッチです。

**次のステップ**

* 機械のライブカメラ（EH3D-NODE-BUNGARD-AUX）を読み取り、実際の基板の写真で試し、コマンドにある新しいマークの学習をウィンドウに入れる。
* ウィンドウを画面で確認して仕上げる。
* 実際のファイルとスイートの他のプログラムで試す。
* ライブのバックエンド（Media Foundation、V4L2、libcamera からのフレーム）を書き、校正ボードを実際のレンズで、点のカードを紙で、位置合わせを実際のステージで試す。

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
* [スコープ](docs/SCOPE.md)： このプログラムの目的、行わないこと、決して判断しないこと。
* [要件](docs/REQUIREMENTS.md)： 機能の一覧、それぞれの状態、それを示すテスト。
* [状況](docs/STATUS.md)： 存在するもの、足りないもの、まだ未決のこと。
* [まだ対象外](docs/NOT_YET.md)： このバージョンに含めなかった良いアイデア。
* [リポジトリ変更の規則](AGENTS.md)： リポジトリを変更する人のための規則。
* [アーキテクチャと構成のデータ](docs/architecture.json)：2 枚の図を描くためのデータ。

## 🤝 コントリビュートとコミュニティ

* プルリクエストの前に `cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test`、`python -I tools/ci_validate.py` を実行し、すべて通してください。[CONTRIBUTING.md](CONTRIBUTING.md) と[行動規範](CODE_OF_CONDUCT.md)を参照。
* README は 8 言語あります。文章の変更は 8 ファイルすべてで行い、2 枚の図は `tools/make_architecture.py` と `tools/make_structure.py` が `docs/architecture.json` と `docs/structure.json` から描きます。
* 脆弱性は非公開で報告してください：[SECURITY.md](SECURITY.md)。質問・提案・報告：electrohobby3d@gmail.com（[SUPPORT.md](SUPPORT.md)）。

## 👤 作者

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 ライセンス

ソフトウェア：GPL-3.0-or-later。[LICENSE](LICENSE) を参照。スイートのハードウェア設計には CERN OHL-S を使います。
