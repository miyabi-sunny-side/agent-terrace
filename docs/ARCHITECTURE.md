# Architecture

## 目的と境界

agent terrace は、tailnet 内のスマートフォンから tmux 上で動く coding agent
を流し見し、将来は agent-talk 経由で作業指示を届けるための Web
アプリケーションです。ターミナルクライアントではなく、既存システムの
読み取り専用ビューと薄い agent-talk クライアントとして振る舞います。

```text
[Svelte PWA]
      │ HTTPS (Tailscale Serve / MagicDNS)
      ▼
[Rust / axum server]
      ├── agent registry: agent-talk who
      ├── screen:         tmux capture-pane -pet <pane>
      └── future letters: agent-talk journal / send
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

## 将来の送信機能の設計制約

端末へ任意の文字列を打ち込める API は作りません。`send-keys` の所有者は
今後も agent-talkd だけです。

将来の送信 API は `{agent, skill, body}` だけを受け付け、次を守ります。

1. `agent` は `agent-talk who` の登録情報と照合する。
2. `skill` は agent terrace 側の静的 allowlist でも検証する。
3. 本文とは分離した `--skill` と、外部送信元ラベルを付けて
   `agent-talk send` を呼ぶ。
4. 本文は stdin から agent-talkd の journal だけへ渡し、tmux の引数や
   bell へ混入させない。

agent-talkd 側の `send-v2`、外部送信元 allowlist、skill allowlist、宛先別
syntax は実装・検証済みの前提です。agent terrace 側の allowlist は
defense in depth として維持します。

## セキュリティ境界

- サーバーは既定で localhost のみ listen し、Tailscale Serve で tailnet
  内へ HTTPS 公開します。
- インターネットへの直接公開や Cloudflare 経由の公開は採用しません。
- デバイス認証は tailnet 参加に委ね、アプリ内認証は作りません。端末ごとの
  制限が必要になった場合は Tailscale ACL で行います。
- 指示本文の検閲フィルターは作りません。公開境界は Tailscale、端末入力の
  構造的な防御は agent-talkd に一本化します。

## 非目標

- `/model` など、ターミナル固有の対話 UI を Web に移植しません。
- ターミナル出力をインターセプトして意味解析しません。
- agent-talk journal に agent 出力の複製を保存しません。journal は
  agent 間メッセージの置き場として小さく保ちます。
- 将来、構造化された会話が必要になった場合は Claude Code の
  `~/.claude/projects/**/*.jsonl` または Codex の `~/.codex/sessions/` を
  読み取り専用で参照します。

## 技術とリポジトリの責務

- Backend: Rust、axum、tokio
- Frontend: Svelte 5、Vite、Bun、installable PWA
- UI design: Sumi design system と [`DESIGN.md`](DESIGN.md)
- systemd unit、update timer、デプロイスクリプトはこのリポジトリではなく
  `home-server/systemd/` に置きます。
