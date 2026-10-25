# halokey

> 🌐 [English](README.md) · [中文](README.zh.md)

Ulanzi Vibe Key（AU05）のためのクロスプラットフォームなオープンソース・コントロールセンターです。3 つのキーとノブの割り当て変更、ホスト側アクションの実行、画面上の円形メニューからの選択、ライト・マイク・電源設定の管理ができます。1 つのコードベースで Linux、macOS、Windows に対応します。

**ステータス：pre-alpha。** Linux ではプロトコル、通信、ホストモデル、ヘッドレスデーモン、設定ウィンドウとハロー（円形メニュー）オーバーレイ付きのトレイアプリまで、一通り動作しています。macOS と Windows のバックエンドはビルドできますが、実機ではまだ動かしていません。

## すぐに試す

![キーマップ画面](docs/screenshots/keymap-en.png)

Rust ツールチェーン（`rustup`）が必要です。Linux では `packaging/linux/70-halokey.rules` を `/etc/udev/rules.d/` にコピーしてレシーバーを挿し直すか、一時的に `chmod a+rw /dev/hidraw*` を実行してください。

```sh
cargo run -p halokey-cli -- list                 # レシーバーのベンダーチャネルを探す
cargo run -p halokey-cli -- status               # オンライン状態、バッテリー、電圧
cargo run -p halokey-cli -- keys                 # デバイス側の 6 つのキースロット
cargo run -p halokey-cli -- monitor --heartbeat  # キーを押しながらベンダーイベントをデコード
cargo run -p halokey-cli -- set-key 0 kb 0x68    # キー 1 を F13 に書き込んで読み戻す（デバイスに保存される）
cargo run -p halokey-cli -- run -v               # デーモン：ハートビート、ジェスチャー、uinput 入力（/dev/uinput が必要）
cargo run -p halokey-cli -- export-keymap > my.json   # デバイスのテーブルを編集可能なホスト用キーマップ JSON に書き出す
cargo run -p halokey-cli -- run --config docs/examples/knob-moves-pointer.json   # ノブでマウスポインターを動かす
cargo run -p halokey-cli -- run --config docs/examples/switch-apps-and-macro.json # キーでアプリ切り替え、ノブ押しでマクロ実行
cd ui && npm install && npm run dev              # 一度だけ / 別ターミナルで常駐：開発版は Vite 開発サーバーを読み込む
cargo run -p halokey-app --no-default-features   # トレイアプリ（開発時は Vite を読み込む。フラグを外すと ui/dist を埋め込む）：設定ウィンドウ + ハローオーバーレイ、~/.config/halokey/keymap.json を読む
```

`docs/examples/halo-menu.json` はノブ押しに 2 階層のハローメニューを割り当てる例です。`~/.config/halokey/keymap.json` にコピーするか、設定ウィンドウに貼り付けてください。

Debian/Ubuntu へのインストールは、GitHub Releases ページの `.deb`（x86_64 と arm64）を使うか、ローカルでビルドします：`apps/halokey` で `cargo tauri build --bundles deb --no-sign` を実行し、`sudo dpkg -i target/release/bundle/deb/halokey_*.deb` でインストールしたら、同梱の udev ルールを反映させるためにレシーバーを挿し直してください。AppImage は udev ルールをインストールできないため、ビルドしていません。

macOS へのインストール：`apps/halokey` で `cargo tauri build --bundles dmg` を実行し、`target/release/bundle/dmg/` の `.dmg` を開いて halokey を「アプリケーション」にドラッグします。アプリは ad-hoc 署名で公証（notarize）されていないため、ダウンロードしたものを初めて開くとブロックされます。「システム設定 → プライバシーとセキュリティ」を開き、「このまま開く」をクリックしてください。続いて halokey がアクセシビリティ権限を求めます。キーやマウスのイベントを送るのに必要なので、「プライバシーとセキュリティ → アクセシビリティ」で許可し、数秒待ってからキーを押せば有効になります。再起動は不要です。アプリは Dock ではなくメニューバーに常駐します。

![設定画面](docs/screenshots/settings-en.png)

設定画面からはライト、電源、マイク、デバイス本体のキーテーブルを直接書き込めるほか、キーマップファイルのインポート / エクスポートや、デバイスからの再構築もできます。

