# ラベリング・ルーブリック v1 - Phase 1 二者評価 (DRAFT)

Author: ktrysmt
Date: 2026-MM-DD (promote 時に確定)
Project: cntrdct Track A (crates.io 上位 1000 件の実証研究)
Supersedes: prereg/2026-05-04-labelling-rubric-v0.md (osf-prereg-rubric-2026-05-04)
Status: DRAFT。`research/projects/A_1000_crate/rubric-v1-draft.md` で起草。
プロモート時に `prereg/<YYYY-MM-DD>-labelling-rubric-v1.md` に移し、
最初の Phase 1 ラベル行が非空になった時点で凍結する。
v0 と同様、以後の修正は v2 ファイルとして起こす。

## 0. 本ドラフトの未確定事項

プロモート前にユーザー側で次を確定する必要がある:

- §7 タイブレーク規約: 3 段階案 (ブラインド → 議論調停 → 第 3 レーター裁定) の
  採否、もしくは代替案 (例: 多数決のみ / シニア裁定のみ / 不一致は Uncertain 確定)。
- §8 第 2 レーターの所属: cntrdct 非所属の具体名 / 募集条件。
- §9 第 3 レーターの選出: 第 2 レーターと別の非所属者を充てるか、
  プロジェクト著者がタイブレーカに就くか。
- 日付: ヘッダの `Date:` と filename の `<YYYY-MM-DD>` 部分。
- §2 で参照する `build_phase1_csv.py` 出力スキーマ拡張の要否
  (現行は consensus 系列の列を出さない。v1 が要求するなら別 commit で追加)。

採用された方針はそのまま本文に反映し、本 §0 は promote 時に削除する。

## 1. この文書の目的

Phase 1 (上位 1000 crate、検出器あたり 30-50 サンプル、合計 200-300 件) は
独立な 2 名の人間評価者によって評価される。Phase 0 単独評価で支配的であった
著者バイアスは構造的に除去されるが、レーター間の判定基準ドリフトと
アンカーバイアスという別種のリスクが浮上する。本ルーブリックは、
いかなるラベル付けより前に判定規則・評価者構成・不一致調停プロトコルを固定し、
次を担保する。

- 同一 finding に対し、第 1 レーターと第 2 レーターは独立に同一基準を適用する。
- 一致度を Cohen's κ で公開可能な定量指標として報告する (受入閾値: κ ≥ 0.6)。
- 不一致は事前合意された段階的調停プロトコルで解決し、
  かつその過程が監査可能な形で残る。
- v0 で評価済みの Phase 0 データは Phase 1 へ流入させない (異なる評価者構成の
  サンプルを混ぜると κ の解釈が壊れる)。

## 2. 入力と出力

入力 (パイプライン上流):

- コーパス: 上位 1000 crate (db_dump 由来の lifetime download ranking)。
- スキャン結果: `cntrdct scan` の `findings.json`。
- サンプル: `cntrdct-research stratified-sample --findings F --corpus-root R
  --per-detector 30 --max-per-crate 3 --seed 0 --out sample.json`
  によって detector × crate の二軸で層化サンプルした JSON。
- ラベリング用 CSV: `python3 research/projects/A_1000_crate/scripts/build_phase1_csv.py
  sample.json --blind-out phase1-labels.csv --context-out phase1-context.json
  --corpus-root R` で生成。
- アンカーバイアス回避のため `phase1-context.json` (anchor 列を保持) は
  ラベリング中はレーターに開示しない。§6 を参照。

評価作業中のファイル:

- `phase1-labels.csv` ラウンド 1 終了時点で `rater1_label` / `rater2_label`
  および `rater1_rubric` / `rater2_rubric` / `rater1_notes` / `rater2_notes`
  が埋まっている。
- ラウンド 2 / 3 が必要な場合、同一 CSV に拡張列
  `consensus_label`、`consensus_rubric`、`consensus_notes`、`tiebreak_rater`、
  `round` (= 1 / 2 / 3) を手動追加する。
  build_phase1_csv.py は v1 確定後にこれらの列を空欄出力するよう拡張する想定
  (§0 参照)。

最終出力:

- `phase1-labels.csv` (全行 `rater1_label`、`rater2_label` 非空、
  該当行は `consensus_label` 非空)。
