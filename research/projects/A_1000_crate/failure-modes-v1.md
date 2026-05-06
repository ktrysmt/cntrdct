# FP 失敗モード分類 v1 (DRAFT)

Author: ktrysmt
Date: 2026-MM-DD (promote 時に確定)
Project: cntrdct Track A (crates.io 上位 1000 件の実証研究)
Status: DRAFT。`research/projects/A_1000_crate/failure-modes-v1.md` で起草。
プロモート時に `prereg/<YYYY-MM-DD>-failure-modes-v1.md` に移し、
rubric v1 の凍結トリガ (`phase1-labels.csv` のいずれか 1 行で
`rater1_label` または `rater2_label` が非空) と同時に凍結する。
以後の修正は v1.x ファイルとして起こす。
Companion: `research/projects/A_1000_crate/rubric-v1-draft.md` §10
からの分離タスク。

## 1. 本ドキュメントの目的

Phase 1 の二者評価で `consensus_label = FP` と判定された finding に対し、
偽陽性が発生した原因を controlled vocabulary で記録する。これにより β
論文 §Threats to Validity および §Discussion で次が定量議論可能になる。

- 検出器ごとに支配的な FP 原因は何か。
- スケール (上位 100 → 上位 1000) で v0 が想定しなかった新規 FP モードが
  どれほど現れたか。
- 各 FP モードが将来の検出器改善 (たとえば `arg-swap` への可換 callee
  ホワイトリスト導入) でどれほど削減されうるか。

範囲外:

- TP の正当化条項分類は v0 §5 の TP-1..TP-3 / rubric v1 §5 を再利用する
  ため、本ドキュメントでは扱わない。
- Uncertain の理由分類は rubric の `U-time` / `U-tie` / `U-1..U-3`
  / `U-time` で十分カバーされており、本 v1 では追加しない。
- モード頻度集計スクリプトは v1.1 への deferred タスク。本 v1 は
  controlled vocabulary とアジュディケータ運用規則のみを規定する。

## 2. スキーマ拡張

### 2.1 CSV 列

`phase1-labels.csv` の rubric v1 §2 拡張列に加えて、本 v1 では次の列を
追加する。

| 列 | 値域 | 記入タイミング |
|---|---|---|
| `failure_mode` | §3 で各検出器について定義する controlled vocabulary、または `other`、または空文字列 | アジュディケータがラウンド 2 / 3 終了後、`consensus_label = FP` の行に対してのみ記入する |
| `failure_mode_notes` | 任意の自由記述 (1-3 行) | `failure_mode = other` の場合は必須。それ以外は任意 |

`failure_mode` は次の場合に空文字列とする。

- `consensus_label != FP` (TP / Uncertain / 空)。
- ラウンド 1 終了直後でアジュディケーションが未完了。

`build_phase1_csv.py` は v1 確定後にこれらの列を空欄出力するよう拡張する
(rubric v1 §0 で言及されている拡張と同じ commit で対応する想定)。

### 2.2 controlled vocabulary の命名

- すべて小文字英語のハイフン区切り (例: `boilerplate-shape-only`)。
- 検出器を跨いで同一意味を持つモード (例: `cross-file-context-resolved`)
  は同一 ID を再利用する。これは集計時に検出器横断のメタモード分析を
  可能にするため (v1.1 の集計スクリプト想定)。
- ID は §3 のテーブルに列挙されたものに限定する。新規モードを追加する
  場合は v1.x ファイルを起こす (§5)。

### 2.3 決定規則

複数モードが該当する場合のタイブレーク:

1. より具体的なモードを優先する。例: `auto-generated-clone` は
   `boilerplate-shape-only` を内包しうるが、自動生成由来であれば
   前者を選ぶ。
2. v0 §5.x で明示的に存在した FP-N に対応するモードを優先する
   (例: `metadata-only-drift` は v0 FP-3 と一対一対応)。
3. それでも決まらない場合は `other` + `failure_mode_notes` で記録し、
   タイブレーク困難理由を残す。これは v1.x で正式モードに昇格させる
   候補となる。

該当モードがない場合:

- `failure_mode = other`、`failure_mode_notes` に最低 2 行で
  「なぜ既存モードが該当しないか」「分類提案 (新規モード名候補)」を記入する。