設定ウィンドウのデバイス画像（`ui/public/device.png`）は [Open VibeKey](https://github.com/palaemonboy/OpenVibeKey) リポジトリ（MIT）のものです。

`set-key` はデバイス本体のテーブルを書き換えます。まずは壊れても困らないスロットを選び、表示される元の値を控えておいてください。

## なぜ新しく作るのか

Ulanzi Studio は Windows と macOS にしか対応していません。これまでのコミュニティプロジェクトは macOS 専用か Linux 専用のどちらかで、Studio にある円形メニューを備えたものもありません。halokey は Rust と Tauri 2 で一から作り直し、デバイスのロジックを一度だけ書いて、プラットフォームごとの違いは薄いアダプターだけにしています。

## 予定している機能

| 機能 | 状態 |
|---|---|
| ベンダーチャネル（TEA 暗号化 HID、report ID `0x55`） | **完了**：`halokey-proto` + `halokey-hid`、Linux `hidraw` で検証済み |
| キーとノブの割り当て変更、レイヤー、シングル / ダブル / 長押し | `halokey-host` で**完了**。olanzi からテストごと移植 |
| ホストアクション：キーボード、マウスボタン、ホイール、ポインター移動、アプリ切り替え、マクロ、シェル | シェルコマンド以外は Linux X11 上で `halokey run` により動作。シェルコマンドは予定 |
| 円形「ハロー」メニュー：ノブで操作する 8 項目のオーバーレイ | Linux で**完了**：キーマップの `halos`、入れ子メニュー、GNOME/X11 で検証済み |
| ライト、ノイズリダクション、スタンバイ時間、バッテリー表示 | **完了**：ライト、ノイズリダクション、マイクのオン/オフ、スタンバイとスリープの遅延、バッテリー、バージョン。さらに本体が実際にスリープできるよう、アイドル時に制御を手放す仕組み |
| トレイ / メニューバーアイコン、ログイン時起動、自動アップデート | **完了**（`apps/halokey`、Tauri 2）：トレイアイコン、設定ウィンドウ、ログイン時の自動起動。インストール版はアップデートを確認して通知し、確認後にインストール |

## アップデートとリリース

インストール版（Linux deb、macOS .app、Windows NSIS/MSI）は、起動 30 秒後とその後 6 時間ごとに `https://github.com/jiangzhuo/halokey/releases/latest/download/latest.json` を確認します。これは既定でオンで、設定の「自動でアップデートを確認」でオフにできます。「アップデートを確認」ボタンでいつでも手動確認できます。新しいバージョンごとにシステム通知は 1 回だけ出ます。アップデートがある間は、トレイに「アップデート vX.Y.Z をインストール…」が表示され、設定 → アップデートにもリリースノートへのリンク付きで表示されます。確認するとダウンロード、署名の検証、インストール、再起動まで自動で行います。ソースから動かしているビルド（`cargo run`、`target/*/halokey-app`）はアップデートを確認しません。トレイメニューと通知は画面の言語設定に従います。

- Linux deb のアップデートでは管理者パスワードを 1 回求められます（システムの認証ダイアログ）。キャンセルした場合、アップデーターのプラグインが代替のプロンプトでもう一度尋ね、その後「キャンセル」ではなく「インストール失敗」と表示することがあります。
- macOS 版は Apple Developer ID で署名されていないため、アップデート後に以前許可したアクセシビリティ権限が効かなくなることがあります。システム設定ではオンのままに見えます。アップデート後にキーやノブが反応しなくなったら、「システム設定 → プライバシーとセキュリティ → アクセシビリティ」で halokey を削除してから追加し直してください。
- アップデーターを含む最初のリリースが起点になります。それより前にインストールしたものは、一度だけ手動でアップデートしてください。

リリース手順（メンテナー向け）：`apps/halokey/tauri.conf.json` とルートの `Cargo.toml`（`[workspace.package] version`）のバージョンを上げてコミットし、`vX.Y.Z` タグをプッシュします。CI はタグと両方のバージョンを照合し、一致しなければリリースジョブが失敗するので、タグを打つ前に両方を上げてコミットしてください。CI は署名付きの deb（x86_64、arm64）、macOS .app/.dmg（arm64、x86_64）、Windows NSIS/MSI をビルドし、`latest.json` を生成してドラフトリリースを作ります。GitHub でドラフトを確認して公開してください。ユーザーにアップデートが届くのは公開後です。アップデート用の成果物が有効になっているため、ローカルの `cargo tauri build` は `TAURI_SIGNING_PRIVATE_KEY` がないと失敗します。ローカルビルドでは `--no-sign` を付けてください（アップデート用の署名を省くだけなので、ローカルで使う分には問題ありません）。

## プラットフォーム対応表

| コンポーネント | Linux | macOS | Windows |
|---|---|---|---|
| HID 通信 | hidapi 経由の `hidraw` | hidapi 経由の IOKit | hidapi 経由の Win32 HID |
| 入力の送出 | uinput（完了） | CGEvent（ビルド確認のみ、未テスト） | SendInput（ビルド確認のみ、未テスト） |
| アプリ切り替え | X11 EWMH（完了）。Wayland では XWayland クライアントのみ | NSWorkspace + `open`（ビルド確認のみ） | SetForegroundWindow（ビルド確認のみ） |
| オーバーレイウィンドウ | X11 対応済み。GNOME Wayland でも動作（位置はコンポジターが決定） | フォーカスを奪わないパネル | フォーカスを奪わないウィンドウ |

## リポジトリ構成

```text
crates/proto     TEA、フレームのエンコード/デコード、コマンド表、キーテーブル – 副作用なし、ユニットテスト多数
crates/hid       hidapi 通信、ハートビート、ホットプラグ
crates/host      キーマップモデル、ジェスチャールーター、マクロ、アクション実行
crates/input     uinput / CGEvent / SendInput バックエンド
crates/wm        アプリ切り替えバックエンド
crates/runtime   CLI とアプリで共有するデバイスサービススレッド
apps/halokey     Tauri 2 シェル：トレイ、設定ウィンドウ、ハローオーバーレイ
docs/            設計とプロトコルのメモ。英語が既定、`.zh.md` が中国語版
tools/           リポジトリのチェックツール
```

## 謝辞

halokey は以下のプロジェクトの成果の上に成り立っています。いずれも MIT ライセンスです。halokey に含まれる部分のライセンス表記は [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) にあります。

- [olanzi](https://github.com/MrCroxx/olanzi)（作者 Croxx）：AU05 プロトコルのリバースエンジニアリング記録と macOS ネイティブアプリ。halokey のプロトコルの知識はその資料に基づいています。ジェスチャールーター、アクション実行、キーブリッジはテストごと移植し、キーマップ形式もそこから派生させています（ファイルに互換性はありません）。macOS の仮想キーコード表と、プロトコルのテストで使う実機の記録もここから来ています。
- [vibed](https://github.com/ihavespoons/vibed)（作者 Ben Gittins）：Vibe Key 用の macOS デーモン。halokey は、それがベンダー SDK から復元した内容を使っています：マウスページのエントリ種別（実機で確認済み）、電源 / マイク / スタンバイのコマンド、ライトの点灯時間の値。
- [Open VibeKey](https://github.com/palaemonboy/OpenVibeKey)（作者 palaemonboy）：AU05 用の macOS カスタマイズアプリ。設定ウィンドウのデバイス画像はここから来ています。

halokey は独立したコミュニティプロジェクトです。Ulanzi や Kehwin とは関係がなく、承認も受けていません。ベンダーのバイナリは一切含まず、読み込みもしません。

## ドキュメント

| # | ドキュメント | 内容 |
|---|---|---|
| 01 | [Architecture](docs/01-architecture.md) | crate 構成、プラットフォームアダプター、オーバーレイ、既知のリスク |
| 02 | [Protocol Notes](docs/02-protocol-notes.md) | olanzi の記録以外に halokey が実測した内容 |
| 03 | [Configuration Format](docs/03-config-format.md) | キーマップ JSON：コントロール、アクション、レイヤー、ハロー |

ドキュメントは英語版（既定のファイル名）と中国語版（`.zh.md`）のみで、日本語版はありません。`python3 tools/check_docs.py` が各版の同期を確認します。

## ライセンス

[MIT](LICENSE)