- `phase1-kappa-summary.md` (`phase1_kappa_wilson.py phase1-labels.csv`
  で生成。受入閾値判定はここを参照する)。
- `phase1-disagreements.md` (ラウンド 2 / 3 の議事録。各不一致 finding に対し
  両レーターの初期主張、議論で参照した周辺コンテキスト、合意ラベル、
  合意理由を 3-6 行で記録)。
- `phase1-precision-summary.md` (`consensus_label` を ground truth とした
  検出器別精度 + Wilson 区間)。

評価中、レーターは `cntrdct scan` の生 JSON、`findings.json`、
`phase1-context.json`、検出器の元ソースコードを開かない。
評価世界は `phase1-labels.csv` のレーター割当列と評価対象 crate の
ソースコードのみとする。

## 3. ラベル値

v0 §3 と同一: `TP`、`FP`、`Uncertain`。値の意味、大文字小文字の取り扱い、
下流 grep 慣習も同一。本節は v0 から変更なし。

## 4. 評価プロトコル

ラウンド 1 (ブラインド独立評価):

1. レーター 1・レーター 2 はそれぞれ独立に `phase1-labels.csv` を開く。
2. 行順 (`detector_id` 昇順、`citation_keys` 昇順) に評価する。
3. `primary_excerpt` 相当の情報は CSV の `file` + `line` 列を頼りに
   評価対象 crate のソースコードから直接読む。
4. §5 の検出器別ルーブリックを適用し、自身の rater 列に
   `TP` / `FP` / `Uncertain`、該当条項 ID、任意の自由記述を記入する。
5. 時間予算: finding 1 件あたり 90 秒 (v0 の 60 秒から 30 秒延長。
   crate ソース直接読みコストを反映)。超過時は `Uncertain` + `U-time`。
6. レーター 1・レーター 2 は相互の rater 列・notes を見ない。
   独立評価の整合性を担保するため、相手列をスプレッドシートで隠す
   (LibreOffice / Excel の列非表示) か、各レーターが自身の列のみを
   含む CSV 抜粋ファイルを使用する。

ラウンド 1 終了条件: 全行で両 rater 列が非空。
レーター双方が完了報告を出した時点で、第 3 者 (プロジェクト著者) が
`phase1_kappa_wilson.py` で κ を計算する。

ラウンド 2 (調停): §7 を参照。
ラウンド 3 (タイブレーク): §7 を参照。

## 5. 検出器別ルーブリック

v0 §5.1 - §5.5 をそのまま継承する。Phase 1 では 5 つの検出器
(clone-drift / arg-swap / comment-code / unreachable-after-terminator /
config-interaction) すべてが finding を発火する想定で、v0 で「草案」だった
arg-swap (5.4) と config-interaction (5.5) も本評価で運用する。

Phase 1 で v0 草案条項に不足が見つかった場合、当該条項を本ファイルに
転記の上で追記し、追記理由を本節末尾の「v0 からの条項追加」サブセクションに
記録する。条項 ID は v0 と継続な番号付け (clone-drift TP-3 など) を用いる。

評価世界の制約 (v0 §4 末尾と同じ):

- レーターは検出器メッセージや rank_score を見てはならない (§6)。
- レーターは自身の cntrdct プロジェクトとの関わりを評価判断に持ち込んではならない。
  特に第 1 レーター (cntrdct 著者・関係者を含む可能性) は、
  「自分が書いた検出器だから TP に違いない」という思い込みを意識的に抑制する。
  この抑制は §6 のアンカーバイアス対策では構造的に防げないため、
  ルール 5.x の文言適用に厳密に依拠する。

## 6. アンカーバイアス対策

Phase 0 v0 §6 の「rank_score / message / anomaly_class 列を右端に移して
スプレッドシートで隠す」運用は v1 で次のように強化する。

- `phase1-labels.csv` (build_phase1_csv.py の `--blind-out` 出力) は
  そもそも anchor 列を含まない。物理的にレーターの目に入らない。
- anchor 情報は `phase1-context.json` に id 列でリンクされた sidecar として
  存在し、ラウンド 2 (調停) 開始まで開示禁止。
- レーターは sidecar の存在を知らされるが、内容は見ない。
  sidecar ファイルへのアクセス権はラウンド 2 進行係 (プロジェクト著者) のみが持つ。
