# Roadmap

## 前提: agent-talkd

agent terrace の送信機能は、agent-talkd 側で実装・検証済みの次の契約を
前提にします。

- fail-closed な `send-v2` protocol
- `--from` の表示ラベルと返信可能な pane identity の分離
- 外部送信元、skill、宛先別 slash/dollar syntax の daemon 側検証
- 本文を journal のみに保存し、固定 bell だけを tmux へ届ける構造

詳細な protocol テストや実機検証記録は agent-talkd リポジトリの責務です。

## Phase 1: 読み取り専用 lookout

実装済みです。

- `agent-talk who` による agent 一覧
- 登録済み pane に限定した `capture-pane` 表示
- ANSI 色を保った 1.5 秒ポーリング
- スマートフォン優先 UI と desktop 2 pane layout
- installable PWA、Sumi/Washi theme

運用環境で残っている確認事項:

- `tailscale serve --bg 3000` の有効化
- tailnet 参加済み Android Chrome からの実機確認とホーム画面追加
- `home-server/systemd/` に moca-server / shoebox と同じ構成の service、
  update service、timer、update script を追加

## Phase 2: letters

journal reader、送信フォーム、skills 選択は一体で実装します。着手前に次を
決定する必要があります。

- 必要な時系列取得範囲
- pane と message の認可境界
- agent-talkd に安定した read-only 一覧 API を追加するか、journal 形式を
  公開契約にするか

agent terrace が journal の内部形式を暗黙に直接 parse する設計にはしません。

確定済みの UI/API 契約:

- API payload は `{agent, skill, body}` とする。
- UI では skill を chip または選択状態として見せ、textarea 本文とは別の
  `skill` field として送る。
- textarea に `/deliver` や `$deliver` など runtime 固有 prefix を混ぜない。
  prefix は agent-talkd が宛先に応じて付与する。
- skill 一覧は当面 `deliver`、`commit` などの静的 allowlist でよい。

## Phase 3+

- ntfy などによる完了 push 通知
- Claude Code / Codex session log の読み取り専用 viewer
- 必要性が確認できた場合の Tauri 2.0 wrapper
