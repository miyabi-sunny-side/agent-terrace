# Roadmap

## 前提: agent-talkd

agent terrace の Letters は、agent-talkd v0.4.0（commit `91e1348`）以降で
実装・検証済みの次の契約を前提にします。

- fail-closed な `send-v2` protocol
- `--from` の表示ラベルと返信可能な pane identity の分離
- 外部送信元、skill、宛先別 slash/dollar syntax の daemon 側検証
- 本文を mailbox storage のみに保存し、固定 bell だけを tmux へ届ける構造
- `mobile` mailbox の version 1 一覧 API と event ID による差分取得
- 外部 mailbox event への返信

詳細な protocol テストや実機検証記録は agent-talkd リポジトリの責務です。
運用環境の daemon / CLI は v0.4.0 以降を使用します。

## Phase 1: 読み取り専用 lookout

実装済みです。

- `agent-talk who` による agent 一覧
- 登録済み pane に限定した `capture-pane` 表示
- ANSI 色を保った 1.5 秒ポーリング
- スマートフォン優先 UI と desktop 2 pane layout
- installable PWA、Sumi/Washi theme

運用環境で残っている確認事項:

- LAN 内の Android Chrome で最新 UI を継続運用し、操作上の問題を洗い出す
- `home-server/systemd/` に moca-server / shoebox と同じ構成の service、
  update service、timer、update script を追加

## Phase 2: letters

実装済みです。

- Screen / Letters tab と hash による選択状態の保持
- `mobile` mailbox の初回最大 500 件取得と global event ID による差分取得
- Letters tab が表示されている間だけの 2 秒ポーリング
- 選択中 pane に絞った送受信 timeline、時刻、skill、返信元の表示
- Screen / Letters の両方から使える共通下部 dock。閉じた状態は右下の「手紙」
  launcher だけを表示し、view を切り替えず下から composer を展開
- dock の開閉と Screen / Letters 切り替えでは下書きと skill を保持し、agent
  切り替えではリセット
- close / Escape、focus 復帰、ARIA 関連付け、skill menu の keyboard 操作、
  reduced-motion 対応
- Letters は composer を持たない送受信 timeline 専用 view
- skill と本文を別 field に保つ composer
- 登録paneのruntimeに応じた `$HOME` 配下のskill検出と送信時の照合
- 空白本文の拒否、UTF-8 16 KiB 制限、送信中の二重送信防止
- 送信失敗時の本文保持と、成功直後の履歴更新
- 本文を `agent-talk send` の stdin へ渡す構造

agent terrace は mailbox の内部保存形式を読みません。agent-talkd の
`mailbox-list-v1` schema を公開契約として厳格に検証します。

UI/API 契約:

- API payload は `{agent, skill, body}` とする。
- UI では skill を chip または選択状態として見せ、textarea 本文とは別の
  `skill` field として送る。
- textarea に `/deliver` や `$deliver` など runtime 固有 prefix を混ぜない。
  prefix は agent-talkd が宛先に応じて付与する。
- skill 一覧は `GET /api/agents/{pane}/skills` で取得する。Claudeは
  `~/.claude/skills`、Codexは共通の `~/.agents/skills` と固有の
  `~/.codex/skills` を対象とし、`SKILL.md` がある安全な名前だけを返す。

運用環境で残っている確認事項:

- LAN 内の Android Chrome で共通 letter dock を含む Screen / Letters を
  継続運用し、操作上の問題を洗い出す
- `5002` を agent terrace の運用 port として service / firewall 設定へ反映する

LAN 公開では `AGENT_TERRACE_ADDR=0.0.0.0:5002` を使用します。インターネット
側へは公開せず、必要な場合だけ localhost と Tailscale Serve の構成へ切り替えます。

## Phase 3+

- ntfy などによる完了 push 通知
- 必要性が確認できた場合の Tauri 2.0 wrapper
- 運用観測後、external `agent-talk` command の timeout / kill、API の同時実行
  数制限、stdout size 制限が必要かを検討

最後の可用性対策は、localhost / tailnet 内で提供する Phase 2 の blocker には
しません。