- detector_id 列はレーターに開示する (検出器別ルーブリックを引くために必須)。
  これは v0 §6 と同じく不可避な漏洩で、軽減策はルーブリック厳格適用のみ。

校正ラウンド (任意、ただし強く推奨):

- レーター 2 が cntrdct 非所属である場合、本評価開始前に校正セッションを
  1 回行う。両レーターが共通の 10 件 (Phase 0 から流用しない別サンプル)
  を blind 評価し、結果を突き合わせる。一致しなかった行のルーブリック適用に
  ついて 30 分以内で議論し、共通理解を確認する。
- 校正セッションの所要時間と一致率は `phase1-disagreements.md` 冒頭に
  記録する。校正ラウンドは κ 計算には算入しない。

## 7. 不一致時の調停プロトコル (タイブレーク私案)

私案 = 3 段階: ブラインド → 議論調停 → 第 3 レーター裁定。
ユーザー判断で代替案に差し替え可。

ラウンド 2 (議論調停):

1. ラウンド 1 終了後、`rater1_label != rater2_label` の行をすべて抽出する
   (`Uncertain` 一致は不一致扱いしない)。
2. ラウンド 2 進行係が抽出行に対し `phase1-context.json` の anchor 情報を
   両レーターに開示する。両レーターは初めて anchor 情報を見る。
3. 両レーターは finding 1 件あたり最大 5 分で議論し、`consensus_label` を
   `TP` / `FP` / `Uncertain` のいずれかに合意する。
4. 合意した行に `consensus_label`、`consensus_rubric`、`consensus_notes`
   (議論サマリ 1-3 行) を記入。`round` 列に `2` を記入。
5. 議論しても合意できない行は ラウンド 3 に持ち越す
   (`consensus_label` を空のまま残す)。

ラウンド 3 (第 3 レーター裁定):

6. ラウンド 2 でも合意に至らなかった行のみが対象。
7. 第 3 レーター (§8 で指名) が `phase1-context.json` の anchor 情報および
   ラウンド 2 議論メモ (`phase1-disagreements.md` のドラフト) を読み、
   独立に `consensus_label` を決定する。
8. 第 3 レーターの裁定は最終とする。`consensus_label`、`consensus_rubric`
   (条項 ID)、`tiebreak_rater` (第 3 レーター ID)、`tiebreak_rationale`
   (3-5 行) を記入。`round` 列に `3` を記入。

調停記録:

- ラウンド 2 で合意した行の議論サマリ、ラウンド 3 で裁定された行の rationale
  はすべて `phase1-disagreements.md` に finding ID 順で時系列追記する。
  これは β 論文の Replication Package に同梱される一次資料となる。

## 8. 評価者構成

レーター 1: cntrdct プロジェクト著者または同等関与者。
v0 と同じ知識基盤を持つことを要求する (検出器ルーブリックの内部構造に
精通している)。

レーター 2: cntrdct プロジェクト非所属者を 1 名指名する。Rust 経験 2 年以上、
静的解析ツールに対する一般的理解を持つ Rust 開発者であることを要求する
(具体名はプロモート時に確定)。

第 3 レーター (タイブレーカ): レーター 1 とレーター 2 のいずれとも別の人物。
理想的には cntrdct 非所属者 (レーター 2 の独立性を担保する目的と整合)
だが、確保できない場合はプロジェクト著者がタイブレーカに就くことを許容する。
許容した場合は `phase1-kappa-summary.md` のメタデータに
「タイブレーカは著者」と明記し、その limitation を β 論文 §Threats to
Validity に記録する。

## 9. インターレーター一致

主要指標: 全 finding について `rater1_label`・`rater2_label` を入力に
Cohen's κ を計算する。Uncertain を除外して TP / FP の binary κ を主指標とし、
3-class κ (Uncertain を独立カテゴリに含む) を補助指標として併報する。

計算ツール: `python3 research/projects/A_1000_crate/scripts/phase1_kappa_wilson.py
phase1-labels.csv` (実装済み、unittest 13 件パス)。出力 Markdown を
`phase1-kappa-summary.md` として保存する。

受入閾値: 主指標 κ ≥ 0.6。

閾値未達時の対応:

- κ ∈ [0.4, 0.6): ルーブリックの個別条項に解釈差が残っている可能性が高い。
  最も不一致が集中した検出器の条項を v1.x として追記し、再評価する。
  追加評価は (10 件 + 不一致集中検出器の全件) を最低単位とする。
