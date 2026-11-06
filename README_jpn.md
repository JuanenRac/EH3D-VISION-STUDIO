<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-VISION-STUDIO banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-VISION-STUDIO
<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | 🇯🇵 <b>日本語</b> | <a href="README_por.md">🇵🇹 Português</a></p>

### カメラ画像の基準マーク：マーク、スケール、基板の補正

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.3-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**正直さの確認 - 現在実際に動くもの：** Rust ライブラリとコマンドライン プログラム（`eh3d-vision`）で、コンピューター上で実行される 138 件のテストがあります：画像から丸い基準マークを見つけ、画素をミリメートルに変換し、基板を設計どおりの位置に合わせる回転と移動量を計算します。ウィンドウ（`--features gui` でビルドする `eh3d-vision-gui` プログラム）があり、画像を開き、マークを座標つきで表示し、スケールを設定し、グリッドからレンズの歪みを求め、設計と位置合わせして ISOCAM 用のオプションを書き出しますが、ウィンドウはまだ画面で確認していません。機械では試しておらず、実際のファイルの集まりでも確認していません。

---

> **安全とセキュリティの状況：** このプログラムは機械を制御せず、安全システムでもありません。出力は、機械で何かを実行する前に人が確認する必要があります。ここにあるものは実機で動かしたことがありません。


## 📋 ひと目で

| | |
|---|---|
| バージョン | `0.0.3` |
| 成熟度 | 足場 |
| テスト | 138 |
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
| テスト | 🟡 | 138 件のテストが通ります（`cargo test`）。通常ケース、境界ケース、破損入力を含みます。 |
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
* **unsafe コードなし：** ライブラリとプログラムに `#![forbid(unsafe_code)]`。
* **ISOCAM 用のマーク：** `align --isocam-args` は各マークの組を `eh3d-isocam` のオプション `--fiducial 設計X,設計Y,測定X,測定Y` として出力します。
* **ウィンドウ：** `--features gui` でプログラム `eh3d-vision-gui` としてビルドします（Windows ではコンソールなしで開きます）。PNG または PGM の画像をズームと移動つきで開き、編集できる値でマークを見つけ、座標つきで描画し、大きさが既知のマークでスケールを設定し、マークのグリッドからレンズの歪みを求め、CSV ファイルの設計と位置合わせし（マークは位置で対応付けるので順序は関係ありません）、位置合わせが合格のときだけ ISOCAM のオプション `--fiducial` を書き出します。8 言語に対応し、ネットワークは不要です。
* **設定ファイル：** `vision.cfg` に調整されるすべての値（マークの見た目、ターゲットのグリッド、合格する位置合わせの限度、読み込む最大ファイル）を書きます。間違いは行番号つきで拒否され、`examples/vision.cfg` がすべてを一覧にしています。

## 🔒 セキュリティと安全

| 脅威 | 対策 |
|---|---|
| 破損・途中切れ・空のファイル | パニックなし：テストは有効な入力をあらゆる位置で切り、ゴミデータも与えます |
| 意味のない数値（負、無限、極端に大きい） | 値を示すエラーで拒否され、もっともらしい値に丸められることはありません |
| 安全でないメモリ使用 | `forbid(unsafe_code)` の安全な Rust のみ |

**防御していないこと：** エラーなく作られた結果は、検証された結果ではありません。ここには機械の限界を知るものはなく、プログラムは独立したレビューを受けていません。脆弱性は非公開で報告してください（[SECURITY.md](SECURITY.md) 参照）。

## 🔌 インターフェース

ライブラリは `eh3d_vision_studio`（`use eh3d_vision_studio::...`）、コマンドラインは `eh3d-vision` です。コマンドは次のとおりで、結果は標準出力に表示され、エラーは標準エラー出力に 0 以外の終了コードで報告されます。ネットワークアクセスはありません。

```text
eh3d-vision find IMAGE [--light] [--min-diam PX] [--max-diam PX] [--known-mm D]   the round marks found (--known-mm gives millimetres)
eh3d-vision align DESIGN.csv MEASURED.csv [--scale] [--isocam-args] [--max-rms 0.1] [--max-stretch 0.01]
eh3d-vision-gui [IMAGE] [DESIGN.csv]   the window (a build with --features gui); on Windows it opens with no console
```

