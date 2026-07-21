# agent terrace

スマートフォンから、tmux 上の coding agent を読み取り専用で眺めるための
セルフホスト PWA です。Phase 1 は agent 一覧と `capture-pane` による流し見
だけを提供します。端末への入力 API、手紙、skills はまだ提供しません。

## 必要なもの

- Rust toolchain
- Bun
- tmux
- 同じ tmux server で動く `agent-talkd` と `agent-talk` コマンド
- スマートフォンから接続する場合は Tailscale

## セットアップ

```sh
cd client
bun install --frozen-lockfile
bun run build
cd ..
cargo build --release
```

サーバーは既定で `127.0.0.1:3000` だけを listen します。

```sh
cargo run --release
```

別の localhost port が必要な場合だけ、listen address を変更できます。

```sh
AGENT_TERRACE_ADDR=127.0.0.1:3100 cargo run --release
```

開発時は別ターミナルでバックエンドと Vite を起動します。

```sh
cargo run
cd client && bun run dev
```

Vite は `/api` を `127.0.0.1:3000` へ proxy します。

## Tailscale Serve

サーバーを起動した状態で、同じマシンから tailnet 内へ HTTPS 公開します。
`tailscale funnel` は使用しません。

```sh
tailscale serve --bg 3000
tailscale serve status
```

表示された HTTPS URL を tailnet に参加済みの Android Chrome で開き、
ブラウザーメニューからホーム画面へ追加します。停止時は、このマシンの他の
Serve 設定への影響を確認したうえで `tailscale serve reset` を実行します。

このリポジトリの検証では、外部状態を変更しないため Serve の有効化とスマホ
実機確認は行いません。デプロイ時に上記 2 点を確認してください。

## API

- `GET /api/agents`: `agent-talk who` に登録された agent 一覧
- `GET /api/agents/{pane}/screen`: 登録済み pane の現在画面

画面 API は呼び出しごとに registry を再確認し、完全一致した pane ID だけを
`tmux capture-pane -pet` へ分離引数で渡します。pane が確認後に終了した
場合は `410 pane_unavailable` を返します。

## 検証

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cd client
bun run fmt:check
bun run lint
bun run test
bun run build
```

## 開発時の設計資料

UI を編集する前に [`docs/DESIGN.md`](docs/DESIGN.md) を読んでください。
共有ルールの大元は `~/.dotfiles/agent/common/designs/` にあり、この
リポジトリの文書には agent terrace 固有の override だけを記録します。

## フェーズ境界

journal の時系列取得契約と認可境界は未決です。そのため Phase 2 の journal
reader、送信フォーム、skills 選択はまとめて対象外にしています。恒久的な
設計判断は [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)、今後の境界と
未決事項は [`docs/ROADMAP.md`](docs/ROADMAP.md) を参照してください。