- κ < 0.4: 評価者構成またはルーブリック設計の根本問題。Phase 1 を中断し、
  Phase 0 + 1 のメソドロジを再検討する。再開時は v2 ルーブリックを起こす。

## 10. 検出器別精度

ground truth: ラウンド 2 / 3 終了後の `consensus_label`。
非 `consensus_label` 行 (Uncertain や両レーター完全 Uncertain 合意) は
精度分母に含めない。

計算: `phase1_kappa_wilson.py` の Wilson 95 % CI ロジックを `consensus_label`
基準で再利用する (上記スクリプトの内部関数 `wilson_ci` を import した
別スクリプト `phase1_precision.py` を用意するか、
`phase1_kappa_wilson.py` に `--precision-mode` フラグを追加するかは
v1 確定後の実装判断)。

報告:

- 検出器ごとに point estimate、Wilson lower / upper を `phase1-precision-summary.md`
  にテーブル形式で記録。
- 偽陽性の失敗モード分類は本ルーブリックでは規定せず、別途
  `research/projects/A_1000_crate/failure-modes-v1.md`
  を起草するタスクに分離する (Phase 1 spec 範囲外、本 v1 では §0 にも
  含めない後続課題)。

## 11. 凍結成果物

`phase1-labels.csv` のいずれか 1 行で `rater1_label` または `rater2_label`
が非空になった時点で、本ルーブリックを凍結する。
以後の規則変更は新規ファイル
(`prereg/<YYYY-MM-DD>-labelling-rubric-v1.<x>.md` または `v2.md`) として作成し、
v1 規則が不十分だった理由を差分セクションで説明する。
v1 のラベル付き CSV は変更せず保存し、v1 規則由来バイアスを監査可能にする。

## 12. v0 からの差分要約

| 領域 | v0 (Phase 0) | v1 (Phase 1) |
|---|---|---|
| 評価者数 | 1 | 2 + 必要時 1 (タイブレーカ) |
| 一致指標 | 自己整合性 (id 1-10 の 90 % 一致) | Cohen's κ ≥ 0.6 |
| 不一致調停 | 該当なし | §7 の 3 段階プロトコル |
| アンカーバイアス対策 | 列右端配置 + 表示非表示 | 物理的に anchor 列を含まない CSV + sidecar |
| サンプル | per-detector cap 30、seed 42 | detector × crate 二軸 stratified、per-detector 30、max-per-crate 3、seed 0 |
| 時間予算 | 60 秒/件 | 90 秒/件 |
| 評価者所属 | 著者単独 | 著者 + 非所属 1 名 |
| 検出器条項 (5.1-5.5) | clone-drift, comment-code, unreachable-after-terminator は確定; arg-swap, config-interaction は草案 | v0 をすべて運用条項に昇格、Phase 1 で不足が出れば §5 末尾に追記 |
| 失敗モード分類 | §5 各条項の `notes` で記録 | 別ドキュメントに分離 (本 v1 範囲外) |

## 13. プロモート手順

1. 本ファイル (`research/projects/A_1000_crate/rubric-v1-draft.md`) の §0 を
   ユーザー判断で確定 (タイブレーク採用案、第 2 / 第 3 レーター指名、日付)。
2. 確定版を `prereg/<YYYY-MM-DD>-labelling-rubric-v1.md` にコピーし、
   §0 を削除。
3. 必要に応じて `prereg/<YYYY-MM-DD>-osf-prereg-phase1-addendum.md` を別途起草し、
   parent prereg と本 v1 の関係 (κ ≥ 0.6 の根拠、第 3 レーター裁定の限界) を
   追記する。
4. `crates/cli/tests/prereg_consistency.rs` の skip フィルタは
   `-rubric-` を既にマッチしているため追加修正不要 (確認済み)。
5. v0 ファイルは削除しない。Phase 0 v0 データの監査可能性を保つため、
   `prereg/2026-05-04-labelling-rubric-v0.md` は永続的に残す。
6. 本ドラフトファイル (`rubric-v1-draft.md`) はプロモート完了後、
   コミット履歴から復元可能であることを根拠に削除して構わない。
   ただし、v1 ドラフト段階の議論を追跡したい場合は残置も可。