- `other` の頻度はラウンド 3 終了時にメタ集計し、5 件以上の同種 `other`
  記述が集まったら v1.x を起こして正式モードに昇格させる。

## 3. 検出器別 failure mode タクソノミ

各検出器について 4-5 モードを定義する。各モードは「定義」「典型例」
「v0 由来か新規か」を持つ。検出器の TP / Uncertain 条項は本ドキュメント
の対象外であり、rubric v1 §5 (および継承元の v0 §5) を参照する。

### 3.1 clone-drift

| failure_mode | 定義 | v0 対応 |
|---|---|---|
| `boilerplate-shape-only` | 主対象と関連対象が短い `match` arm、`From` 実装、trait の forward 委譲などで構文形のみ類似し、本体の概念的役割が異なる | v0 FP-1 |
| `type-or-cfg-justified-drift` | ドリフトが型・generics・lifetime・`cfg`・feature gate の差で完全に説明でき、ロジック欠落がない | v0 FP-2 |
| `metadata-only-drift` | ドリフトが doc comment、属性 (`#[inline]`、`#[cold]`)、可視性修飾子のみに存在し、実行本体は同一または自明な改名のみ | v0 FP-3 |
| `auto-generated-clone` | 主対象または関連対象の少なくとも 1 つが自動生成コード (build.rs 出力、proc-macro 展開、derive 展開) | v0 FP-4 |
| `cross-file-context-resolved` | アジュディケータが `phase1-context.json` および周辺 crate (呼び出し元、trait 定義、テスト期待値) を読んだ結果、ドリフトが正当な意図的差分であると判断できた | NEW |

典型例:

- `boilerplate-shape-only`: serde の `Visitor` 実装で `visit_str` と
  `visit_bytes` が短く類似しているが、扱う入力種別が違うため別ロジックを
  期待することは妥当。
- `cross-file-context-resolved`: setter グループのうち 1 つだけが追加
  バリデーションを行うが、別ファイルの builder API がそのバリデーションを
  呼び出し前に必須化していたため、setter 側の差は正しい設計。

### 3.2 arg-swap

| failure_mode | 定義 | v0 対応 |
|---|---|---|
| `type-distinct-positions` | 引数型が呼び出し側と被呼び出し側で異なるため、位置取り違えが型システム上不可能 | v0 §5.4 草案 FP |
| `commutative-callee` | callee が当該 2 引数について可換 (`min`、`max`、`std::cmp::Ord::cmp`、集合和、加算など) であり、入れ替えても意味が変わらない | v0 §5.4 草案 FP (`min(a, b)` 例) |
| `builder-positional-convention` | builder パターンや座標系コンストラクタ (`Point::new(x, y)`、`Range { start, end }`) で、識別子マッチが偶発的かつ呼び出し慣習が確立している | NEW |
| `cross-file-context-resolved` | アジュディケータが呼び出し先のドキュメント・テスト・既存呼び出しを参照した結果、現引数順が意図的であると確認できた | NEW (clone-drift と共有) |

典型例:

- `commutative-callee`: `cmp::min(local_min, candidate)` で 2 引数の
  ローカル名が callee の `(a, b)` と部分一致するが、`min` は可換。
- `builder-positional-convention`: `Vec2::new(x, y)` を `Vec2::new(y, x)`
  と書いた、と検出器が見えても、呼び出し側のローカル変数名が
  `pos_y, pos_x` の順で宣言されているのは UI 慣習であり swap ではない。

### 3.3 comment-code

