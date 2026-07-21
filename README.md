# agent terrace

スマートフォンから、tmux 上の coding agent を眺め、agent-talk 経由で手紙を
届けるためのセルフホスト PWA です。Screen は `capture-pane` による読み取り
専用表示、Letters は履歴の閲覧と構造化された指示の送信を提供します。

## 必要なもの

- Rust toolchain
- Bun
- tmux
- 同じ tmux server で動く `agent-talkd` v0.4.0 以降と `agent-talk` コマンド
- スマートフォンから接続する場合は Tailscale

Letters は agent-talkd commit `91e1348` で追加された外部 mailbox API を
前提にします。デプロイ前に共有 daemon と CLI を v0.4.0 以降へ更新して
ください。このリポジトリの実装作業では共有 daemon の更新は行っていません。

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
- `GET /api/skills`: 送信時に選択できる静的 skill 一覧
- `GET /api/letters?after=<id>&limit=<1..500>`: `mobile` mailbox の履歴と差分
- `POST /api/letters`: `{agent, skill, body}` 形式の手紙を送信

画面 API は呼び出しごとに registry を再確認し、完全一致した pane ID だけを
`tmux capture-pane -pet` へ分離引数で渡します。pane が確認後に終了した
場合は `410 pane_unavailable` を返します。

手紙 API は `agent` を registry と再照合し、`skill` を `deliver` / `commit`
の allowlist で検証します。本文は空白だけを許可せず、UTF-8 で 16 KiB まで
です。本文をコマンド引数へ連結せず、`agent-talk send <pane> --from mobile`
の stdin へ渡します。API は same-origin でのみ使い、CORS は有効にしません。
`send-keys` や任意の端末入力 API は提供しません。

agent terrace 自体を tmux pane から起動した場合でも、mailbox 一覧・送信の
子プロセスでは継承した `TMUX_PANE` を明示的に除去します。これにより
agent-talk は呼び出し元を pane agent ではなく external client として扱い、
`mobile` mailbox の caller identity を維持します。

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

Phase 1 の Screen viewer と Phase 2 の Letters viewer / composer を実装済み
です。恒久的な設計判断は [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)、
デプロイ前提と今後の機能は [`docs/ROADMAP.md`](docs/ROADMAP.md) を参照して
ください。

agent を選ぶと、Screen / Letters のどちらにも画面右下の「手紙」が表示されます。
Screen を読みながら「手紙」を開いて指示を書けます。開閉や view の切り替えでは
下書きと skill を保持し、別の agent を選ぶとリセットします。Letters は送受信履歴を
確認する timeline です。