## ⚙️ 設定

調整される値はコードではなく、単純な `キー = 値` のファイルに書きます。プログラムを起動したフォルダー、またはプログラムの隣の `vision.cfg` です（変数 `EH3D_VISION_CONFIG` で別のファイルを指定できます）。ウィンドウの言語、マークが暗いか明るいか、しきい値、直径の範囲、マークの充填率と細長さ、スケールを決めるマークの大きさ、グリッドの列数と行数、位置合わせでサイズの変更を許すか、位置合わせが満たすべき限度、読み込む最大ファイルを設定します。`examples/vision.cfg` はすべてのキーを意味と起動時の値つきで一覧にしています。ファイルがなければその値が使われ、間違いのあるファイルは行番号つきで拒否されます。コマンドラインでは同じことをオプションで指定します（インターフェースを参照）。

## 🚀 クイックスタート

安定版の Rust ツールチェーン（1.88 以降）が必要です。EH3D-COMMON リポジトリをこのリポジトリの隣にクローンしておく必要があります。画像リーダーはそこからパスで取り込みます。

```bash
cargo build --release
cargo test
cargo run --release -- align examples/design.csv examples/measured.csv
cargo run --release --features gui --bin eh3d-vision-gui   # the window
cargo run --release --features gui --bin eh3d-vision-gui -- --shot screen.png --open examples/marks.pgm --open examples/design.csv   # a picture of the window
```

手動で開かずにウィンドウの画像を撮るには、`--shot FILE.png` を付けて実行します（`--size WxH`, `--theme forge|dark|light`, `--wait N`, `--lang CODE`, `--open FILE`, `--replace` も使えます）。指定のサイズで開き、PNG を保存し、設定は保持せず、自動で閉じます（終了コードは、ファイルを書き込めたら 0、書けなければ 1、引数が誤りなら 2）。

## 🧪 テスト

テストは 138 件あります（`cargo test`、CI では Windows と Linux で実行）。通常ケース、境界ケース、破損入力を確認し、空のファイル、ゴミデータ、途中で切れたファイルでもパニックしません。CI は `cargo fmt --check`、警告をエラーにした `cargo clippy`、`python -I tools/ci_validate.py` も実行します。

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
│   ├── config.rs                                設定ファイル：調整されたすべての値
│   ├── fiducial.rs                              画像の円形マーク：しきい値・領域・検査
│   ├── grid.rs                                  マークのグリッドから求めるレンズの歪み
│   ├── points.rs                                設計の点のファイル
│   ├── scale.rs                                 ピクセルからミリメートルへ、レンズの放射歪み
│   ├── shot.rs                                  ウィンドウの画像モード（--shot）：画面なしで読み取り・検証される引数
│   ├── browse.rs                                ファイルを開くウィンドウのファイル一覧
│   ├── gui.rs                                   ウィンドウ（egui、gui 機能）：状態を描き、操作を伝える
│   ├── lang.rs                                  ウィンドウの文言（8言語）
│   ├── viewport.rs                              画像の拡大縮小と移動（単純な数値）
│   └── window.rs                                画面なしのウィンドウの状態：画像・マーク・スケール・レンズ・位置合わせ
├── tests/
│   ├── readme.rs                                READMEとrunスクリプトが約束する内容を実行するテスト
│   ├── strict.rs                                コマンドラインのテスト：知らないオプションと語
│   ├── shot_sample.rs                           サンプルが画像用ウィンドウで開くことのテスト
│   └── vision.rs                                ライブラリとコマンドラインのテスト
├── examples/
│   ├── design.csv                               試すためのサンプル
│   ├── marks.pgm                                試すためのサンプル
│   ├── measured.csv                             試すためのサンプル
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
* 読むのは画像ファイルでライブカメラではなく、見つけるのは丸いマークだけ（十字や四角は不可）で、テストが描いた画像で試しただけで、実際の基板の写真では試していません。

**次のステップ**

* 機械のライブカメラ（EH3D-NODE-BUNGARD-AUX）を読み取り、十字と四角を見つけ、実際の基板の写真で試し、新しいマークの学習をウィンドウに加える。
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