| failure_mode | 定義 | v0 対応 |
|---|---|---|
| `higher-abstract-intent` | コメントが関数全体の意図・不変条件を述べ、可視コードはその一分岐に過ぎず両者は整合している | v0 FP-1 |
| `future-work-marker` | コメントが TODO / FIXME / NOTE / XXX の将来作業メモであり、現在挙動の主張ではない | v0 FP-2 |
| `doctest-divergence` | コメントが doctest (` ```rust ` ブロック) で、見かけの不一致は doctest 内例示コードと実装本体の差。doctest は別実行されるため矛盾ではない | v0 FP-3 |
| `translation-ambiguity` | 非英語コメント (日本語、中国語、ロシア語など) でアジュディケータが翻訳ニュアンス一致に確信を持てず、ベストエフォート訳の上で FP と判断した | v0 FP-4 |
| `stale-but-harmless` | コメントは確かに陳腐化しているが、不一致が純粋に表層的 (リネーム済み引数名への doc 言及など) で、読者を誤誘導するリスクがゼロ | NEW |

典型例:

- `stale-but-harmless`: 関数 doc が `// returns Result<T, Error>` と
  書いてあるが、実際の戻り型は `Result<T, MyError>` (型エイリアスから
  具象型へのリネーム)。意味は等価で、読者は IDE で実際の型を即座に
  確認できる。
- `higher-abstract-intent`: 関数頭の `// returns the parsed value or
  error` に対し、可視抜粋では error 経路のみが見えている。

### 3.4 unreachable-after-terminator

| failure_mode | 定義 | v0 対応 |
|---|---|---|
| `cfg-gated-divergence` | 終端が `cfg(...)` または feature flag で分岐されており、「到達不能」コードは代替コンパイル構成で実行される | v0 FP-1 |
| `macro-internal-divergence` | 終端がマクロ展開内部にあり、「到達不能」コードは設計どおりのマクロ else 分岐 | v0 FP-2 |
| `non-divergent-loop` | 見かけ上の終端が `loop { ... }` だが、本体に到達可能な `break` または `return` を含み、loop は実際には発散していない | v0 FP-3 |
| `wrong-control-flow-block` | 終端後コードが同一制御フローブロックではなく、`match` の別 arm、別 `if` 分岐、別関数にある (検出器のスコープ判定誤り) | v0 FP-4 |
| `runtime-conditional-divergence` | 終端が実行時条件 (`if FEATURE_FLAG.load() { return; }`、環境変数チェック、CLI フラグ) でガードされており、検出器がそれを常に成立すると扱った | NEW |

典型例:

- `runtime-conditional-divergence`: `if std::env::var("DEBUG").is_ok()
  { return; }` の後続コードを「到達不能」と判定したが、`DEBUG` 環境変数
  がない通常実行では到達する。

### 3.5 config-interaction

| failure_mode | 定義 | v0 対応 |
|---|---|---|
| `non-exclusive-on-tier1` | 述語が見かけ上排他に見えるが、tier-1 target triple の少なくとも 1 つで両方が成立する (例: `target_os = "linux"` と `target_arch = "x86_64"` を排他と誤解した場合) | v0 §5.5 草案 FP |
| `complementary-by-design` | 述語は確かに排他だが、両方とも実装が別モジュール / 別 feature path で確実に提供されており、ある実行時構成で実装不在になることはない | NEW |
| `build-script-resolved` | `cfg` 述語の組合せが `build.rs` によって動的に解決され、ビルド時に常に少なくとも 1 つの分岐が選ばれる構成になっている | NEW |
| `target-spec-mismatch-not-bug` | 述語が異なる CPU アーキテクチャ・OS 向けに別実装を提供しており、tier-1 で実装不在になる組合せがないことをアジュディケータが target-spec から確認できた | NEW |

典型例:

- `complementary-by-design`: `#[cfg(unix)]` と `#[cfg(windows)]` の
  片側ずつに別実装があり、両 OS で必ず一方が選択される。検出器は
  「`other_os` ターゲットで実装不在」と発火するが、当該 crate は
  unix / windows のみを target とする宣言が `Cargo.toml` にある。

## 4. アジュディケータ運用フロー

ラウンド 2 (議論調停) または ラウンド 3 (第 3 レーター裁定) で
`consensus_label = FP` を確定した直後に、同じアジュディケータが
`failure_mode` 列を埋める。これは独立タスクではない。

決定手順:

1. §3 の該当検出器テーブルを上から順に確認する。
2. 最初に該当する (definition を満たす) モードを採用する。
3. 複数該当する場合、§2.3 のタイブレーク順序を適用する。
4. 該当なしなら `other` + `failure_mode_notes` (§2.3)。
5. 1 finding あたり目安 60 秒。超過したら `other` + 「未分類: 時間切れ」
   と記入し、後で再訪する (再訪する旨も notes に書く)。

監査記録:

- ラウンド 3 終了時点で `failure_mode = other` の行をすべて
  `phase1-disagreements.md` 末尾の「Unclassified FP」節に列挙する。
- ラウンド 3 終了時点での controlled vocabulary 別件数集計は v1.1 で
  実装する集計スクリプト (deferred、§6) の責務とする。本 v1 では
  controlled vocabulary が CSV 列に一貫して入っていることだけを
  保証すれば十分。

## 5. 凍結成果物

`phase1-labels.csv` のいずれか 1 行で `failure_mode` が非空になった
時点で、本 v1 を凍結する (rubric v1 と同期)。
以後の規則変更は新規ファイル
(`prereg/<YYYY-MM-DD>-failure-modes-v1.<x>.md` または `v2.md`) として
作成し、v1 が不十分だった理由を差分セクションで説明する。
v1 のラベル付き CSV は変更せず保存し、v1 規則由来バイアスを監査可能にする。

新規モードの追加プロセス:

1. ラウンド 3 終了後の `other` 集計で 5 件以上の同種記述があるか、
   査読プロセスで査読者が分類粒度の不足を指摘した場合に v1.x を起こす。
2. 既存モード ID の意味は変更しない (削除・再定義は v2 に持ち越す)。
   v1.x ではモードの追加と decision rule の追加のみ行う。
3. v1 で `other` だった行は v1.x 確定後に再分類する。
   再分類前後の `failure_mode` 値は両方保持し、
   `failure_mode_v1` / `failure_mode_v1x` の二列に分割する
   (β 論文 Replication Package で audit trail を保つため)。

## 6. 後続課題 (本 v1 範囲外)

- アグリゲータスクリプト `phase1_failure_modes_aggregate.py` の実装。
  入力: `phase1-labels.csv` (failure_mode 列が埋まっている)。
  出力: `phase1-failure-modes-summary.md` で検出器 × failure_mode の
  クロス集計テーブル。v1.1 で起草する。
- 検出器横断メタモード分析: `cross-file-context-resolved` のような
  共有モードについて、検出器ごとの発生率を比較する論文用の図表。
  Phase 2 (write-up) フェーズで実施。
- v0 → v1 でモード列を導入したことの threats to validity への影響。
  Phase 0 データには `failure_mode` 列がないため、v0/v1 統合分析は
  failure_mode を欠損として扱う。これは β 論文 §Threats to Validity
  に明記する。

## 7. v0 からの差分要約

| 領域 | v0 (Phase 0) | v1 (Phase 1) |
|---|---|---|
| FP 原因記録 | 各 FP 行の `notes` 列に自由記述 | controlled vocabulary を持つ `failure_mode` 列を追加 |
| 検出器カバレッジ | clone-drift / comment-code / unreachable-after-terminator のみ確定 (v0 §5.1-§5.3)。arg-swap / config-interaction は草案で finding 未発火 | 5 検出器すべてに 4-5 モードを定義 |
| 集計 | なし (notes は読むだけ) | v1.1 で集計スクリプト導入予定 (本 v1 範囲外) |
| 監査 | `notes` テキストの目視 | `failure_mode` の controlled vocabulary 一致と `other` 件数による |

## 8. プロモート手順

1. 本ファイル (`research/projects/A_1000_crate/failure-modes-v1.md`) の
   §0 相当の未確定事項はないが、rubric v1 と同期で promote する。
   先行して promote しない (rubric v1 が frozen ファイルとして prereg/
   に入る前に本 v1 を frozen にしても、参照先が DRAFT のままで整合
   しなくなるため)。
2. rubric v1 の promote と同じ commit、または直後の commit で
   `prereg/<YYYY-MM-DD>-failure-modes-v1.md` にコピーする。
   `Date:` ヘッダと filename 部の `<YYYY-MM-DD>` は rubric v1 の
   日付と揃える。
3. `crates/cli/tests/prereg_consistency.rs` の skip フィルタは
   現状 `-rubric-` のみマッチさせている。`-failure-modes-` は
   OSF schema を持たないため、テストが拾わないよう skip フィルタ追加が
   必要 (技術側 commit、研究側からは PR 提案のみ)。確認事項として
   prereg promote PR に明記する。
4. v0 ファイル (もし将来作られた場合) は削除しない。本 v1 の audit
   trail を保つため。
5. 本ドラフトファイル (`failure-modes-v1.md`) はプロモート完了後、
   コミット履歴から復元可能であることを根拠に削除して構わない。
   ただし、v1 ドラフト段階の議論を追跡したい場合は残置も可。
