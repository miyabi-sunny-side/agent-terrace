# Architecture

## 目的と境界

agent terrace は、信頼できる LAN または tailnet 内のスマートフォンから tmux
上で動く coding agent を流し見し、agent-talk 経由で作業指示を届けるための
Web アプリケーションです。ターミナルクライアントではなく、既存システムの
読み取り専用 Screen viewer と薄い agent-talk Letters クライアントとして
振る舞います。

```text
[Svelte PWA]
      │ HTTP (trusted LAN) / HTTPS (Tailscale Serve)
      ▼
[Rust / axum server]
      ├── agent registry: agent-talk who
      ├── screen:         tmux capture-pane -pet <pane>
      ├── letters:        agent-talk mailbox-list-v1 mobile
      └── delivery:       agent-talk send <pane> --from mobile
```

## 画面読み取り

- agent 一覧は `agent-talk who` を唯一の登録情報として使います。
- pane の表示には `tmux capture-pane -pet <pane>` をポーリングで実行します。
- API は capture の直前にも登録一覧を確認し、完全一致した pane ID だけを
  `tmux` の分離引数として渡します。
- `attach` は禁止です。クライアントとして参加するとデスクトップ側の tmux
  window size に干渉するためです。
- ANSI SGR はフロントエンドで安全な表示データへ変換します。OSC、カーソル
  移動などの非 SGR 制御列は実行も再現もせず除去します。

## Letters の取得と送信

端末へ任意の文字列を打ち込める API は作りません。`send-keys` の所有者は
agent-talkd だけです。

送信 API は `{agent, skill, body}` だけを受け付け、次を守ります。

1. `agent` は `agent-talk who` の登録情報と照合する。
2. `skill` は agent terrace 側の静的 allowlist（`deliver`、`commit`）でも
   検証する。
3. 本文とは分離した `--skill` と、外部送信元ラベルを付けて
   `agent-talk send` を呼ぶ。
4. 空白だけの本文を拒否し、UTF-8 で 16 KiB までに制限する。
5. 本文は stdin から agent-talkd の mailbox だけへ渡し、コマンド引数や
   tmux の bell へ混入させない。

履歴 API は `agent-talk mailbox-list-v1 mobile` の version 1 schema だけを
受理します。`GET /api/letters` は初回に最大 500 件を取得し、その後は最後に
見た global event ID を `after` に渡します。フロントエンドは mailbox 全体の
event を統合してから、選択中 agent の `target_pane` へ絞り込みます。Letters
tab が mount されている間だけ 2 秒ごとに差分を取得し、tab を離れると timer
を停止します。

agent-talkd v0.4.0（commit `91e1348`）以降の外部送信元 allowlist、skill
allowlist、宛先別 syntax、mailbox 一覧・返信契約をデプロイ前提とします。
agent terrace 側の allowlist と schema 検証は defense in depth として維持
します。

agent terrace server が tmux pane から起動されて親の `TMUX_PANE` を継承
していても、`mailbox-list-v1` と `send` の子プロセスからはこの環境変数を
明示的に除去します。agent-talk に external caller として認識させ、pane
agent の identity と `mobile` mailbox の identity が混ざるのを防ぎます。

## セキュリティ境界

- サーバーは既定で localhost のみ listen します。信頼できる LAN へ公開する
  運用では `0.0.0.0:5002` を listen し、LAN 側の firewall 境界を使います。
- HTTPS または tailnet 越しの接続が必要な場合は、localhost で listen して
  Tailscale Serve を使います。
- ブラウザー API は same-origin とし、CORS を有効にしません。
- インターネットへの直接公開や Cloudflare 経由の公開は採用しません。
- アプリ内認証は作りません。LAN 公開時は同じ LAN にいる利用者を信頼し、
  tailnet 公開時はデバイス認証を tailnet 参加に委ねます。端末ごとの制限が
  必要になった場合は Tailscale ACL で行います。
- 指示本文の検閲フィルターは作りません。公開境界は LAN firewall または
  Tailscale、端末入力の構造的な防御は agent-talkd に一本化します。

## 非目標

- `/model` など、ターミナル固有の対話 UI を Web に移植しません。
- ターミナル出力をインターセプトして意味解析しません。
- agent-talk mailbox に agent の terminal 出力を複製しません。mailbox は
  agent 間と外部クライアントのメッセージ置き場として小さく保ちます。
- coding agent の session log viewer は現在のスコープに含めません。

## 技術とリポジトリの責務

- Backend: Rust、axum、tokio
- Frontend: Svelte 5、Vite、Bun、installable PWA
- UI design: Sumi design system と [`DESIGN.md`](DESIGN.md)
- systemd unit、update timer、デプロイスクリプトはこのリポジトリではなく
  `home-server/systemd/` に置きます。
