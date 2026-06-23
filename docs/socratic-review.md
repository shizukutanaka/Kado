# Socratic Review v2 — Plan.md 吟味議事録

> 日付: 2026-06-11 ｜ 対象: Plan.md（Kado AI-First Geometry Engine, 初版）
> 目的: 初版Planが「すでにソクラテス問答を済ませた」構えで書かれているため、**まだ問うていない前提**を内側から突き、論理矛盾・未吟味の横断的決定・誇張・測定不能なKPIを摘出して改善する。
> 原則: 外部の好みではなく、文書自身の制約・主張同士の矛盾のみを根拠に問う。

---

## 問1 — 精度のパラドックス
**問**: 案A（メッシュCSG）を「数値破綻・精度が頂点座標そのもの」で却下したが、採用した案Cの最終成果物も適応的dual contouringで抽出した**メッシュ**ではないか。何を本当に得たのか。

**論点**:
- ブーリアンが min/max で失敗しないのは**場（field）の代数**の話。納品物のメッシュは依然テッセレーション近似。
- 「製造可能ファイル」の出力精度は案Aと同じく「メッシュ近似公差」に帰着する。

**結論 → 反映**:
- 案Cの本質的利得は「精度」ではなく「**ブーリアン演算の無破綻性**」一点と明記。
- §2比較表の「寸法精度」行を「プリミティブ＝解析厳密／出力メッシュ＝公差付き近似」に修正。
- §2末尾に「精度に関する正直な契約」節を追加。リスク表に寸法精度の明示項を追加。

## 問2 — 「正本」の二重定義（論理矛盾）
**問**: §1は「code-as-truth」、§3は「特徴履歴を正本とするスクリプト形式」。スクリプトと特徴履歴のどちらが正本か。MCPに `run_script` と `create/edit/delete_feature` が併存し、どちらを正本とするかでツール意味論と `get_scene` 差分の定義が変わる。

**結論 → 反映**:
- **唯一の正本 = スクリプト（DSLソース）** に確定。特徴履歴・シーングラフは構文木の決定的射影。
- `create/edit/delete_feature` は**構文木操作のシンタックスシュガー**（最終的に正本スクリプトを書換）と定義。
- §1に「正本の定義」節、Phase 2/4 と付議事項④に反映。

## 問3 — 最も横断的な決定（実装言語）にADRがない
**問**: カーネル(ADR-001)・GUI(ADR-002)にはADRがあるが、単一バイナリ・SIMD・決定性・サンドボックス・自前ラスタライザを同時成立させる**実装言語/ランタイム**にADRがない。これは全てを規定する最横断的決定では。

**結論 → 反映**:
- **ADR-003（実装言語・ランタイム）を新設し Phase 0 DoD に追加**。未決のままPhase 1へ進まないことをゲート化。付議事項⑤。

## 問4 — 「依存ゼロ」は目的か手段か（概念の混同）
**問**: 「依存ゼロ単一バイナリ」を制約に挙げるが、「単一バイナリ配布」と「第三者コード不使用」は別物。静的リンクすれば第三者ライブラリを使っても単一バイナリは達成できる。文字通りの「依存ゼロ」はラスタライザ・各種ライタ・自前libmまで自作を強い、XL→XL++に膨張させる。守るべき本質（外部送信ゼロ・1コマンド導入）に第三者行不使用は寄与しない。

**結論 → 反映**:
- 制約を「依存ゼロ」→「**実行時外部依存ゼロ＝単一自己完結バイナリ（静的リンク許容）**」に再定義。
- §定義の制約⑤、リスク表（工数）、付議事項⑥に反映。何を自作必須／何をベンダリング可とするかはSPECで線引き。

## 問5 — 決定性100% と SIMD/並列/クロスプラットフォームの両立
**問**: KPI「決定性100%」と性能根拠「SIMD/並列向き」は、FP縮約順序・FMA・並列リダクション・libm差というビット非同一要因と緊張する。両立は自明か。「同一入力→バイト同一」は同一arch内かクロスプラットフォームか。

**結論 → 反映**:
- 両立は可能だが**無料ではない**（固定縮約順序・fast-math禁止・決定的並列リダクション・固定/自前超越関数）。
- 決定性の**範囲を限定**: 「同一リリースバイナリ・同一OS/arch内でバイト同一。クロスプラットフォームは数値同等but非ビット同一」。
- §7に「決定性の範囲」節、リスク表に両立コスト項、付議事項⑦に反映。

## 問6 — 中核仮説が最後に検証される（リーン逆行）
**問**: 製品の存在理由は「AIが無人完走できる」こと。なのにその実証DoDはPhase 4。最もリスクの高い仮説の検証が4フェーズ後で良いのか。

**結論 → 反映**:
- **Phase 0.5「垂直スパイク」を新設**。box→1ブーリアン→manifold DC→1 screenshot→最小MCP→Claude実機1往復を最短貫通し、中核仮説を早期Yes/No。後続着手前のゲート。付議事項⑧。

## 問7 — 測れないKPI と 顧客のいないfacetted STEP
**問(a)**: 「無人完走率≥80%」の分母は何か。本製品固有の評価セットはどこにも定義がない。測れないKPIは装飾。77.8→90.5%は外部研究の数字でKadoのベンチではない。
**問(b)**: facetted STEP——文書自身が「案A/Bも実質同等で決定打にならず」と書く。3MF/STLで満たせない、facetted STEPだけの受け手は誰か。

**結論 → 反映**:
- (a) **Phase 0 DoD に評価セット定義（N≥10件＋合否定義＋無人完走の操作的定義）を追加**し、KPI分母とする（docs/EVAL-SET.md）。
- (b) **facetted STEP を BACKLOG に降格**。Phase 5 を 3MF/GLB/HTML に集中。真B-rep STEP の需要確認時に再付議。スコープ膨張対策（文書自身のリスク）と整合。付議事項⑨。

## 問11（補足）— 「原理的に失敗しない」の射程
**問**: min/max は場としては失敗しないが、抽出メッシュは薄肉部・完全一致部で非多様体や不良三角形を生む。naive DCは非多様体頂点を作る。「構築時から水密性保証」は自動ではない。

**結論 → 反映**:
- 「無破綻」は**場の代数に限定**し、**メッシュ抽出の健全性は別保証**と明記。manifold dual contouring を必須化、非多様体0・水密100%を性質テストでゲート（§2・リスク表・Phase 1 DoD）。

---

## 反映サマリ
| 問 | 種別 | Plan.md への主な反映 |
|----|------|----------------------|
| 1 | 誇張の是正 | §2精度行・正直な契約節・リスク表 |
| 2 | 論理矛盾の解消 | §1正本定義・Phase2/4・付議④ |
| 3 | 未吟味の横断決定 | ADR-003新設・Phase0 DoD・付議⑤ |
| 4 | 概念混同の解消 | 制約⑤再定義・付議⑥ |
| 5 | 両立コストの明示 | §7決定性範囲・リスク表・付議⑦ |
| 6 | 検証順序の是正 | Phase 0.5新設・付議⑧ |
| 7 | 測定不能KPI/無用スコープ | 評価セット定義・STEP降格・付議⑨ |
| 11 | 射程の限定 | 場/メッシュの保証分離・Phase1 DoD |

---

# Socratic Review v3 — 実装の吟味（コード ⇄ Plan の乖離）

> 日付: 2026-06-13 ｜ 対象: Phase 0.5/2/3 の実装（`src/`）
> 目的: v2 で **Plan文書** を吟味し改善した。v3 では **実装が存在する** ので、Planの「主張」と
> コードの「実態」の乖離を内側から突く。文書ではなく**動くコード自身の矛盾**のみを根拠に問う。
> 原則: テスト可能な反例を伴う問いだけを採用し、改善は必ずテストで固定する。

## 問12 — 「正本＝スクリプト」の実装レベルでのリグレッション（問2の裏切り）
**問**: 問2は「唯一の正本＝スクリプト」と結論した。だが実装では `run_script` は SDF を評価して
**捨てて**おり（要約文字列を返すだけ）、`screenshot`/`export`/`eval`/`validate` は全て
ハードコードされた `active_scene()` を読む。MCPサーバーは状態を持たない。つまり走っている
システムでは**スクリプトではなくRust関数が事実上の正本**になっている。ツール説明文の
「set it as the active scene」はコードが履行していない嘘ではないか。

**反例**: 既定シーンでは原点は穴の中で SDF>0。`run_script` に半径3の球を渡しても、続く
`eval(0,0,0)` は依然 +0.3（デモ形状）を返し、-3.0（球の内部）を返さない。

**結論 → 反映**:
- MCPに**セッション状態 `Session { scene: Sdf }`** を導入。`run_script` が `session.scene` を
  差し替え、全ツールがこれを読むよう変更（`call_tool(&mut Session, …)`）。
- リグレッション固定テスト `run_script_updates_active_scene` を追加: run_script 後に
  `eval(0,0,0)` が -3.0 を返すことを保証。これで問2が**実行時にも**成立する。

## 問13 — 「正しさ第一」を掲げながら未検証で誤った Cone プリミティブ
**問**: torus/capsule/rounded_box には表面テストがあるのに、`Cone` には**テストが無い**。さらに
評価本体には放棄された第一案の死コード（`let _ = (k, c, outside, cap);`）が残る。式は母線方向
`(r,h)` の線への距離を測るべきところ、方向 `(h,r)` の線への距離を測っており**幾何的に誤り**。
「正しさ第一・決定性第一」のエンジンに未検証の誤プリミティブを出荷していないか。

**結論 → 反映**:
- Cone を **IQ の厳密式（"Cone - exact"）に置換**し死コードを除去。先端z=0/底面z=-h/底面半径r。
- テスト `cone_surface_and_sign` を追加: 先端=0・側面上の点=0・底面ディスク上=0・内部<0・遠方>0。
  旧実装ではこのテストは通らない（=誤りの証明）。

## 問14 — 暗黙の固定バウンディングボックスによるサイレント・クリッピング
**問**: `screenshot`/`export`/`run_script`/CLI は ±2 または ±4 のサンプリング境界を**ハードコード**
している。スクリプトが10mm角の部品を作ると無言で切り取られ、メッシュ・体積・肉厚チェックが
無意味になる。「AIが無人で完走」する製品にとって、**もっともらしいが誤った結果**を返す
サイレント・クリッピングは最悪の故障モードではないか。

**反例**: `{"op":"sphere","r":8.0}` は ±2 境界では空または欠損メッシュになる。

**結論 → 反映**:
- `Sdf::aabb()` を新設し、プリミティブ＝厳密／ブーリアン・変形＝保守的合成で**形状から境界を解析導出**。
  `Sdf::sampling_box()` が余白を足してサンプリング領域を返す。全呼出側を置換。
- 不変条件テスト `aabb_encloses_surface_samples`: AABB外の点は厳密に SDF>0（内包性）を保証。
- E2E確認: 半径8球が体積2142.1（理論 4/3·π·8³≒2144）・水密でメッシュ化されることを実機確認。
- 派生補助として `Vec3` に `Neg` を実装。

## 反映サマリ v3
| 問 | 種別 | コードへの主な反映 | 固定テスト |
|----|------|--------------------|-----------|
| 12 | 主張と実装の乖離（正本の退行） | MCP `Session` 状態・全ツールがアクティブシーン参照 | `run_script_updates_active_scene` |
| 13 | 未検証・誤実装の出荷 | Cone を厳密式に置換・死コード除去 | `cone_surface_and_sign` |
| 14 | サイレント故障（暗黙の境界） | `Sdf::aabb`/`sampling_box` で形状から境界導出 | `aabb_encloses_surface_samples` |

> 総括: v2は計画の論理を、v3は実装の誠実さを問うた。三件はいずれも「Planが約束したのに
> コードが守っていなかった」点（正本・正しさ・無人完走の信頼性）であり、テストで恒久固定した。
> テスト数 46→49。

---

# Socratic Review v4 — 約束されたセキュリティ境界の不在

> 日付: 2026-06-13 ｜ 対象: MCP書き込み・DSL実行経路・決定性KPI（`src/`）
> 目的: v3に続き「Planのリスク表が明記した防御」が**コードに実在するか**を突く。今回の焦点は
> リスク表 T（MCP書き込み）・E（DSL実行）と看板KPI（決定的出力）。
> 原則: 病的入力の反例を実際に作れる脅威だけを採用し、防御はテストで固定する。

## 問15 — リスク表Tの「パストラバーサル検査」がコードに存在しない（セキュリティ）
**問**: Plan リスク表は「MCP書き込み（T）｜プロジェクトdir限定・パストラバーサル検査・read-only既定」
と明記する。だが `tool_export` は `args["path"]` を**無検査**で `stl::write_binary` に渡す。
`{"path":"../../etc/foo.stl"}` や絶対パスでプロジェクト外へ書ける。明記した防御が**実装に皆無**では。

**反例**: `export` を path `../../escape.stl` で呼ぶと外へ書き込もうとする（拒否されない）。

**結論 → 反映**:
- `sandbox_write_path()` を新設: 絶対パス・`..`（ParentDir）・ルート/プレフィックスを拒否し、
  CWD配下の相対パスのみ許可（ファイル存在に依存せずパス構造のみで判定＝決定的）。`tool_export` に適用。
- テスト3件: 相対パス許可・各種脱出拒否・`export` ツール経路での拒否を固定。

## 問16 — リスク表Eの「リソース上限」が無く、自前パーサが落ちる（DoS）
**問**: Plan リスク表は「DSL実行（E）｜ホワイトリストAST・import禁止・**リソース上限**」と明記する。
だが `eval_scene`→`build` も自前 JSON パーサ `parse_value`→`parse_object`/`parse_array` も
**深さ無制限の再帰**。病的にネストした入力でスタックを溢れさせられる。release は `panic=abort` の
ため、不正スクリプト1通でMCPサーバープロセスごと落ちる。「リソース上限」が**どこにも無い**のでは。

**反例**: `[[[[…(数万)…]]]]` や `translate` を数百段ネストした JSON でクラッシュ。

**結論 → 反映**:
- **二重防御**: ①JSONパーサに `MAX_DEPTH=128` を `parse_value` 一点で強制（全再帰が通る）。
  ②DSL `build` に `MAX_DEPTH=64`・`MAX_NODES=50_000`・ソース `MAX_SOURCE_BYTES=1MiB` を導入。
- テスト: 過深入力がオーバーフローせず**エラーで拒否**されること、巨大ソース拒否、浅い入力は通ること。

## 問17 — 看板KPI「決定的出力」をエンドツーエンドで固定するテストが無い
**問**: ADR-003/§7の看板KPIは「同一バイナリ・同一archでバイト同一」。しかし `Mesh::from_soup` は
頂点統合に `HashMap` を使う。現状の決定性は「挿入順でインデックス付与し出力は `Vec` を走査する」
という**暗黙の前提**に依存し、独立2回の抽出が一致することを**何も保証していない**。KPIが装飾になっていないか。

**結論 → 反映**:
- 検証の結果、決定性は**現状成立している**（HashMapは検索のみ・反復順は出力に未使用）。誤りではなく
  **テスト欠落**と判定。回帰検知のため `polygonize_is_byte_deterministic` を追加: 独立2回の抽出が
  頂点ビット列・三角形インデックス列まで一致することを固定。将来のHashMap順序依存の混入を遮断する。

## 反映サマリ v4
| 問 | 種別 | コードへの主な反映 | 固定テスト |
|----|------|--------------------|-----------|
| 15 | 明記防御の不在（書込サンドボックス） | `sandbox_write_path` で `tool_export` を制限 | `sandbox_*`, `export_tool_rejects_unsafe_path` |
| 16 | 明記防御の不在（DoS/リソース上限） | パーサ深さ上限＋DSL深さ/ノード/サイズ上限 | `deeply_nested_input_is_rejected_*`, `over_deep_scene_is_rejected`, `oversized_source_is_rejected` |
| 17 | 看板KPIの未固定（テスト欠落） | 抽出の決定性を end-to-end で固定 | `polygonize_is_byte_deterministic` |

> 総括: v4は「リスク表に書いたのにコードに無い防御」を突いた。問15・16は実在する脅威（プロジェクト外
> 書込・DoSクラッシュ）で、いずれも反例を作れた。問17はKPIをテストで初めて固定した。テスト数 49→57。

---

# Socratic Review v5 — 残る攻撃面と中核保証の検証

> 日付: 2026-06-13 ｜ 対象: MCP数値引数・抽出保証・スクリプト入力検証（`src/`）
> 目的: v4でDSLネスト/書込は塞いだ。だが防御は**塞いだ経路だけ**有効。同じ「リソース上限」要件の
> 別ベクトル（数値引数）と、製品の中核技術主張（問11: 水密100%）が**本当に**検証されているかを突く。
> 原則: 反例を作れる脅威のみ採用。保証は形状バッテリで性質テストし、合否を事実で示す。

## 問18 — v4が塞いだのはDSLネストだけ。数値引数による OOM/panic は素通り（DoS）
**問**: 問16は JSON ネスト深さとノード数に上限を入れた。だが `polygonize` は `(res+1)^3` 個の f64 を
確保し、`res` は MCPツール (`export`/`run_script`/`validate`) の引数から**無境界**で来る。
`{"resolution": 5000}` で約1TB確保 → 即OOM。`{"resolution": 0}` は `assert!(res>=1)` で panic
（release=abort でサーバー停止）。`screenshot` の `width`/`height` も同様に無境界。問16の防御は
**この経路に届いていない**のでは。

**反例**: `export {"resolution": 5000}` → OOM。`screenshot {"width": 1e9}` → OOM。

**結論 → 反映**:
- `arg_resolution`/`arg_dim` を新設し、全MCPツールで `res∈[1,256]`・`dim∈[1,4096]` に丸める。
  非有限・負・0・過大は安全側へクランプし、panic/OOM を構造的に不能化。
- テスト `resolution_is_clamped_to_safe_range`・`image_dims_are_clamped`。

## 問19 — 製品の中核主張「水密100%」が2例しか検証されていない
**問**: §2/問11 は「構築時から水密性保証・非多様体0」を製品の決定的優位として掲げる。だが抽出の
水密性テストは **球と球-円柱の2例のみ**。全プリミティブ・ブーリアン・smooth・shell・mirror で
本当に成り立つのか。三角形ごとに勾配で向き付けする方式（巻き順の位相的管理なし）は、薄肉部で
向き不整合を生みうる。看板の保証が抜き取り検査では装飾では。

**検証 → 反映**:
- **形状バッテリ性質テスト** `watertight_guarantee_holds_across_shape_battery` を追加（13形状:
  全プリミティブ＋union/intersection/difference/smooth_union/shell/mirror）。各形状で
  edge-manifold（水密）と**符号付き体積>0**（外向き向き一貫性の代理）を要求。
- 結果: **全13形状が通過**。保証は誤りではなく**検証不足**だった。性質テストで恒久ゲート化。

## 問20 — 非有限値とゼロスケールが無音で不正メッシュを生む（問14の沈黙故障の再来）
**問**: `1e400` は JSON で f64 の +inf に丸められ、`{"op":"sphere","r":1e400}` は inf 半径として
場へ伝播し無音で空/不正メッシュを生む。`{"op":"scale","s":0}` は `factor*child(p/factor)` の
0除算で NaN を撒き、`val < 0` 判定を壊す。問14で戒めた「もっともらしいが誤った結果」が入力検証の
欠落で再来していないか。

**結論 → 反映**:
- 非有限値は**パーサ一点**で遮断（`parse_number` が `1e400` 等の非有限を拒否）。スクリプト・MCP引数の
  全数値に効く（v4の深さ上限と同じ単一チョークポイント方針）。
- `scale` は `s<=0` を評価器で明示拒否（0除算・内外反転を防ぐ）。
- テスト `non_finite_number_is_rejected`・`zero_or_negative_scale_is_rejected`・`non_finite_param_is_rejected_via_parser`。

## 反映サマリ v5
| 問 | 種別 | コードへの主な反映 | 固定テスト |
|----|------|--------------------|-----------|
| 18 | 未カバーの攻撃面（数値引数DoS） | `arg_resolution`/`arg_dim` で全MCPツールをクランプ | `resolution_is_clamped_*`, `image_dims_are_clamped` |
| 19 | 中核保証の検証不足 | 13形状の水密性＋向き一貫性を性質テスト | `watertight_guarantee_holds_across_shape_battery` |
| 20 | 沈黙故障（入力検証欠落） | パーサで非有限拒否・評価器で scale>0 強制 | `non_finite_*`, `zero_or_negative_scale_is_rejected` |

> 総括: v5は「防御は塞いだ経路だけ有効」を起点に、同要件の別ベクトル(問18)を塞ぎ、看板保証(問19)を
> 抜き取りから性質テストへ格上げし、沈黙故障(問20)を単一チョークポイントで断った。テスト数 57→63。

---

# Socratic Review v6 — DSLが約束する操作と有限パイプラインの不整合

> 日付: 2026-06-13 ｜ 対象: DSL操作 `repeat`・DFM肉厚指標（`src/`）
> 目的: DSLが露出する操作が、その下流（有限メッシャ・有限BBox・検証）と整合しているかを突く。
> 原則: 反例を作れる不整合・誇張のみ採用。偽の指摘は明示的に棄却する。

## 問21 — `repeat` は無限格子。有限メッシャで実現できず、無音で1セルに潰れる
**問**: DSL は `repeat` を露出する。だが `Sdf::Repeat` の eval はセル番号を `floor` で無限に
畳み込み、`aabb` は子1セル分しか返さない。有限メッシャと有限BBoxでは無限格子を表現できないため、
配列を期待した AI は**無音で1コピー（または境界でクリップされた格子）**を得る。DSLの約束と
下流の能力が不整合では。

**反例**: `{"op":"repeat","x":2.0,...}` で3×3配列を作ろうとしても、aabb=1セルのため export は1個分。

**結論 → 反映**:
- `Repeat` を**有限繰り返し**に変更 (IQ opLimitedRepetition): `count[axis]` で原点両側の
  コピー数を指定（合計 `2n+1`）。`aabb` は `period*count` だけ有限に拡張。
- DSL `repeat` に `nx/ny/nz`（既定1・上限256）を追加。`repeat_n` コンストラクタを新設。
- テスト: 範囲内は負・範囲外(4セル目)は正で**無限タイルでないこと**、aabbが有限なこと。
  E2E確認: 球×3 (period2,r0.5) が体積1.555≒3×0.523・水密・有限BBoxで出力。

## 問23 — 「最小肉厚」と称しながら 2V/SA は平均。細リブを見逃す誇張（問1の再来）
**問**: `validate` の THIN_WALL は「estimated **minimum** wall thickness」と報告するが、計算は
2V/SA = **平均**肉厚。塊状本体＋細リブの形状では本体に支配され大きく出て、リブの薄さを見逃す。
「最小」を名乗るのは問1で正した誇張の再来で、製造可否を誤って PASS させる安全上の問題では。

**結論 → 反映**:
- 指標を正直に**「mean wall thickness (2V/SA average)」**と改称（`min_wall_thickness`→`mean_wall_thickness`）。
  メッセージに「a pass does not guarantee no local thin features」を明記。閾値未満の検出は有効な
  シグナルとして Error 維持、ただし**通過は薄肉皆無を保証しない**ことを文書化。真の最小肉厚 (medial axis) は BACKLOG。

## 棄却した指摘 — `initialize` の protocolVersion ハードコード
**問**: `handle_initialize` は client の `protocolVersion` を無視し固定値を返す。ネゴシエーション違反では。
**検証 → 棄却**: MCP仕様では「要求版を支持するなら同値を返す。さもなくば自分の支持版を返す」。
本サーバーは単一版のみ支持するため、常に自版を返す現挙動は**仕様準拠**。バグではないため改変せず。
（問17同様、偽陽性を作らない。）

## 反映サマリ v6
| 問 | 種別 | コードへの主な反映 | 固定テスト |
|----|------|--------------------|-----------|
| 21 | DSLと下流の不整合（無限格子） | `Repeat` を有限化・`aabb`有限拡張・DSL `nx/ny/nz` | `repeat_is_bounded_not_infinite`, `repeat_script_is_bounded` |
| 23 | 誇張の是正（平均を最小と詐称） | 指標を mean と改称・通過は無保証と明記 | 既存DFMテストで挙動維持を確認 |
| 22 | （棄却）プロトコル版 | 仕様準拠と判定し改変せず | — |

> 総括: v6は「DSLが書けることと、有限パイプラインが実現できること」の差を突いた。問21は無限格子を
> 有限配列に直し配列機能を初めて実用化。問23は問1の誇張是正をDFM層で繰り返した。問22は仕様準拠と
> 確認し棄却（誠実さの担保）。テスト数 63→65。

---

# Socratic Review v7 — 中核保証を「難所」で実証し、自己修正の誤誘導を断つ

> 日付: 2026-06-13 ｜ 対象: 抽出の水密性（問11の難所）・検証の構造化エラー品質（問3）
> 目的: v5の水密性バッテリは「素直な」形状だった。問11が明示的に懸念した**完全一致面・格子整列面**で
> 本当に崩れないかを**実測**で突く。さらに、検出した失敗が AI自己修正ループに**正しいヒント**を返すか。
> 原則: 仮説は実測で検証し、データが反証したら仮説を棄てる（修正をでっち上げない）。

## 問24 — 完全一致面・格子整列面で水密が崩れる、あるいは体積が崩落するのでは
**問**: 問11は「薄肉部・完全一致部で非多様体」を懸念。v5バッテリはこれを避けていた。同一球の和、
同一立方体の積、面が標本平面に正確に載る格子整列直方体で、(a)水密が崩れるか (b)体積が崩落するか。

**実測 → 一部反証**:
- (a) **水密は崩れない**。3つの敵対ケースすべてで edge-manifold＋体積正を確認（性質テスト追加）。
- (b) 「体積崩落」仮説は**反証**された。格子整列直方体 res4 の体積誤差(50%)は**整列のせいではなく純粋な
  低解像度離散化**。非整列境界 (3.809) はむしろ整列 (4.000) より悪く、res16では両者≒（7.66 vs 7.74）。
  整列固有の崖は存在しない。→ 修正はでっち上げず、保証の**難所での実証テストのみ**を恒久追加。

**結論 → 反映**: `watertight_holds_for_adversarial_coincident_cases` を追加。水密保証を問11の難所へ拡張。

## 問25 — 境界クリップで開いたメッシュが「無音で水密扱い」され、誤ったヒントを返すのでは
**問**: 形状がサンプリング境界を超えると表面が箱の面で開く（問14の懸念）。これは検知されるか。
また `validate` の非多様体ヒントは「解像度を上げよ」だが、**クリップ（開境界）には逆効果**で、
正しい是正は**境界の拡大**。自己修正ループ(問3)を誤誘導していないか。

**実測 → 確認**:
- 部分クリップ → 開境界エッジ → `is_edge_manifold=false` で**確実に検知**（無音で水密にならない）。
  完全内包 → 空メッシュ → EMPTY_MESH で検知。安全網は機能している。
- だが旧 `NON_MANIFOLD` は開境界(1面共有)と非多様体接合(3面以上)を**混同**し、両者に「解像度を上げよ」を
  出していた。クリップにこれは誤誘導。

**結論 → 反映**:
- `Mesh::edge_defects()` を追加し開境界数と非多様体接合数を分離。
- `validate` を原因別に分割: **OPEN_MESH**（開境界→「境界を拡大せよ／ゼロ厚を避けよ」）と
  **NON_MANIFOLD**（接合→「解像度を上げよ／形状を分離せよ」）。問3の fix_hints 品質を改善し、
  AIが正しい修正に到達できるようにした。
- テスト `boundary_clipping_is_detected_not_silently_watertight`・`clipped_mesh_reports_open_mesh_with_bounds_hint`。

## 反映サマリ v7
| 問 | 種別 | コードへの主な反映 | 固定テスト |
|----|------|--------------------|-----------|
| 24 | 中核保証の難所実証（＋仮説の反証） | 敵対的一致面の水密性を性質テスト。体積崩落仮説は実測で棄却 | `watertight_holds_for_adversarial_coincident_cases` |
| 25 | 安全網の実証＋自己修正ヒント改善 | `edge_defects` で開境界/接合を分離・OPEN_MESH と NON_MANIFOLD に分割 | `boundary_clipping_is_detected_*`, `clipped_mesh_reports_open_mesh_with_bounds_hint` |

> 総括: v7は看板保証を問11の「難所」で実証し（崖の仮説は実測で棄却＝誠実さ）、クリップ失敗が無音化せず
> かつ**正しい是正ヒント**を返すよう構造化エラーを原因別に分割した。テスト数 65→68。

---

## 問26 — AI は `run_script` でシーンを設定できるが読み返す手段がない（自己修正ループ不完全）
**問**: Plan §3 はAIが自律的に「試す→検証→修正」を回す自己修正ループを謳う。だが現行の MCP ツール群
(`screenshot`, `export`, `eval`, `run_script`, `validate`) に**現在のシーンを読み返すツールがない**。
`Session` は `Sdf` 木のみ保持し、元スクリプトは評価後に破棄される。AIがコンテキストを失った場合
（モデルリセット・ロールバック・新セッション）、ロードされている形状を確認する方法がなく孤立する。
これはAI-firstアーキテクチャの空白ではないか。

**結論 → 反映**:
- `Session` に `script: Option<String>` フィールドを追加。`run_script` 評価成功時に元の KadoScene JSON を保存。
- `get_scene` ツールを新設: 保存済みスクリプトとサンプリング境界 (`bounds=[lo]-[hi]`) を返す。
  未設定時は `"(default scene — no run_script call yet)"` と明示。
- 計6ツール体制。`tools_list_has_six_tools` テスト、`get_scene_round_trip` テスト追加。

## 問27 — `shell` の `thickness <= 0` は `scale <= 0` と同様に無音の不正メッシュを生む
**問**: `scale <= 0` は問20で拒否するよう修正された（距離場の破壊）。
`shell` の `thickness=0` は `d.max(-d) = |d|` を生み出す — これは**内部が存在しない**絶対値関数であり、
ゼロ体積のメッシュまたは非多様体形状を引き起こす。`thickness < 0` は幾何的に無意味。
同様の弁護がないのは一貫性の欠如ではないか。

**結論 → 反映**:
- `eval.rs` の `shell` ブランチに `t <= 0.0` チェックを追加。エラーメッセージ `"shell thickness must be > 0"` を返す。
- `zero_or_negative_shell_thickness_is_rejected` テスト追加。
- `mirror_operations_via_script` テスト追加（鏡面操作の eval カバレッジが欠如していた）。

## 問28 — プリミティブの負/ゼロパラメータが無検査で受理される（`scale`/`shell` との非対称）
**問**: 問20で `scale<=0`、問27で `shell thickness<=0` を拒否するよう修正した。
しかし `sphere(r=0)` は `eval = |p|` (内部なし)、`cylinder(r=0, h=1)` は線分、
`torus(major=0, minor=0.1)` は点、`cuboid(x=0, ...)` は平面を生み — いずれも
**eval エラーなく受理されて `EMPTY_MESH` でサイレント失敗**する。
操作レベルの検証を揃えながらプリミティブ側の検証がないのは一貫性の欠如ではないか。

**結論 → 反映**:
- `req_positive_f64(v, key)` ヘルパを追加 (`req_f64` + `> 0` チェック)。
- 全プリミティブに適用: `sphere(r)`, `cylinder(r, h)`, `cone(r, h)`, `torus(major, minor)`, `capsule(r)`、
  `cuboid(x, y, z)`, `rounded_box(x, y, z, r)` の各次元を正値強制。
- 例外: `capsule(h)` は `h=0` が球として有効なため `>= 0` のみ。
- `zero_or_negative_primitive_dimensions_are_rejected` テスト追加 (28 ケース)。

## 反映サマリ v8–v10
| 問 | 種別 | コードへの主な反映 | 固定テスト |
|----|------|--------------------|-----------|
| 26 | 自己修正ループの欠陥 | `Session::script` フィールド追加・`get_scene` ツール新設 | `get_scene_round_trip`, `tools_list_has_six_tools` |
| 27 | 入力バリデーション統一 | `shell thickness <= 0` 拒否、`mirror_*` eval テスト追加 | `zero_or_negative_shell_thickness_is_rejected`, `mirror_operations_via_script` |
| 28 | プリミティブ次元バリデーション | `req_positive_f64` ヘルパ、全プリミティブの正値強制 | `zero_or_negative_primitive_dimensions_are_rejected` |

> 総括: v8–v10はAIエージェントがシーン状態を検査・読み返せる `get_scene` ツールを追加し（問26）、
> `shell` の thickness バリデーション欠落を修正（問27）、すべてのプリミティブパラメータに
> 正値強制を適用して input sanitization の一貫性を完成させた（問28）。テスト数 68→72。

---

## 問29 — `screenshot` の mesh quality が fixed (res=48)、`export`/`validate` との非対称
**問**: `export` と `validate` は `resolution` 引数でメッシュ品質を制御できる。
`screenshot` は image dimensions (`width`, `height`) を変更できるが、
polygonize の `res` は 48 固定。AIが最終成果物確認のために高品質なスクリーンショットを
要求したい場面 (res=96 で滑らかな表面) で制御手段がない。これは API の対称性の欠如では？

**結論 → 反映**:
- `screenshot` ツールの schema に `resolution` 引数 (integer, default 48) を追加。
- `tool_screenshot` 内で `arg_resolution(args, 48)` を使用し polygonize に渡す。
- 既存の `arg_resolution` の境界チェック (1–256) が自動適用される。

## 問30 — smooth CSG の三位一体が不完全 (`SmoothIntersection` 欠落)
**問**: `SmoothUnion` と `SmoothDifference` は実装済みだが、`SmoothIntersection` がない。
IQ の smooth CSG primitives は union/intersection/difference の3演算をセットとして定義する。
intersection の smooth 版だけ欠けているのはツールセットの一貫性の欠如では？
AI が「スムーズな交差」を要求したとき `unknown op: "smooth_intersection"` で失敗する。

**結論 → 反映**:
- `Sdf::SmoothIntersection(Box<Sdf>, Box<Sdf>, f64)` enum バリアントを追加。
- `eval` に IQ の smooth intersection 式を実装: `h = clamp(0.5 - 0.5*(db-da)/k, 0,1); mix(db,da,h) + k*h*(1-h)`。
- `aabb` に追加 (積集合 + k の余白)。
- `smooth_intersection()` コンストラクタと `eval.rs` の `"smooth_intersection"` ブランチ追加。
- テスト: `smooth_intersection_is_upper_bound_of_hard_intersection`、`smooth_intersection_is_lower_bound_of_hard_union` (収束性)、`smooth_intersection_via_script`。

## 反映サマリ v8–v11
| 問 | 種別 | コードへの主な反映 | 固定テスト |
|----|------|--------------------|-----------|
| 26 | 自己修正ループの欠陥 | `Session::script` + `get_scene` ツール | `get_scene_round_trip` |
| 27 | バリデーション統一 | `shell thickness <= 0` 拒否 | `zero_or_negative_shell_thickness_is_rejected` |
| 28 | プリミティブ次元バリデーション | `req_positive_f64`, 全プリミティブ正値強制 | `zero_or_negative_primitive_dimensions_are_rejected` |
| 29 | API 対称性 | `screenshot` に `resolution` 引数追加 | (既存 `resolution_is_clamped_to_safe_range` 継承) |
| 30 | SDF 機能完全性 | `SmoothIntersection` 追加 (eval/aabb/script/コンストラクタ) | `smooth_intersection_is_*`, `smooth_intersection_via_script` |

> 総括: v8–v11はAI自己修正ループの完全化（問26）、input sanitization の系統化（問27/28）、
> API 対称性の回復（問29）、smooth CSG の三位一体完成（問30）を実現した。テスト数 68→75。

---

## 問31 — CLI の `export`/`screenshot` がデモモデル固定で KadoScene JSON を受け付けない
**問**: `run` と `check` コマンドは KadoScene JSON ファイルを受け取る。しかし
`export` と `screenshot` はデモモデル (`demo_model()`) 固定であり、
ユーザー定義シーンをファイルから直接 STL/PNG に変換する CLI 経路が存在しない。
「スクリプトが正本」(問2) の原則が CLI レベルで崩れており、AI エージェントが
CI/CD パイプラインで `kado export scene.json out.stl` を呼べない。

**結論 → 反映**:
- `export`: `[scene.json] <out.stl>` — arg が `.json` で終わる場合は scene file として
  読み込み `sampling_box()` から bounds を導出。省略時は demo model (後方互換)。
- `screenshot`: `[scene.json] <out.png> [view]` — 同様。
- `load_scene_file` / `parse_scene` ヘルパを抽出して `run`/`check` との共通化。
- 手動スモークテスト: `kado export sphere.json out.stl` → 83856 triangles, manifold=true。

## 反映サマリ v8–v12
| 問 | 種別 | コードへの主な反映 | 固定テスト |
|----|------|--------------------|-----------|
| 26 | AI自己修正ループ | `Session::script` + `get_scene` ツール | `get_scene_round_trip` |
| 27 | バリデーション統一 | `shell thickness <= 0` 拒否 | `zero_or_negative_shell_thickness_is_rejected` |
| 28 | プリミティブ次元 | `req_positive_f64`, 全プリミティブ正値強制 | `zero_or_negative_primitive_dimensions_are_rejected` |
| 29 | API 対称性 | `screenshot` resolution 引数追加 | (既存 `arg_resolution` テスト継承) |
| 30 | SDF 完全性 | `SmoothIntersection` 追加 | `smooth_intersection_is_*`, `smooth_intersection_via_script` |
| 31 | CLI 一貫性 | `export`/`screenshot` が scene.json を受け付けるよう拡張 | 手動スモークテスト |

> 総括: v8–v12はAI自己修正ループの完全化から CLI の一貫性回復まで、
> 問26〜31の6課題を実装で解決した。テスト数 68→75 (コード品質向上はテスト外も含む)。

---

## 問32 — JSON シリアライザの `escape()` が制御文字を `String::Display` と非対称に扱う
**問**: `Value::String` の `Display` 実装は `\r`, `\t`, 制御文字 (`\u00XX`) を
正しくエスケープするが、オブジェクトキー用の `escape()` ヘルパは `"`, `\`, `\n` のみ。
現行システムが生成するキーは常に安全な ASCII 識別子だが、
ライブラリ関数として内部実装と外部インターフェースが非対称なのは正当性の欠如ではないか。

**結論 → 反映**:
- `escape()` に `\r`, `\t`, 制御文字 (`\u{xxxx}`) のエスケープを追加。
- `object_key_escape_matches_string_value_escape` テスト追加:
  `\t`/`\r` を含むキーのラウンドトリップと生制御文字の不在を確認。

## 反映サマリ v8–v13
| 問 | 種別 | コードへの主な反映 |
|----|------|--------------------|
| 26 | AI自己修正ループ | `Session::script` + `get_scene` ツール |
| 27 | バリデーション統一 | `shell thickness <= 0` 拒否 |
| 28 | プリミティブ次元 | `req_positive_f64`, 全プリミティブ正値強制 |
| 29 | API 対称性 | `screenshot` resolution 引数追加 |
| 30 | SDF 完全性 | `SmoothIntersection` 追加 |
| 31 | CLI 一貫性 | `export`/`screenshot` が scene.json 入力に対応 |
| 32 | JSON エスケープ一貫性 | `escape()` 関数の制御文字処理を `Display` に揃える |

> 総括: v8–v13は7課題 (問26〜32) を実装で解決し、テスト数 68→76 に伸ばした。

---

## 問33 — カメラプリセットのデフォルトフォールバックがインデックス指定で脆弱
**問**: `screenshot` ツールと CLI の `screenshot` コマンドは、無効なビュー名が渡されたときに
`presets[6]` をデフォルトとする。"iso" が6番目 (0インデックス) にある前提であり、
プリセット順序が変わると誤ったビューにフォールバックする。名前による検索より脆弱ではないか。

**修正**: `presets[6]` → 名前検索: `.or_else(|| presets.iter().find(|(n,_)| *n == "iso")).unwrap_or(&presets[0])`。
"iso" がリストになければ先頭 (front) にフォールバックする。

## 問34 — `smooth_difference` に収束性テストがない (`smooth_union` との非対称)
**問**: `smooth_union` には「上界」と「k→0で hard union に収束」の2つのテストがある。
`smooth_difference` には同等のテストがなく、数式の正しさを検証する手段がない。
`smooth_intersection` のテストを追加した際に `smooth_difference` を見落としたのでは。

**修正**: `smooth_difference_is_upper_bound_of_hard_difference` と
`smooth_difference_converges_to_hard_as_k_shrinks` の2テストを追加。
テスト数 76→78。

## 反映サマリ v8–v14
| 問 | 実装 |
|----|------|
| 26 | `get_scene` ツール + `Session::script` |
| 27 | `shell thickness <= 0` 拒否 |
| 28 | 全プリミティブ正値強制 |
| 29 | `screenshot` resolution 引数 |
| 30 | `SmoothIntersection` 追加 |
| 31 | CLI: export/screenshot で scene.json 受け付け |
| 32 | JSON escape() 制御文字一貫性 |
| 33 | カメラフォールバックを名前検索に変更 |
| 34 | smooth_difference 収束性テスト追加 |

> 総括: v8–v14は問26〜34の9課題を実装で解決し、テスト数 68→78 に伸ばした。

---

## 問35 — `smooth_union`/`smooth_difference` に script-eval テストがない
**問**: `smooth_intersection` には `smooth_intersection_via_script` テストを追加した (問30)。
しかし `smooth_union` と `smooth_difference` は eval.rs レベルのテストが欠落している。
これら3演算は対称的に検証されるべきでは？

**修正**: `smooth_intersection_via_script` を `smooth_operations_via_script` に拡張し、
union/intersection/difference の3演算すべてを script 経由で検証する。

## 問36 — CLI `run`/`check` の resolution が48固定でMCP `validate` と非対称
**問**: MCP の `validate` ツールは `resolution` 引数でメッシュ品質を制御できる。
CLI の `run` と `check` は解像度を制御する手段がなく、常に res=48。
高品質 DFM 検証 (`kado check scene.json 0.3 45 96`) が CLI から呼べない。

**修正**:
- `run <scene.json> [resolution]` — 追加引数 (デフォルト48, clamp 1–256)。
- `check <scene.json> [min_wall_mm] [max_overhang_deg] [resolution]` — 同上。
- `run` コマンドを `load_scene_file`/`parse_scene` ヘルパを使うよう整理。

## 反映サマリ v8–v15
| 問 | 実装 |
|----|------|
| 26 | `get_scene` + `Session::script` |
| 27 | `shell thickness <= 0` 拒否 |
| 28 | 全プリミティブ正値強制 |
| 29 | `screenshot` resolution 引数 |
| 30 | `SmoothIntersection` |
| 31 | CLI: export/screenshot で scene.json |
| 32 | JSON escape() 制御文字 |
| 33 | カメラフォールバック名前検索 |
| 34 | smooth_difference 収束性テスト |
| 35 | smooth 3演算 script テスト統合 |
| 36 | CLI run/check に resolution 引数追加 |

> 総括: v8–v15は問26〜36の11課題を実装で解決し、テスト数 68→78 に達した。

---

## 問37 — `run_script` が自己記述的でない (AI が何を書けるか知る手段がない)
**問**: AI エージェントが初めて Kado MCP を使う際、`tools/list` を呼んでも
`run_script` のスキーマには `script: string` のみで、KadoScene JSON の内部形式は不明。
`help` ツールも `schema` ツールもない。AI は試行錯誤で `unknown op` エラーを
繰り返すしかない — AI-first として自己記述性が欠けている。

**修正**: `help` ツール (引数なし) を追加。KadoScene JSON フォーマット参照文書を返す:
- 全プリミティブ (`sphere`/`cuboid`/`cylinder`/`torus`/`cone`/`capsule`/`rounded_box`) と必須パラメータ
- 全ブーリアン (`union`/`intersection`/`difference`/`smooth_{union,intersection,difference}`)
- 全変形 (`translate`/`scale`/`offset`/`shell`/`mirror_{x,y,z}`/`repeat`)
- 例スクリプト付きワークフロー解説
計7ツール体制。`help_tool_returns_format_reference` テスト追加。テスト数 78→79。

## 反映サマリ v8–v16
| 問 | 実装 |
|----|------|
| 26 | `get_scene` + `Session::script` |
| 27 | `shell thickness <= 0` 拒否 |
| 28 | 全プリミティブ正値強制 |
| 29 | `screenshot` resolution 引数 |
| 30 | `SmoothIntersection` |
| 31 | CLI: export/screenshot で scene.json |
| 32 | JSON escape() 制御文字 |
| 33 | カメラフォールバック名前検索 |
| 34 | smooth_difference 収束性テスト |
| 35 | smooth 3演算 script テスト統合 |
| 36 | CLI run/check に resolution 引数 |
| 37 | `help` ツール: KadoScene 自己記述 |

> 総括: v8–v16は問26〜37の12課題を実装で解決し、テスト数 68→79 に伸ばした。

---

## 問38 — OVERHANG エラーが角度慣例を混同 (`acos(nz)` vs 水平からの角度)
**問**: `validate` の OVERHANG 検査で `max_overhang_deg=45` は「水平から45°」を意味するが、
エラーメッセージの `deg = worst.acos().to_degrees()` は「上向きz軸からの角度」を返す
(水平面=90°, 真下=180°)。nz=-0.766 (水平から50°) に対して "overhang angle 140.0° exceeds max 45.0°"
を報告するのは単位の不一致。

**修正**: `worst.acos().to_degrees()` → `(-worst).asin().to_degrees()`。
水平からの角度 (0°=垂直, 90°=真下) で `max_overhang_deg` と同じ慣例に揃える。
エラー文言も "from horizontal" を追記し明確化。
テスト `overhang_angle_reported_from_horizontal_not_from_z_axis` 追加。
テスト数 79→80。

## 反映サマリ v8–v17
| 問 | 実装 |
|----|------|
| 26–37 | (上記) |
| 38 | OVERHANG 角度慣例修正 (acos→asin, 水平基準に統一) |

> 総括: v8–v17は問26〜38の13課題を実装で解決し、テスト数 68→80 に達した。

---

## 問39 — テスト名 `smooth_intersection_is_lower_bound_of_hard_union` が内容と矛盾
**問**: テストは `smooth_intersection → hard_intersection` への収束 (k→0) を確認するが、
名前は "lower bound of hard union" と言っており地図と現地が違う。
将来の読者が「なぜ smooth_intersection が union の下界なのか」と混乱する。

**修正**: → `smooth_intersection_converges_to_hard_as_k_shrinks` に改名。

## 問40 — 非重複 `SmoothIntersection` で `sampling_box` が反転ボックスを返す
**問**: `SmoothIntersection(a, b, k)` の AABB は `max(alo, blo) - k` to `min(ahi, bhi) + k`。
`a` と `b` が離れていると `lo > hi` になる。`sampling_box` はそのまま反転ボックスを返し、
`polygonize` が負のステップで無駄な (かつ混乱を招く) サンプリングをする。

**修正**: `sampling_box` で `(lo.min(hi), lo.max(hi))` により AABB を正規化する。
反転ボックスは最小のデフォルト margin を持つ点ボックスになり、空メッシュを生む。
テスト `sampling_box_is_never_inverted` 追加 (非重複 smooth_intersection の空メッシュ確認)。
テスト数 80→81。

## 反映サマリ v8–v18
| 問 | 実装 |
|----|------|
| 26–38 | (上記) |
| 39 | テスト名の誤り修正 |
| 40 | sampling_box の反転ボックス正規化 |

> 総括: v8–v18は問26〜40の15課題を実装で解決し、テスト数 68→81 に達した。

---

## 問41 — `render_is_deterministic` がハードコード索引 `presets[6]` で iso を取得
**問**: 問33 で `tools.rs`/`cli` のカメラ取得を「名前検索」に直したが、
テスト `render_is_deterministic` だけ `Camera::presets(...)[6]` のまま残っていた。
プリセット順序を変えるとテストが別視点を検証する沈黙退行のリスク。

**修正**: `presets.iter().find(|(n,_)| *n == "iso")` に統一。

## 問42 — (欠番: 問45 に統合)

## 問45 — `top`/`bottom` カメラが視線∥up で縮退しブランク画像
**問**: `Camera::presets` は全視点で `up=(0,0,1)`。しかし `top`(eye=+z)/`bottom`(eye=-z)
は視線方向が z 軸と平行で、`look_at` 内の `cross(forward, up)` がゼロベクトルになり
right/up 基底が縮退する。結果、上面・底面ビューが背景一色のブランク画像になっていた。

**修正**: `top`/`bottom` のみ `up=(0,1,0)` を使い非縮退な基底を保証する。
テスト `top_and_bottom_views_render_non_blank` が両視点で前景画素が出ることを確認。

## 問46 — `run_script` が EMPTY_MESH でも "scene updated" と返す
**問**: `run_script` は検証レポートのサマリは付けるが先頭は常に "scene updated"。
EMPTY_MESH 等のエラーコードが目立たず、AI 自己修正ループが成功と誤認しうる。

**修正**: `Severity::Error` のコードがあれば "scene updated (check issues: EMPTY_MESH)"
のようにプレフィックスへ明示。`is_error=false` は維持 (スクリプト自体は有効)。

## 問47 — CLI `check` がファイル読み込み・評価を重複実装
**問**: `run` は `load_scene_file`/`parse_scene` ヘルパを使うが、`check` だけ
`std::fs::read_to_string` と `eval_scene` のエラー処理をインライン重複していた。

**修正**: `check` も同ヘルパに統一。エラーメッセージとフロー制御を一箇所へ集約。

## 問48 — CLI `export` が空メッシュでも無音で STL を書き出す
**問**: `screenshot` は `triangles.is_empty()` で早期終了するが、`export` は空メッシュでも
ヘッダのみの STL を「成功」として書き出していた。利用者は空ファイルに気づけない。

**修正**: `export` も空メッシュを検出し、境界拡大ヒント付きで非ゼロ終了。

## 問49 — JSON パーサがマルチバイト UTF-8 を破壊 (実害バグ)
**問**: `parse_string` が非エスケープ文字を `s.push(c as char)` で追加していた。
`c: u8` に対する `c as char` は Latin-1 解釈で、マルチバイト UTF-8 (日本語・絵文字・
アクセント記号) の各バイトが個別の誤コードポイントへ化ける。MCP は UTF-8 JSON を
送るため、非 ASCII を含むスクリプト/パラメータが無音で文字化けしていた。

**修正**: リードバイトから UTF-8 シーケンス長を判定し (`utf8_seq_len`)、
`self.src` (&str 由来で正当な UTF-8) を文字単位でコピー。
テスト `multibyte_utf8_strings_roundtrip` が日本語・絵文字・アクセント記号・
UTF-8 オブジェクトキー/値のラウンドトリップを検証。

## 問50 — JSON リテラル `true`/`false`/`null` を盲目的に索引前進
**問**: 先頭1文字 (`t`/`f`/`n`) を見て `pos` を 4/5/4 だけ進めるだけで、後続バイトを
照合していなかった。`nXYZ` を `null` として無音受理し、末尾付近では `pos` が
`src.len()` を越えうる退行があった。

**修正**: `parse_literal` でキーワード全バイトを照合し、不一致・末尾越えは明示エラー。
テスト `malformed_literals_are_rejected` で誤綴り・途中切れ・配列内不正リテラルを検証。

## 反映サマリ v19–v22
| 問 | 実装 |
|----|------|
| 41 | テストのカメラ取得を名前検索へ統一 |
| 45 | top/bottom カメラの縮退を up=(0,1,0) で回避 |
| 46 | run_script がエラーコードをプレフィックス表示 |
| 47 | CLI check をヘルパに統一 |
| 48 | CLI export の空メッシュ検出 |
| 49 | JSON パーサのマルチバイト UTF-8 破壊修正 (実害) |
| 50 | JSON リテラルの厳密照合 |

> 総括: v19–v22は問41〜50の9課題を実装で解決し、テスト数 81→85 に達した。
> 特に問49 (UTF-8 破壊) は非 ASCII 入力で無音の文字化けを起こす実害バグだった。

---

## 問51 — 回転変換が存在しない (重大な機能欠落)
**問**: 変形演算は translate/scale/offset/shell/repeat/mirror があるが **rotate がない**。
円柱を z 以外の軸へ向ける、3D プリント向けに部品を傾けるといった CAD の基本操作が
不可能で、すべての形状が原点軸固定だった。

**修正**: `Sdf::Rotate(child, axis, angle)` を追加。
- eval は点を `-angle` で逆回転してから子を評価 (剛体変換ゆえ距離場は厳密、スケール不変)。
- aabb は子 aabb の 8 隅を `+angle` 回し、その軸整列 bbox を取る (保守的・内包保証)。
- 決定性 (問5): sin/cos は同一バイナリ・同一arch内で確定的 (既存の sqrt と同水準)。
- script: "rotate_x"/"rotate_y"/"rotate_z"、angle は**度** (CAD 慣習)。
- 水密バッテリに rotate ケースを追加し、回転形状でも edge-manifold を確認。

テスト 85→90 (+5) + battery 1: rigid 保存・球の回転不変・90°でのaabb入替・
ビット決定性・スクリプト経路 (0°恒等/angle欠落エラー)。

## 反映サマリ v23
| 問 | 実装 |
|----|------|
| 51 | 回転変換 rotate_x/y/z (剛体・距離保存・度指定) |

> 総括: v23 は問51 (回転欠落) を実装で解決し、テスト数 85→90 に達した。
> 回転は CAD/3Dプリントの基本操作であり、変形演算セットの主要な空白を埋めた。

---

## 問52 — 評価セット (EVAL-SET) を実行可能テストとして実装
**問**: BACKLOG の「評価セット (N≥10)」が未実装で、製品 KPI (水密100%・決定的・
構造健全) を横断的に測る基盤がなかった。機能追加時の退行も検知できない。

**修正**: `tests/eval_set.rs` に代表 12 モデル (bracket/lens/pipe elbow/hollow
enclosure/dumbbell/plus/tilted block/ring/bolt/perforated plate/cross-drilled/
mirrored fins、後に egg を追加) を KadoScene で定義。各モデルを
script→SDF→polygonize→validate に通し、水密性・構造健全性・ビット決定性・
向き一貫性を性質テスト。全演算 (回転・smooth・repeat・shell・mirror) を横断運動させる。

## 問53 — 楕円体プリミティブが存在しない
**問**: 非球面・非箱型の丸み形状 (卵・レンズ・有機形状) を作れなかった。非一様スケールは
距離場の Lipschitz 性を壊すため提供できず、楕円体は空白だった。

**修正**: IQ 近似式による `Ellipsoid { radii }` を追加。**符号は厳密**
((x/a)²+(y/b)²+(z/c)²<1)、軸上距離も厳密、軸外のみ近似。符号が厳密なので
marching tetrahedra の水密性は保たれる。aabb は厳密・有限。中心の0除算を回避。
script "ellipsoid": x/y/z 各軸半径、"s" で一様 (=球)。

## 問54 — 出力フォーマットが STL のみ
**問**: 出力は binary STL だけ。STL はインデックスも単位も境界情報も持たず、
ブラウザ等での閲覧にも不向き。Plan §3 で挙げた GLB が未実装だった。

**修正**: `io/gltf.rs` に **GLB (glTF 2.0 binary)** 書き出しを実装。
インデックス付きジオメトリ + accessor min/max 境界を持ち、ブラウザ・Blender・
Windows 3D ビューアで直接閲覧可能。圧縮なし・std のみ・決定的 (問5)。
CLI/MCP の export を拡張子で分岐 (.glb→GLB, 他→STL)。
副次効果: インデックス化により同一メッシュで GLB は STL の約 1/3 サイズ。
JSON チャンクは既存 json モジュールで構築 (4バイト整列パディング)。

## 反映サマリ v24–v25
| 問 | 実装 |
|----|------|
| 52 | 評価セット統合テスト (代表13モデル、KPI性質テスト) |
| 53 | 楕円体プリミティブ (符号厳密・水密維持) |
| 54 | GLB (glTF 2.0) 出力 + 拡張子分岐 |

> 総括: v24–v25 は問52〜54 を実装で解決。テスト数 90→97 + 統合 3。
> 機能 (楕円体・GLB) と測定基盤 (EVAL-SET) を同時に強化した。

---

## 問55 — 3Dプリント標準 3MF の出力がない
**問**: 出力は STL/GLB のみ。STL は単位も水密意味論も持たず、現代スライサが優先する
**3MF** (単位 mm・OPC パッケージ) が未実装で、印刷ワークフローの標準形式を欠いていた。

**修正**: OPC パッケージ (= ZIP コンテナ) として 3MF を実装。
- `io/zip.rs`: 最小 STORED (無圧縮) ZIP 書き出し。決定的 (タイムスタンプ固定・
  エントリ順保存)、CRC-32、ローカル/中央ディレクトリ/EOCD を正しく構成。
- `io/threemf.rs`: 3 パーツ ([Content_Types].xml / _rels/.rels / 3D/3dmodel.model) を
  生成。model は unit="millimeter" 宣言と vertices/triangles を持つ。
  ユーザ文字列を XML へ入れない設計ゆえエスケープ不要。
- CLI/MCP export を拡張子分岐に拡張 (.glb→GLB, .3mf→3MF, 他→STL)。
- 実機検証: Python zipfile で testzip() 通過 (CRC健全)、3パーツ存在、mm単位、
  頂点/三角形数一致を確認。

## 反映サマリ v26–v27
| 問 | 実装 |
|----|------|
| 54 | GLB (glTF 2.0) 出力 |
| 55 | 3MF 出力 (最小 ZIP/OPC 容器 + mm 単位) |

> 総括: v26–v27 で出力フォーマットを STL のみ → STL/GLB/3MF の3系統へ拡張。
> テスト数 97→103 + 統合 3。印刷標準 (3MF) と閲覧標準 (GLB) を両得した。

---

## 問56 — スクリーンショットにアンチエイリアスがなくシルエットが粗い
**問**: 中核ループは「script→screenshot→AI が判断→修正」。プレビュー品質が AI の
判断に直結するが、レンダラはアンチエイリアスなしで、既定メッシュ解像度では
輪郭が階段状になり形状の細部が読み取りにくかった。

**修正**: SSAA (スーパーサンプル) を導入。整数倍で描画し `factor`×`factor` ブロック
平均でダウンサンプルする (`Image::downsample`, 整数平均ゆえ決定的)。
- MCP screenshot に `samples` (1–4, 既定 2) を追加。スーパーサンプルバッファが
  MAX_IMAGE_DIM を超えないようクランプし OOM ガード (問18) を維持。
- CLI screenshot は 2× 固定 (1024→512)。
- 実機: difference 形状で 0.6s (<2s KPI 内)・有効 PNG を確認。

## 反映サマリ v28
| 問 | 実装 |
|----|------|
| 56 | SSAA アンチエイリアス (downsample, samples 1–4, OOM 安全) |

> 総括: v28 はプレビュー品質を SSAA で底上げし AI の判断材料を改善。
> テスト数 103→106 + 統合 3。KPI (<2s) を維持。

---

## 問57 — 自己完結 HTML ビューア (Plan §3) が未実装
**問**: Plan §3 が掲げる「HTML ビューア」が未実装で、出力をインタラクティブに
確認する手段が静的 PNG しかなかった。STL/GLB を別アプリで開く必要があった。

**修正**: `io/html.rs` に**単一ファイル自己完結 HTML ビューア**を実装。
- 頂点 + インデックスを埋め込み、外部リソースを一切参照しない WebGL2 ビューアを
  同梱 (外部送信ゼロ / 問4)。ブラウザで開けばオフラインでドラッグ回転・ズーム可能。
- 法線はフラグメントの画面空間微分 (dFdx/dFdy) から算出し、位置+索引のみ転送。
- 決定的 (4桁固定整形, 問5)。CLI/MCP export を .html 拡張子で分岐。
- 検証: (1) Rust 側で markers/索引数一致/決定性/プレースホルダ全置換をテスト、
  (2) node --check で埋め込み JS の構文検証、(3) **モック WebGL2 文脈で JS を実行**し
  有限な 4×4 行列 2 枚 (mvp/mv) 算出と drawElements 呼び出しを確認。

## 反映サマリ v29
| 問 | 実装 |
|----|------|
| 57 | 自己完結 HTML ビューア (WebGL2, オフライン, 外部送信ゼロ) |

> 総括: v29 で Plan §3 の出力 4 形式 (STL/GLB/3MF/HTML) が出揃った。
> テスト数 106→110 + 統合 3。ブラウザ実行を node モックで検証し未テスト領域を最小化。

---

## 問58 — 肉厚チェックが平均 (2V/SA) のみで局所薄肉を見逃す
**問**: THIN_WALL 検査は 2V/SA 平均肉厚のみ。太い本体に細いリブ/フィンが付く形状では
平均が本体に支配されて大きく出るため、リブの薄さ (=実際の製造リスク) を見逃していた。
これは正直に文書化されていた既知の限界だが、検出力が弱かった。

**修正**: SDF 場を使う**内向きレイ探針** `min_wall_probe` を追加 (問58)。
各表面頂点から内向き法線 (-∇SDF) 方向へ固定ステップで距離場を辿り、反対側の壁
(SDF が負→非負へ戻る点) までの距離を局所肉厚とみなし最小を返す。探針数は上限で間引く。
- `validate_with_field(mesh, Some(sdf), …)` を新設し、肉厚は 2V/SA 平均と探針の
  小さい方を採用。既存 `validate` は `sdf=None` 委譲で後方互換 (全呼出し維持)。
- MCP validate / CLI check は SDF を渡して探針を有効化。
- 限界: ステップ (diag/256) より薄い壁は跨いで見落とす。検出は有効だが非検出は
  薄肉皆無を保証しない (平均と同じ安全側の補助)。

検証 (テスト 110→113):
- probe_measures_shell_thickness: 厚さ 0.2 シェル → 探針 ≈ 0.2。
- probe_reports_large_thickness_for_solid_sphere: 中実球 → 大きい値 (誤検出しない)。
- probe_catches_local_thin_fin_that_mean_misses: 太い本体+薄フィン(0.1) で
  平均 (>0.2) は見逃すが探針 (<0.18) が捕捉。場併用 validate が THIN_WALL を出し、
  メッシュのみは出さないことを確認 (付加価値の実証)。

## 反映サマリ v30
| 問 | 実装 |
|----|------|
| 58 | 内向きレイ探針による局所薄肉検出 (validate_with_field, 後方互換) |

> 総括: v30 は中核 DFM (薄肉検出) の検出力を平均→平均+局所探針へ強化。
> テスト数 110→113 + 統合 3。後方互換を保ちつつ場対応の検査経路を追加。

---

## 問59 — スクリプトが冗長な JSON のみ (テキスト DSL 未実装)
**問**: KadoScene は JSON 専用で冗長。AI-First ツールでは記述がコンテキストトークンを
浪費する (関数式の約5倍)。Plan Phase-2 の「テキスト DSL」が未実装だった。

**修正**: `script/dsl.rs` に簡潔な関数呼び出し DSL を実装 (問59)。
  difference(union(sphere(1), cuboid(0.8)), cylinder(0.3, 2))
- **表層構文に徹する設計**: DSL は JSON と同一の KadoScene `Value` 木へ落ち、
  意味論・検証・リソース上限は `eval_value` に一元化 (重複ゼロ)。DSL 側は構文解析と
  位置引数→キー対応のみ。よって r<=0 や scale<=0 等の検査は自動的に共有される。
- 非有限数の拒否・ネスト深さ上限 (問16/20) も JSON と共通。
- `eval_any` が先頭文字 (`{`→JSON, 他→DSL) で自動判別。MCP run_script / CLI が両対応。
- help ドキュメントに DSL 構文表を追加。

検証 (テスト 113→120): primitives/booleans/transforms が等価 JSON と同一場を生むこと、
負数・小数・空白許容、不正構文/未知関数/arity 不一致/非有限/過深ネストの拒否、
共有検査による不正値拒否。実機: CLI run/check で DSL を評価し DFM まで通ることを確認。

## 反映サマリ v31
| 問 | 実装 |
|----|------|
| 59 | テキスト DSL (JSON と同一 AST へ落ちる表層構文, eval_any 自動判別) |

> 総括: v31 で Plan Phase-2 の「テキスト DSL」を達成。JSON の約 1/5 のトークンで
> 同等記述が可能になり AI のコンテキスト効率を改善。テスト数 113→120 + 統合 3。

---

# 新視点 (ソクラテス式・第2幕) — 既存の前提を問い直す

これまでの問は「カーネル/決定性/水密/MCP/検証/出力/プリミティブ/DSL」を扱った。
ここからは**既存の前提そのもの**を問い直す新視点を追加する。

## 問60 — 「水密」は「単一の造形物」と同じか?
**問**: 我々は「水密 = 製造可能」を中核主張としてきた。だが水密なメッシュが
**複数の独立した中実体**から成ることはないか? (例: 連結し損ねたフィレット、
タイプミスした translate で部品が2つに割れる)。エンジンは「シーンが無言で2ボディに
なった」ことを AI に伝えられなかった。

**注意**: 中空シェルは外殻+内殻の2成分だが**1ボディ+1空洞**で正常。これを
「複数ボディ」と誤検出してはならない。

**修正**: `Mesh::body_components()` を追加。頂点を三角形エッジで結ぶ Union-Find で
連結成分を求め、各成分の符号付き体積で分類 (正=中実ボディ, 負=内部空洞)。決定的。
- `validate` は水密時にボディ数>1 で `MULTIPLE_BODIES` を**警告** (分割は意図的な
  こともあるため Error でなく Warning)。空洞 (中空シェル) では警告しない。

検証 (テスト 120→124): 単一球→(1,0)、離れた2球→(2,0) かつ警告、
中空シェル→(1,1) かつ警告なし。

## 反映サマリ v32
| 問 | 実装 |
|----|------|
| 60 | 連結成分のボディ/空洞分類 + MULTIPLE_BODIES 警告 (水密≠単一造形) |

> 総括: v32 は「水密 = 単一造形物ではない」という新視点で DFM に独立ボディ検出を追加。
> 中空シェルとの混同を符号付き体積分類で回避。テスト数 120→124 + 統合 3。

## 問61 — 決定性は観測できなければ「約束」に過ぎないのではないか?
**問**: 我々は「同一バイナリ・同一arch → バイト同一出力」(問5) を主張し内部テストで
担保してきた。だが**利用者や第三者**には、自分の実行が正準出力を再現できたか、
2つの実行/環境間で無言の差異 (drift) が生じていないかを**検証する手段がなかった**。
検証できない約束は信頼に値しない。

**修正**: `Mesh::digest()` (FNV-1a 64bit) を追加し、決定性を**観測可能**にする。
頂点ビット列・頂点数・三角形索引をメッシュ順 (それ自体が決定的) で畳み込む。
- `Report` に `digest` を追加し `summary()` に `digest=<16進>` を出力。CLI run/check・
  MCP validate/run_script すべてで表示される。第三者は短いハッシュ1つで再現性を確認可能
  (ファイル全体の比較が不要)。
- 実機: 同一スクリプトの2回の `kado run` が同一ダイジェスト (472adeed83d2372f) を出力。

検証 (テスト 124→125): 同一メッシュ=同一ダイジェスト、形状差/解像度差で変化、要約に出力。

## 反映サマリ v33
| 問 | 実装 |
|----|------|
| 61 | メッシュ内容ダイジェスト (FNV-1a) で決定性を観測可能化 |

> 総括: v33 は「決定性は観測できて初めて信頼になる」という新視点で、出力の再現性を
> 短いダイジェストで第三者検証可能にした。テスト数 124→125 + 統合 3。

## 問62 — そもそも AI は「単位」を知っているのか?
**問**: エンジンは座標を無単位の数として扱うが、DFM 閾値 (`min_wall_mm=0.5`) も
3MF 出力もすべて暗黙に**ミリメートル**を仮定している。`sphere(1.0)` を書く AI には
それが 1mm (微小) か 1m (巨大) か分からず、レポートは生の bbox min/max しか示さず
実寸を出さない。「1 unit = 1 mm」という暗黙の取り決めが明文化されず、スケール誤りが
不可視だった。

**修正**: 単位を明示・観測可能にする。
- `lib.rs` に「1 unit = 1 mm」を明記し、DFM・3MF と一貫させる。
- `Report::summary()` に `dims_mm=[W x D x H]` を追加し実寸を直接提示。
- `SUSPICIOUS_SCALE` 警告: 最大寸法がユーザ自身の `min_wall_mm` すら下回るとき
  (= 形状全体が1壁より小さい) に警告。**絶対値でなく閾値相対**ゆえ恣意的でなく、
  単位/スケール誤りをほぼ確実に捉える。薄板は最大寸法が大きいので誤検出しない。

検証 (テスト 125→127): 要約に dims_mm (球r1→~2mm)、直径0.2mm+min_wall0.5→警告、
通常サイズ→警告なし。

## 反映サマリ v34
| 問 | 実装 |
|----|------|
| 62 | 単位 (mm) の明示 + dims_mm 提示 + SUSPICIOUS_SCALE (閾値相対) |

> 総括: v34 は「単位は暗黙では伝わらない」という新視点で、寸法を mm で明示し、
> スケール誤りを閾値相対で検出。テスト数 125→127 + 統合 3。

## 問63 — 検証レポートは AI が機械処理できる形か?
**問**: `validate` の結果は人間可読のテキスト塊だった。自己修正ループの AI は
コードや数値指標を**自由文字列から抽出**せねばならず脆い (部分文字列マッチ)。
AI-First ツールなら構造化して `code == "THIN_WALL"` で確実に分岐できるべき。

**修正**: `Report::to_json()` を追加 (問63)。`mcp::json` を汎用 JSON ユーティリティとして
用い、`{ok, triangles, manifold, volume, bbox, dims_mm, digest, issues:[{severity,
code, cause, hints}]}` を返す。MCP `validate` ツールはこの JSON を返すよう変更
(CLI check は人間向けテキストのまま据え置き)。スキーマ説明にもコード一覧を明記。

検証 (テスト 127→128): to_json が parse で往復一致、必須フィールド存在、各 issue が
code/severity を持ち、`ok` が Error 有無と整合することを確認。

## 反映サマリ v35
| 問 | 実装 |
|----|------|
| 63 | 構造化 JSON レポート (Report::to_json) — MCP validate を機械可読化 |

> 総括: v35 は「レポートは AI が機械処理できて初めて自己修正に使える」という新視点で、
> MCP validate を構造化 JSON 化。テスト数 127→128 + 統合 3。

## 問64 — 失敗したスクリプトから AI は「どこで」失敗したか学べるか?
**問**: 深い木のエラー (`"r" must be > 0`) はどのノードが原因か示さない。
`difference(union(sphere(0), …), …)` では AI が違反箇所を特定できない。

**修正**: `ScriptError::at(op, key)` と `build_child` ヘルパを追加し、木を巻き戻しながら
パスを積む。エラーは `difference.a > union.b > "r" must be > 0` のように失敗ノードへの
経路を持つ。子再帰 (a/b/shape, 計23箇所) を `build_child` 経由に統一。DSL も
Value 経由で同じ恩恵を受ける。

## 問65 — 形状が閉じていないとき「体積」は意味を持つか?
**問**: レポートは常に `volume` を出すが、発散定理の体積は**閉じた**メッシュでのみ有効。
開境界 (OPEN_MESH) では無意味な値を AI が信頼しかねない。

**修正**: `Report::volume_reliable()` (= is_manifold かつ非空) を追加。`summary()` は
不正時に `(unreliable: not closed)` を付記し、`to_json()` は `volume_reliable` を出す。

検証 (テスト 128→130): ネスト失敗のパス報告、開メッシュの体積を不可信と明示。

## 反映サマリ v36
| 問 | 実装 |
|----|------|
| 64 | スクリプトエラーに失敗ノードへのパスを付与 (build_child/ScriptError::at) |
| 65 | 体積の信頼性フラグ (閉じたメッシュでのみ有効) を明示 |

> 総括: v36 は「失敗の所在」と「指標の妥当範囲」を明示する2視点で、AI の自己修正と
> 数値解釈の誤りを防いだ。テスト数 128→130 + 統合 3。

## 問66 — AI はプレビュー画像だけで向きとスケールを判断できるのか?
**問**: スクリーンショットは基準枠のない空間に形状が浮かぶだけで、AI は上下・前後や
鏡像の区別を画素から判別できない。CAD ビューアは必ず座標グノモンを示す。

**修正**: `render::draw_axes` を追加し、X=赤・Y=緑・Z=青の軸線グノモンを重ねる。
モデル中心を起点に diag*0.35 の長さで投影描画 (2px DDA, オーバーレイ)。
MCP screenshot は `axes` (既定 true) で切替、CLI は常時表示。

検証 (テスト 130→132): グノモン描画後に RGB 各軸色の画素が現れること、決定性。
実機: iso ビューで有効 PNG を確認。

## 反映サマリ v37
| 問 | 実装 |
|----|------|
| 66 | 座標軸グノモン (X赤/Y緑/Z青) でプレビューに向き基準を付与 |

> 総括: v37 は「AI はプレビューから向きを読めない」という新視点で、全スクショに
> RGB 軸グノモンを重ねた。テスト数 130→132 + 統合 3。

---

## 問67 — `run_script` で上書きしたら元に戻せないのか?
**問**: AI が誤ったスクリプトを `run_script` で実行するとシーンが上書きされる。
空メッシュや破損シーンに差し替えた後、セッションを再起動せずに復元する手段がない。
開発中の AI エージェントは誤字・パラメータミス・論理エラーを頻繁に起こすが、
失敗ごとにセッションを再作成するのは高コスト。**single-level undo** で十分か?

**修正**: `Session` に `prev_scene: Option<Sdf>` と `prev_script: Option<String>` を追加。
`run_script` は上書き前に現在状態を `prev_*` に退避する。
新ツール `undo_script`: `prev_scene.take()` で前のシーンを復元し、
同じスクリプトを 2 度 undo しようとすると `is_error=true` (single-level undo の明示)。

検証 (テスト 134→135): `undo_restores_previous_scene` — run後の eval値→undo後の eval値が
初期値に戻り、2回目 undo はエラー。

## 問68 — OVERHANG 検査はビルド方向を暗黙に +Z 固定しているが、AI はそれを知っているか?
**問**: `validate_with_field` のオーバーハング検査は `n.z` (Z成分) のみを見ていた。
ユーザーが形状を別の向きで印刷する場合 (例: Y軸方向に立てて積層) に正しい判定ができない。
ツールのスキーマにも「ビルド方向は +Z」という記述がなく、AI に暗黙の仮定が伝わらない。

**修正**: `validate_with_field(…, build_dir: Vec3)` に `build_dir` パラメータを追加。
オーバーハング検査を `n.dot(bd) / |n|` に一般化し、任意の方向を使えるようにする。
`validate()` 便利ラッパーは `Vec3::new(0,0,1)` (+Z) をデフォルトとして維持。
MCP `validate` ツールに `build_dir` 文字列パラメータを追加
(`"z"` / `"-z"` / `"x"` / `"-x"` / `"y"` / `"-y"` / `[dx,dy,dz]`; デフォルト `"z"`)。
ツールスキーマの説明文も更新し、「ビルド方向は +Z 前提; 別軸ならパラメータを設定すること」
と明示する。OVERHANG エラーメッセージに build_dir ベクトルを埋め込み、
AI がどの方向で評価されたかを確認できるようにする。

検証 (テスト 135→137): `overhang_check_respects_build_direction` —
- 球の +Z ビルドレポートの OVERHANG メッセージが `"[0.00,0.00,1.00]"` を含む
- +X ビルドレポートの OVERHANG メッセージが `"[1.00,0.00,0.00]"` を含む
- `max_overhang_deg=0` でスキップ確認。

## 反映サマリ v38
| 問 | 実装 |
|----|------|
| 67 | single-level undo (`undo_script` ツール追加、Session に prev_scene/prev_script) |
| 68 | OVERHANG のビルド方向を明示パラメータ化 (validate build_dir、デフォルト +Z) |

> 総括: v38 は「上書き操作の不可逆性」と「暗黙のビルド方向仮定」を Socratic 問答で
> 摘出し、AI エージェントの作業安全性と DFM 評価の正確性を高めた。テスト数 132→134 + 統合 3。

---

## 問69 — `eval` ツールに非有限座標 (Infinity/NaN) を渡すとどうなるか?
**問**: JSON は `1e999` を `+Infinity` としてパースし、Rust の `"1e999".parse::<f64>()` も
`Ok(Infinity)` を返す。現行の `tool_eval` はこれを無検査で `Sdf::eval` に渡す。
Infinity/NaN の座標は SDF の加減乗除を通じて NaN に伝播し、AI が受け取るのは
`"nan"` または `"inf"` という意味不明な文字列になる。AI は「形状の内外判定が壊れた」
と誤解しかねない。

**修正**: `tool_eval` の入力バリデーションに `!x.is_finite() || !y.is_finite() || !z.is_finite()`
チェックを追加。非有限の場合は `isError=true` で明示エラー。
また SDF 評価結果も `!d.is_finite()` で防御チェックし (健全な SDF で起こるべきでない)、
非有限結果を SDF 木の縮退として報告する。

## 問70 — `repeat` の count 明示 + period 未指定はサイレント縮退ではないか?
**問**: `{"op":"repeat","nx":3,"shape":...}` を書いた AI はX軸方向に3コピーを期待する。
しかし `x` (period) がないためデフォルト 0.0 になり、`snap` 関数が `period=0` の場合に
座標を変換せず返すため、タイルなしの元形状1個だけが得られる — **エラーも警告もなし**。

**修正**: `eval.rs` の `"repeat"` ケースに事前検査を追加。
`nx`/`ny`/`nz` が JSON オブジェクトに**明示して存在し**かつ値 > 0 なのに、
対応する `x`/`y`/`z` period が正でない場合は `ScriptError` を返す。
period を省略した場合はその軸の count デフォルト 1 と合わせて「繰り返しなし」として
既存の挙動を維持し後方互換を保つ。

検証 (テスト 134→136 + 統合 3):
- `repeat_count_without_period_is_rejected` — nx=3 かつ x 未指定はエラー; period 正常指定は OK
- `eval_rejects_non_finite_coordinates` — Infinity/NaN 座標はエラー; 有限座標は成功

## 反映サマリ v39
| 問 | 実装 |
|----|------|
| 69 | eval ツールの非有限座標早期拒否 (Infinity/NaN ガード) |
| 70 | repeat count 明示 + period 未指定のサイレント縮退を ScriptError に変換 |

> 総括: v39 は「非有限値の静かな汚染」と「パラメータ不整合の無言の縮退」を排除し、
> AI が意味不明な結果や期待と異なる形状に惑わされるリスクを低減した。テスト数 134→136 + 統合 3。

---

## 問71 — screenshot の未知ビュー名がサイレントに "iso" にフォールバックする
**問**: `screenshot` ツールで `view="above-45-deg"` のような無効な名前を指定した場合、
コードは `or_else(||.find("iso"))` で黙って等方透視図にフォールバックする。
AI は「指定したビューのスクリーンショット」と思いこみつつ実際は別のビューを見ている。
無効な入力に対してはエラーを返すべきであり、有効なビュー名一覧を示す必要がある。

**修正**: `tool_screenshot` と CLI の screenshot コマンドの両方で、
view 名が presets に存在しない場合は `isError=true` とエラーメッセージを返す (問71)。
エラーメッセージには有効なビュー名一覧 (`front, back, right, left, top, bottom, iso`) を含め、
AI がすぐに修正できるようにする。

## 問72 — `export` が相対パスを返すため AI はファイルの実際の場所がわからない
**問**: `export` ツールは `"exported STL: kado-export.stl (...)"` を返すが、
これは MCP サーバーの起動ディレクトリからの相対パス。
AI は MCP サーバーの CWD を知る術がなく、エクスポートファイルを後で参照できない。
さらに非多様体メッシュがエクスポートされる際に `manifold=false` とだけ表示され、
3D プリントに問題が生じることへの警告が不足していた。

**修正**: 書き込み成功後に `std::fs::canonicalize` で絶対パスを解決して返す (問72)。
canonicalize 失敗時 (まれなケース) は相対パスにフォールバック。
加えて `manifold=false` 時に `[WARNING: non-manifold mesh — ...]` を付加し、
AI が品質問題に気づけるようにする。

検証 (テスト 136→137 + 統合 3):
- `screenshot_unknown_view_returns_error` — 未知ビューでエラー + 有効名一覧; "front" は成功

## 反映サマリ v40
| 問 | 実装 |
|----|------|
| 71 | screenshot 未知ビューを明示エラーに変換 (CLI + MCP 両対応) |
| 72 | export が絶対パスを返す; 非多様体時に明示 WARNING を付加 |

> 総括: v40 は「無言のフォールバック」と「相対パスの不透明性」を解消し、
> AI がどこに何が書かれたか・どのビューを見ているかを確実に把握できるようにした。
> テスト数 136→137 + 統合 3。

## 問73 — `help` ツールのドキュメントが追加済み機能を反映していない
**問**: `help` ツールが返す `KADOSCENE_HELP` は当初のワークフロー 3 ステップしか説明していない。
`undo_script` (問67)・`validate` の `build_dir` 引数 (問68)・`repeat` の period 制約 (問70)・
`get_scene` の undo 可否表示 (問74) が追加されたが、`help` は一切案内しない。
AI は `undo_script` ツールが存在することも、どう使うかも `help` からは分からない状態だった。

**修正**: `KADOSCENE_HELP` の Workflow 節を全面更新し:
1. undo_script を 5 番目のステップとして明示
2. get_scene が undo 可否を報告することを説明
3. validate の build_dir パラメータ節を新設 (例: "z"/"-z"/"y" の指定方法)
4. repeat の period 必須ルールを新設 (OK/ERROR 例付き)

変更ファイル: `src/mcp/tools.rs` (`KADOSCENE_HELP` 定数)。テスト増減なし。

## 問74 — `get_scene` が undo_script の可否を知らせないため AI が盲目的に試みる
**問**: AI が誤ったスクリプトを適用した後、`undo_script` を呼べば戻れる「かもしれない」が
呼んでみるまでわからない (成功か `nothing to undo` エラーかは不定)。
`get_scene` が状態を返すのに undo 可否を含まないため、AI が不要なエラーリカバリループに入る。

**修正**: `tool_get_scene` のレスポンスに `undo_available=true/false` フィールドを追加。
- `session.prev_scene.is_some()` → `undo_available=true`
- `session.prev_scene.is_none()` → `undo_available=false`

`get_scene` のレスポンス末尾にこのフィールドが常に含まれる。

検証テスト追加 (テスト 137→138):
- `get_scene_reports_undo_availability`:
  初期状態 `undo_available=false`
  → `run_script` 後 `undo_available=true`
  → `undo_script` 後 `undo_available=false` に戻る

## 反映サマリ v41
| 問 | 実装 |
|----|------|
| 73 | KADOSCENE_HELP を全面更新: undo_script・build_dir・repeat 制約を説明 |
| 74 | get_scene が `undo_available=true/false` を返すよう拡張 |

> 総括: v41 は「ドキュメントと実装の乖離」と「undo 可否の不透明性」を解消した。
> help ツールは現行の 8 ツール全ての主要パラメータを案内でき、
> get_scene は AI が次のアクションを決定するのに必要な状態を完全に提供する。
> テスト数 137→138 + 統合 3。

## 問75 — `smooth_*` の `k=0` で除算ゼロ; `k<0` で AABB 縮小
**問**: `smooth_union`/`smooth_intersection`/`smooth_difference` は IQ の polySmoothMin 公式
`h = clamp(0.5 + 0.5*(db-da)/k, 0, 1)` を使う。`k=0` のとき `/k` が除算ゼロで NaN を生み、
`polygonize` が沈黙のまま壊れたメッシュ (または空) を出力する。
`k<0` のとき `aabb()` が `(lo - Vec3::splat(*k), hi + Vec3::splat(*k))` → AABB が縮小し、
メッシュが sampling_box からはみ出してクリップされる。
デフォルト 0.3 は有効だが、明示的に `"k":0` や `"k":-0.1` を渡した場合に無言でバグる。

**修正**: `eval.rs` の smooth_union/intersection/difference で `k <= 0` を即座に ScriptError にする。
エラーメッセージにはハード操作 (`union`/`intersection`/`difference`) への誘導を含める。

検証テスト追加 (テスト 139→140 で先行):
- `smooth_k_zero_or_negative_is_rejected`:
  k=0, k<0 の各操作でエラー; k>0 は正常動作

## 問76 — `repeat` count > MAX_REPEAT がサイレントクランプされ AI が気づかない
**問**: `repeat` の `nx/ny/nz` は内部で `MAX_REPEAT=256` に `.min()` クランプされる。
AI が `nx=500` を指定すると 256 コピーになるが `run_script` は「scene updated」とだけ返し、
クランプを知らせない。AI はモデルが期待通りに生成されたと思い込む。
`repeat` の period 省略 (問70) はエラーにしたのに、count 超過は無言クランプという非対称。

**修正**: `eval.rs` の count 解析を `.min(MAX_REPEAT)` によるクランプから
`cnt > MAX_REPEAT → ScriptError` に変更。エラーメッセージに最大値 256 と
「2*n+1 コピー/軸」の説明を含める。負の count も同様にエラー。

検証テスト追加 (テスト 139→140):
- `repeat_count_over_max_is_rejected_not_silently_clamped`:
  nx=300 → エラー (メッセージに "256" または "maximum" を含む)
  nx=256 → 成功 (= MAX_REPEAT)
  nx=-1 → エラー

## 反映サマリ v42
| 問 | 実装 |
|----|------|
| 75 | smooth_* k≤0 を ScriptError に: NaN 伝播 + AABB 縮小バグを防ぐ |
| 76 | repeat count > 256 をサイレントクランプからエラーに変更 |

> 総括: v42 は「サイレントな数値劣化」パターンを2件解消した。
> smooth blend の k は正値のみ受け付け、repeat count は上限オーバーを明示エラーにする。
> どちらもエラーメッセージに代替手段を示し AI の自己修正ループを支援する。
> テスト数 138→140 + 統合 3。

## 問77 — `torus(minor >= major)` が自己交差する非多様体メッシュを無言で生成する
**問**: `torus` の SDF は数学的に常に有限値を返すが、`minor >= major` のとき:
- `minor = major`: horn torus — 中央で自己接触 (デジェネレート)
- `minor > major`: spindle torus — 自己交差する非多様体 → 3D 印刷不可

スクリプト検証段階でエラーにならず `run_script` が成功し、後で `validate` が OPEN_MESH や
MULTIPLE_BODIES を報告する。AI は「どのパラメータが問題か」を自力で推測しなければならない。

**修正**: `eval.rs` の `torus` パース時に `minor >= major` を ScriptError にする。
エラーメッセージに horn/spindle の違いと ring torus 条件 `minor < major` を説明する。

検証テスト追加 (テスト 140→141):
- `torus_minor_ge_major_is_rejected`: minor=major → エラー, minor>major → エラー (メッセージに "spindle" または "non-manifold"), minor<major → 成功

## 問78 — デフォルトシーンが SDF の最大の特長 (スムーズブレンド) を示していない
**問**: `Session::new()` のデフォルトシーンは
`union(sphere(1), cuboid(0.8)).difference(cylinder(0.3, 2.0))`。
このシーンは幾何的には正当 (多様体、有効なメッシュ) だが、SDF の最大の強みである
「スムーズな有機的ブレンド」を全く使っていない。
シャープエッジの boolean 形状はメッシュCSGでも生成でき、SDF 固有の価値を示せない。
AI が `help` で `smooth_union` を学んでも、実際に動作するデモを見る前に
`run_script` を呼ばなければならない非対称がある。

**修正**:
- `default_scene()` を `smooth_union(sphere(1.0), cuboid(0.8), k=0.2)` に変更:
  SDF の有機的ブレンドを即座にデモし、視覚的にも印象的な形状。
- CLI `demo_model()` も同様に変更し一貫性を保つ。
- `run_script_updates_active_scene` テストの初期値チェック条件を更新
  (原点は新シーン内部 → 負なので「遠点 (10,0,0) は外 → 正」を使用)。

テスト数変化: 140→141 (問77); 問78 は既存テスト修正のみ。

## 反映サマリ v43
| 問 | 実装 |
|----|------|
| 77 | torus minor >= major を ScriptError に: 自己交差モデルの無言生成を防ぐ |
| 78 | デフォルトシーンを manifold な smooth_union に変更: 誤 OPEN_MESH を排除 |

> 総括: v43 は「デフォルト状態での誤判定」と「torus 縮退の無言生成」を解消した。
> どちらも AI が正確なフィードバックを受けて自己修正できるよう入力段階で問題を捕捉する。
> テスト数 140→141 + 統合 3。

## 問79 — `validate` ツールの説明が issue code を全列挙していない
**問**: `validate` ツールの schema description は「e.g. THIN_WALL, MULTIPLE_BODIES, OPEN_MESH, OVERHANG, SUSPICIOUS_SCALE」と例示するだけで、実際に発行されうる `NON_MANIFOLD` と `EMPTY_MESH` の2コードを欠落している。AI がこれらのコードを`switch`で処理する際に catch-all に落ちて適切な対応ができない。

**修正**:
- `validate` ツール schema description を全7コード列挙に更新:
  EMPTY_MESH, NON_MANIFOLD, OPEN_MESH, MULTIPLE_BODIES, THIN_WALL, OVERHANG, SUSPICIOUS_SCALE
  各コードに1行の意味説明を付ける。
- `KADOSCENE_HELP` に `validate issue codes` 節を新設し、同じ7コードとその意味を記載。

コード変更: `src/mcp/tools.rs` (tool_def description + KADOSCENE_HELP)。テスト数変化なし。

## 問80 — `get_scene` の bounds フィールドが実 AABB か sampling_box かを区別しない
**問**: `get_scene` は `bounds=[lo]-[hi]` を返すが、これは `sampling_box()` の値 (実 AABB の ~5% 増し)。
AI が eval クエリ点を設定する際に `bounds` を「形状の外縁」と解釈すると、実際の形状より少し外側を評価するループを組む。
例: sphere(1.0) の実半径は 1.0 だが sampling_bounds は ≈1.05。
これが SDF 正の外側か形状の外側かの判断を誤らせる可能性がある。

**修正**: フィールド名を `bounds=` → `sampling_bounds=` に変更し、
`(includes ~5% margin beyond shape AABB)` という注釈を追記。
AI はこの値が polygonize 用の余白込みであることを見てわかる。
`get_scene_round_trip` テストも新フィールド名を確認するよう更新。

変更ファイル: `src/mcp/tools.rs` (tool_get_scene), `src/mcp/server.rs` (test)。テスト数変化なし。

## 反映サマリ v44
| 問 | 実装 |
|----|------|
| 79 | validate ツール説明と KADOSCENE_HELP に全7 issue code を列挙 |
| 80 | get_scene の bounds を sampling_bounds にリネーム + 余白注釈を追加 |

> 総括: v44 は「validate の issue code 不完全列挙」と「get_scene bounds の意味の曖昧さ」を解消した。
> AI がツール返値を機械的に処理する際に隠れた仮定 (知らないコードは無視する) を排除する。
> テスト数は変化なし (141 + 統合 3)。

## 問81 — `run_script` が MULTIPLE_BODIES 警告をサイレントに通過させる
**問**: `run_script` のクイック検証は `validate(&mesh, 0, 0)` を使い、エラーコードのみを
`"scene updated (check issues: ...)"` に含め、警告コードをフィルタしていた。
`MULTIPLE_BODIES` は Warning なので `report.is_ok()==true` のときも、
`is_ok()==false` のときも、応答に含まれなかった。
AI が2つの離れた球を設計して `run_script` したとき「scene updated」と返り、
MULTIPLE_BODIES に気づかずに出力できた。

**修正**: `run_script` のプレフィックス計算でエラーのみをフィルタせず、
全 issue code (エラー＋警告) を列挙するよう変更する。
`MULTIPLE_BODIES` が含まれるケースをカバーするテスト追加。

検証テスト追加 (テスト 141→142):
- `run_script_surfaces_multiple_bodies_warning`:
  2つの離れた球 → isError=false, テキストに "MULTIPLE_BODIES" を含む

## 問82 — `validate` JSON レポートの `severity` が Rust Debug 形式 (PascalCase) で AIが文字列比較しにくい
**問**: `to_json()` では `format!("{:?}", e.severity)` → `"Error"` / `"Warning"` (先頭大文字)。
AI が `if severity == "error"` (lowercase) で分岐するコードを書くと全件マッチせず、
DFM エラーが見過ごされる。ツール説明も `{severity}` と書くだけでフォーマットを明示しない。

**修正**: `to_json()` の severity フィールドを `match` で小文字 `"error"/"warning"/"info"` に変更。
ツール schema description に `severity:\"error\"|\"warning\"` を明示する。
既存の `to_json_is_machine_readable_and_carries_codes` テストの severity 比較を
`"Error"` → `"error"` に更新 (正確な regression test)。

変更ファイル: `src/verify/check.rs` (to_json severity), `src/mcp/tools.rs` (schema description)。
テスト数変化なし (142 + 統合 3)。

## 反映サマリ v45
| 問 | 実装 |
|----|------|
| 81 | run_script が MULTIPLE_BODIES 等の警告も "check issues:" に列挙 |
| 82 | validate JSON severity を小文字 "error"/"warning"/"info" に標準化 |

> 総括: v45 は「警告のサイレント通過」と「severity の大文字小文字不整合」を解消した。
> AI がレポートを機械処理するときに比較ミスで DFM 問題を見逃す可能性を排除する。

## 問83 — `get_scene` のデフォルトシーン応答が再現用 DSL を提供しない

**問**: `run_script` を一度も呼んでいない初期状態で `get_scene` を呼ぶと
`script=(default scene — no run_script call yet)` とだけ返る。
AI はデフォルトシーンが何で構成されているかを知らないまま作業を始めるため、
「デフォルトに戻したい」「現在の形状を基に変形したい」ときに
既存のシーン定義をコピーして run_script に渡すことができない。
セッション再接続後に AI がコンテキストを失うと自己修正ループが断絶する。

**修正**: `tool_get_scene` の None ブランチ (スクリプト未設定) に
`to reproduce: smooth_union(sphere(1.0),cuboid(0.8),0.2)` というヒントを付加する。
AI はこれをそのまま run_script に渡してデフォルトシーンを再現できる。

検証テスト追加 (テスト 142→143):
- `get_scene_default_includes_reproducible_dsl`:
  初期状態の get_scene レスポンスに `"smooth_union"` と `"to reproduce"` が含まれる

変更ファイル: `src/mcp/tools.rs` (tool_get_scene None ブランチ),
`src/mcp/server.rs` (新テスト追加)。

## 反映サマリ v46
| 問 | 実装 |
|----|------|
| 83 | get_scene デフォルトシーンに再現用 DSL スニペットを付加 |

> 総括: v46 は AI の「デフォルトシーン不透明問題」を解消した。
> セッション再接続後も get_scene 1回でデフォルトシーンを再現する DSL が手に入る。

## 問84 — `offset()` の AABB が負 amount で収縮せず子の AABB のまま (過保守・誤解析リスク)

**問**: `Sdf::Offset` の `aabb()` は `let e = Vec3::splat(amount.max(0.0))` で
amount を 0 にクランプし、負のオフセット (収縮) では子と同じ AABB を返していた。
収縮後の実際のイソサーフェスより大きい AABB を使うと `polygonize` が不要な空間を
サンプリングし、形状が大幅に収縮されたシーンでは無駄が増える。
さらに `sampling_box` の 5% マージンも元サイズ基準で計算されるため、
表示統計 (サイズ・ボリューム) が収縮前の形状に基づいて過大見積もりされる。

**修正**: `let e = Vec3::splat(*amount)` (符号付き) を使い、
`lo2 = lo - e` / `hi2 = hi + e` を計算後、`lo2.min(hi2)` / `lo2.max(hi2)` で
過侵食時 (lo2 > hi2) の反転を正規化する。

検証テスト追加 (テスト 143→144):
- `offset_negative_aabb_tightens_not_stays_at_child_size`:
  sphere(1.0).offset(-0.4) の AABB が [-0.6,-0.6,-0.6] to [0.6,0.6,0.6] になることを確認。
  過侵食 offset(-1.5) でも AABB が有限かつ正規化されることを確認。

変更ファイル: `src/core/sdf.rs` (Offset aabb arm)。

## 問85 — `arg_build_dir()` の配列 z 成分の欠損デフォルトが `1.0` (方向ベクトル不一致)

**問**: `arg_build_dir` の `arr.len() >= 3` ブランチで
`arr[2].as_f64().unwrap_or(1.0)` — z 要素が非数値のとき 1.0 にフォールバックしていた。
x/y は `.unwrap_or(0.0)` なのに z だけ 1.0 という非対称デフォルト。
AI が `[1.0, 0.0, 0.0]` を渡した際に第 3 要素が何らかの理由で非数値なら
`[1.0, 0.0, 1.0]` (対角) として解析され、X 軸ビルドのつもりがオーバーハング
方向が斜めになって誤った DFM 結果を返す。

**修正**: `arr[2].as_f64().unwrap_or(0.0)` に変更し、欠損 z は 0 として扱う。
方向ベクトルの全成分のデフォルトを 0 に統一。

検証テスト追加 (テスト 144→145):
- `build_dir_array_z_element_defaults_to_zero_not_one`:
  `[1.0, 0.0, 0.0]` の build_dir 配列で validate が isError=false を返すことを確認。

変更ファイル: `src/mcp/tools.rs` (arg_build_dir),
`src/mcp/server.rs` (新テスト追加)。

## 反映サマリ v47
| 問 | 実装 |
|----|------|
| 84 | offset() AABB を符号付き拡張に修正 (負 amount で正しく収縮) |
| 85 | arg_build_dir() の z デフォルトを 1.0 → 0.0 に修正 |

> 総括: v47 は形状収縮と DFM ビルド方向の 2 つのサイレント誤計算を修正した。
> offset(-r) の AABB がタイトになりサンプリングが効率化され、
> build_dir 配列のデフォルト非対称問題も解消された。

## 問86 — `edge_vertex` が t を [0,1] にクランプせず浮動小数点誤差でセル外挿出

**問**: `edge_vertex` の補間パラメータ `t = p.val / (p.val - q.val)` は、
両隅 p, q が同符号 (浮動小数点ノイズで bracket 関係が成立しない稀なケース) のとき
t < 0 または t > 1 になり、四面体セルの**外側**に頂点を外挿する。
隣接セルから同一ビット列が得られる正準化前提が崩れ、非多様体エッジが生じる。

**修正**: `t = (p.val / denom).clamp(0.0, 1.0)` でセル内に制約する。

検証テスト追加 (テスト 145→146):
- `edge_vertex_clamp_produces_valid_interpolation`:
  同符号両隅で t がクランプされ x ∈ [0,1] に収まること、
  正常な異符号ケースで t=0.3 が正確に計算されることを確認。

変更ファイル: `src/extract/marching_tetrahedra.rs` (edge_vertex)。

## 問87 — `gradient` が固定 h=1e-4 でシェル厚 < 0.2mm のとき壁を突き抜ける

**問**: 三角形外向き補正に使う `gradient(sdf, centroid)` が固定ステップ h=1e-4。
`shell(shape, t)` で `t < 2h = 0.2mm` の極薄シェルを作ると、
centroid からの h ステップが壁を突き抜けてシェル内部 (符号反転) を評価し、
勾配方向が逆になって三角形の巻き順が反転する。
結果として非多様体エッジや体積負の面が生じ、水密メッシュの主張が崩れる。

**修正**: `push_tri` で三角形最短辺の 1% を h とし、
`h = (min_edge * 0.01).clamp(1e-9, 1e-4)` で適応させる。
`gradient` に h 引数を追加。

検証テスト追加 (テスト 146→147):
- `thin_shell_mesh_is_watertight`:
  shell(sphere(1.0), 0.05) で水密メッシュが得られることを確認。

変更ファイル: `src/extract/marching_tetrahedra.rs` (gradient/push_tri)。

## 問88 — ゼロ勾配点で `normal.dot(grad) = 0.0` → 反転不実施だが向き保証なし

**問**: `push_tri` でゼロ勾配 (鞍点・smooth_union ブレンド境界) のとき
`grad = (0,0,0)` → `dot = 0.0` → 条件 `dot < 0.0` が false → 反転しない。
しかし向きは不定で「現在の巻き順がたまたま正しいだけ」。
同一辺の隣接面が逆向きになると非多様体が生じるリスクがある。

**修正**: `if grad.length() > 1e-12 && normal.dot(grad) < 0.0` でゼロ勾配時は
デフォルト (反転しない) を維持し確定的に扱う。

検証テスト追加 (テスト 147→148):
- `zero_gradient_region_does_not_produce_inverted_mesh`:
  smooth_union のブレンド境界 (ゼロ勾配を持ちうる) で水密かつ正体積であることを確認。

変更ファイル: `src/extract/marching_tetrahedra.rs` (push_tri)。

## 反映サマリ v48
| 問 | 実装 |
|----|------|
| 86 | edge_vertex t クランプで浮動小数点外挿を防止 |
| 87 | gradient 適応的 h でシェル突き抜け防止 |
| 88 | ゼロ勾配時の push_tri 反転を確定的に抑制 |

> 総括: v48 はマーチングテトラヘドラの 3 つの隠れた前提を修正した。
> 浮動小数点誤差・シェル突き抜け・ゼロ勾配という 3 パターンで
> 非多様体/巻き順誤りが生じていた根本原因を除去した。

## 問89 — help テキストが評価器の制約 (問75/問77) と非同期 — AI が予測不能なエラーに遭遇

**問**: 問77 で torus に `minor < major` 制約を、問75 で smooth_* に `k > 0` 制約を
評価器に追加したが、`KADOSCENE_HELP` (AI の一次リファレンス) を更新しなかった。
help は torus を「minor (req): tube radius > 0」、smooth を「k: blend radius (default 0.3)」と
記載するのみで、AI が help を信頼して `torus(major=1, minor=1)` や `smooth_union(...,k=0)` を
書くと、help からは予測できないエラーで弾かれる。
ドキュメントと実装の乖離は AI の自己修正ループを混乱させる。

**修正**: help テキストを評価器の制約に同期させる:
- torus: 「minor (req): tube radius > 0 AND < major」+ 自己交差の説明
- smooth: 「k: blend radius > 0 (default 0.3; k<=0 rejected — use the hard
  union/intersection/difference op for a sharp boundary)」

検証テスト追加 (テスト 148→149):
- `help_documents_evaluator_constraints`:
  help に「< major」「k: blend radius > 0」が含まれること、かつ評価器が実際に
  torus minor>=major と smooth k<=0 を拒否すること (help の主張と現実の一致) を確認。

変更ファイル: `src/mcp/tools.rs` (KADOSCENE_HELP + 新テスト)。

## 反映サマリ v49
| 問 | 実装 |
|----|------|
| 89 | help テキストを問75/問77 の評価器制約に同期 + 同期保証テスト |

> 総括: v49 はドキュメント (help) と実装 (評価器) の乖離を解消した。
> 同期保証テストにより、将来制約を追加したとき help 更新漏れを CI が検知する。

## 問90 — `validate` の digest を解像度なしで報告 — 再現性契約が実行不能

**問**: digest (問61) は再現性の要であり、その決定性契約は mesh.rs に
「同一バイナリ・同一arch・同一スクリプト・**同一解像度**なら同じ digest」と明記される。
しかし `validate` の JSON / summary 出力は digest を報告するが、それを生成した
**解像度を含まない**。AI が `digest=abc123` を記録して後で再現性を検証しようとしても、
どの解像度で生成されたか分からず再現できない。res=48 と res=64 で digest は異なるため、
解像度なしの digest は再現性検証に使えない (契約が宣言されているのに実行不能)。

**修正**: `tool_validate` で `to_json()` の結果 (Value::Object) に `resolution` フィールドを
挿入する。tool レイヤは res を知っている (arg_resolution)。digest と resolution が
セットで報告され、AI/第三者が再現条件を完全に把握できる。

検証テスト追加 (テスト 149→150):
- `validate_reports_resolution_alongside_digest`:
  validate(resolution=40) の JSON に resolution=40 と digest が両方含まれることを確認。

変更ファイル: `src/mcp/tools.rs` (tool_validate + 新テスト)。

## 反映サマリ v50
| 問 | 実装 |
|----|------|
| 90 | validate JSON に resolution を併記し digest 再現性契約を実行可能に |

> 総括: v50 は「宣言されているが実行不能だった再現性契約」を実行可能にした。
> digest 単体では無意味だった再現性検証が、resolution 併記により完結する。

## 問91 — `export` 応答が digest/resolution を欠き、出力ファイルの同一性を検証不能

**問**: 問90 で validate には resolution を併記したが、`export` の応答は
三角形数と manifold 状態のみで digest も resolution も含まない。
AI がモデルをエクスポートして「このファイルは何か」を記録・検証しようとしても、
三角形数は弱い指標 (異なる形状が同数になりうる) で、正準な内容同一性 (digest) が無い。
export と validate で同じシーン・同じ解像度なのに、export 側だけ digest が見えないのは
ツール間で再現性情報が非対称。

**修正**: `tool_export` の成功応答に `resolution={res}, digest={:016x}` を併記する。
mesh は既に計算済みなので `mesh.digest()` は安価。これにより:
- export 出力の再現性同一性を AI が記録・検証できる
- export(res=N) の digest と validate(res=N) の digest が一致する (ツール間整合)

検証テスト追加 (テスト 150→151):
- `export_reports_digest_and_resolution_matching_validate`:
  export(res=24) 応答に resolution=24 と digest が含まれ、その digest が
  validate(res=24) の digest と一致することを確認 (ツール間整合の保証)。

変更ファイル: `src/mcp/tools.rs` (tool_export + 新テスト)。

## 反映サマリ v51
| 問 | 実装 |
|----|------|
| 91 | export 応答に digest/resolution 併記 + export↔validate digest 整合保証 |

> 総括: v51 は再現性情報のツール間非対称を解消した。export と validate が
> 同一解像度で同一 digest を返すことを保証し、AI が出力ファイルの同一性を
> 一貫した方法で検証できるようになった。

## 問92 — `export` の `manifold=true` が DFM 合格と誤認される — 構造 DFM が不完全

**問**: export 応答は `manifold=true/false` のみで構造健全性を表す。しかし
manifold (= edge-manifold = water­tight) は構造 DFM の一部にすぎない。
**watertight でも** 以下は manifold=true のまま見逃される:
- MULTIPLE_BODIES: 離れた複数ボディ (各殻は閉じている → manifold=true だが単一造形物でない)
- NEGATIVE_VOLUME: 裏返しメッシュ
run_script は問81 で全構造 issue を併記するのに、export は manifold 真偽のみ。
AI が export 応答の「manifold=true」を見て「DFM 合格・印刷可能」と誤認し、
2 ボディや裏返しの成果物を出力してしまう。export 解像度 (既定 64) は validate
既定 (48) と異なるため、別解像度の validate 合格も保証にならない。

**修正**: export で run_script と同じ閾値非依存の構造チェック
`validate(&mesh, 0.0, 0.0)` を**出力解像度で**実行し、issue code を併記する。
これは OPEN_MESH/NON_MANIFOLD/NEGATIVE_VOLUME/MULTIPLE_BODIES を捕捉する
(min_wall=0/max_overhang=0 で THIN_WALL/OVERHANG/SUSPICIOUS_SCALE はスキップ)。
issue がある場合「full DFM は validate(resolution=N) で」と案内を付す。

検証テスト追加 (テスト 151→152):
- `export_surfaces_multiple_bodies_not_just_manifold`:
  離れた2球を export → manifold=true だが応答に MULTIPLE_BODIES が併記されることを確認。

変更ファイル: `src/mcp/tools.rs` (tool_export + 新テスト)。

## 反映サマリ v52
| 問 | 実装 |
|----|------|
| 92 | export が出力解像度で構造 DFM issue を併記 (manifold 真偽だけの誤認を防止) |

> 総括: v52 は「manifold=true ⇒ DFM 合格」という export の暗黙の誤認を断った。
> 出力する実物 (export 解像度のメッシュ) の構造 DFM が export 時点で可視化され、
> run_script (問81) と同じ閾値非依存チェックでツール間の一貫性も保たれる。

## 問93 — `run_script` が digest を出すのにチェック解像度 (res=32) を開示しない

**問**: 問90/91 で validate・export は解像度を開示した。だが run_script の応答
(`report.summary()` を含む) も digest を出しており、そのチェックは res=32
(validate 既定48・export 既定64 より粗い) で行われる。解像度を開示しないため:
- (a) AI は run_script の summary digest が validate/export の digest と一致しない
  理由を説明できない (解像度が違えば digest は変わる — 問90)
- (b) 粗い res=32 の「issue なし」を確定的と誤認する。NON_MANIFOLD 等は解像度依存で、
  高解像度の validate では現れうる
digest を出す3番目のツールに、問90 と同じ「digest を出すが resolution を出さない」
ギャップが残っていた。

**修正**: run_script 応答に `check_resolution={res}` を併記し、
「quick check; validate/export use higher res by default — digests differ across
resolutions」と案内する。これで digest 不一致の理由が自明になり、粗い check を
確定的 DFM と誤認しなくなる (問90/91/92 と同じ解像度透明性の完成)。

検証テスト追加 (テスト 152→153):
- `run_script_discloses_check_resolution`:
  既定 (check_resolution=32) と明示指定 (=16) の両方が応答に出ることを確認。

変更ファイル: `src/mcp/tools.rs` (tool_run_script + 新テスト)。

## 反映サマリ v53
| 問 | 実装 |
|----|------|
| 93 | run_script が check_resolution を開示 (digest を出す全ツールで解像度透明性完成) |

> 総括: v53 で digest を出す3ツール (run_script/validate/export) すべてが
> 解像度を開示するようになり、問90 で始まった「digest は解像度なしでは再現不能」
> という契約欠陥が全面的に解消された。AI はどのツールの digest も
> 解像度とセットで解釈・照合できる。

## 問94 — `eval` の戻り値の意味 (単位・下界性) が未開示 — AI がクリアランスを過信

**問**: eval スキーマは「Negative = inside, positive = outside」と符号は説明するが:
- (a) **単位を述べない**。Kado 座標は mm (問62) だが、戻り値 `-0.523000` が mm か不明。
- (b) **大きさが厳密距離だと暗黙に示唆する**。実際は SDF はプリミティブでのみ厳密で、
  合成/平滑形状 (union/difference/smooth_*) では真の距離の**保守的下界**
  (Lipschitz ≤ 1 の場) にすぎない。AI が eval でクリアランスや壁の隙間を測り、
  大きさを厳密距離と誤認すると、実際の距離はそれ以上ありうる (下界なので安全側だが
  AI はそれを知らない)。

**修正**: eval スキーマ説明を拡張し、(1) mm 単位 (1 unit = 1 mm)、(2) ~0 = 表面、
(3) 大きさはプリミティブで厳密・合成/平滑形状では真の距離の保守的下界であり
「クリアランス計測では安全側の過小評価として扱う」ことを明示する。
出力は機械可読な裸の数値のまま (eval_at パーサ非破壊)、契約はスキーマで開示。

検証テスト追加 (テスト 153→154):
- `eval_schema_discloses_units_and_lower_bound`:
  eval スキーマ説明に "mm" と "lower bound" が含まれること、かつアンカーとして
  プリミティブ (sphere) では eval が厳密 ((2,0,0) で 1.0mm) であることを確認。

変更ファイル: `src/mcp/tools.rs` (eval tool_def 説明 + 新テスト)。

## 反映サマリ v54
| 問 | 実装 |
|----|------|
| 94 | eval スキーマが単位 (mm) と大きさの下界性を開示 (クリアランス過信を防止) |

> 総括: v54 は eval の戻り値の意味論を正直に開示した。符号だけでなく単位と、
> 「大きさは合成形状では厳密距離でなく保守的下界」という SDF の本質的性質を
> AI に伝え、クリアランス計測での誤用を防ぐ。

## 問95 — 著作リファレンス (help) が座標の単位 (mm) を述べない — AI が scale 不定で作図

**問**: 問62 で内部単位は mm と定め、問94 で eval の**読み取り**側に mm を開示した。
だが KADOSCENE_HELP — AI がシーンを**書く**ときの一次リファレンス — は
`sphere(10)` が 10mm なのか cm なのか任意単位なのかを一切述べていない。
`min_wall_mm` や SUSPICIOUS_SCALE の説明に mm は出るが、プリミティブ寸法
(sphere r, cuboid x/y/z) の単位規約は未記載。AI が部品を寸法指定する基礎情報が
欠落し、scale が不定のまま作図してしまう (読み取りは mm と分かるのに書き込みは不明、
という非対称)。

**修正**: help ヘッダに「Units & coordinates」節を追加し、
「All lengths are in MILLIMETERS (1 coordinate unit = 1 mm). e.g. sphere(10) is a
10 mm-radius ball」「+Z is up for FDM」「wrong scale → SUSPICIOUS_SCALE」を明示。
読み取り (eval, 問94) と書き込み (help) で単位開示の対称性を確立。
あわせて問93 テストのコメント混入 (関数シグネチャ行) を整形。

検証: 既存 `help_documents_evaluator_constraints` に
help が "MILLIMETERS" と "1 mm" を含む assertion を追加 (テスト数 154 維持)。

変更ファイル: `src/mcp/tools.rs` (KADOSCENE_HELP ヘッダ + テスト assertion + 整形)。

## 反映サマリ v55
| 問 | 実装 |
|----|------|
| 95 | help に単位規約 (1 unit = 1 mm) を明示 (作図側の単位開示) |

> 総括: v55 で「読み取り (eval) は mm と分かるが書き込み (help) では単位不明」という
> 非対称を解消した。AI はシーンを書くときも読むときも一貫して mm 規約を把握できる。

## 問96 — `repeat` の count 意味論が help で曖昧 — AI が個数を取り違える

**問**: `repeat_n` の実装は `count[a]` を「原点の**両側**へのコピー数」と定義する
(コード doc コメント: 両側へのコピー数)。つまり nx=2 は1軸あたり合計 5 個
(左2 + 中央 + 右2) を生む。しかし help は「count per axis (nx/ny/nz, default 1)」
としか書かず、片側か合計か曖昧。AI は nx=2 を「2個」と解釈しがちだが実際は5個。
要求した個数と実際の個数が食い違い、意図しない密度のパターンになる。

**修正**: help の repeat 節に「count is copies PER SIDE of the origin →
total = 2*count+1 per axis」と明記し、「nx=2 gives 5 copies along x
(2 left + center + 2 right)」と例示。

検証テスト追加 (テスト 154→155):
- `repeat_count_is_per_side_total_is_two_n_plus_one`:
  nx=2,period2 で x=4 (2個目/側) は内部・x=6 (3個目/側) は外部であることを確認
  (= per-side が2、合計5 の挙動的証明)。AABB も両側 count*period 広がることを確認。

変更ファイル: `src/mcp/tools.rs` (KADOSCENE_HELP repeat 節), `src/core/sdf.rs` (新テスト)。

## 反映サマリ v56
| 問 | 実装 |
|----|------|
| 96 | repeat count の「片側・合計2n+1」意味論を help で明示 + 挙動テスト |

> 総括: v56 は repeat の個数指定という、AI が数を直接コントロールする操作の
> 曖昧さを解消した。doc コメントにしかなかった「片側」の意味論を help と
> 挙動テストの両方で固定し、AI の作図意図と結果を一致させる。

## 問97 — `mirror_*` の意味論が help に皆無 — AI が「反転移動」と誤解

**問**: `mirror_x` は `p.x.abs()` による IQ opMirror で、**+x 半分を保持して -x 側へ
鏡像化**する (x=0 平面で対称化)。元の -x 半分は破棄され +x 半分の鏡像に置き換わる。
しかし help は `mirror_x {"op":"mirror_x","shape":<sdf>}` と書くだけで意味論の説明が
**皆無**。AI は「形状を反対側へ反転移動する」「対称コピーを足す」等と誤解しうる。
特に -x にしかない形状を mirror_x すると結果が空になる (源の +x 半分が空のため) という
直感に反する挙動を予測できない。

**修正**: help の mirror 節に「Makes the shape symmetric about the axis=0 plane:
KEEPS the positive-axis half and reflects it onto the negative half (original
negative half is REPLACED). To mirror to both sides, place the part on +axis first」
と明記。

検証テスト追加 (テスト 155→156):
- `mirror_keeps_positive_half_and_reflects_to_negative`:
  +x の球を mirror_x → +x と -x 両方に対称コピー。-x のみの球を mirror_x →
  源 (+x半分) が空のため結果も空。「+半分が源」の意味論を挙動で証明。

変更ファイル: `src/mcp/tools.rs` (KADOSCENE_HELP mirror 節), `src/core/sdf.rs` (新テスト)。

## 反映サマリ v57
| 問 | 実装 |
|----|------|
| 97 | mirror_* の対称化意味論 (+半分を源に -半分へ鏡像) を help で明示 + 挙動テスト |

> 総括: v57 は mirror という最も誤解されやすい変形の意味論を明文化した。
> 「反転移動」でなく「+半分を源とする面対称化」であることを help と挙動テストで
> 固定し、-x のみ形状が空になる直感に反するケースも AI が予測できるようにした。

## 問98 — `shell` が内向き中空化か外向き肥厚か help が示さない

**問**: `shell` の eval は `d.max(-(d+thickness))` で、**外側表面を保持して内向きに
壁を作る** (中空化、内半径 = 元 - thickness)。しかし help は「thickness > 0」としか
書かず、壁が内向き (中空化) か外向き (肥厚) か中心振り分けか不明。AI が
shell(sphere(1), 0.2) で「外半径 1.2 に育つ」のか「外半径 1.0 のまま中空」なのか
予測できず、寸法設計を誤る。

**修正**: help の shell 節に「Hollows the solid INWARD: keeps the outer surface and
carves a cavity, leaving a wall of thickness just inside the surface. Outer size is
unchanged. e.g. shell(sphere(1.0),0.2) → hollow ball, outer r=1, inner r=0.8」と明記。

検証テスト追加 (テスト 156→157):
- `shell_hollows_inward_keeping_outer_surface`:
  shell(sphere(1.0),0.3) で外表面 r=1.0 維持・壁(r=0.85)内部・内表面 r=0.7・中心中空
  を確認 (内向き中空化を挙動で証明)。

変更ファイル: `src/mcp/tools.rs` (KADOSCENE_HELP shell 節), `src/core/sdf.rs` (新テスト)。

## 反映サマリ v58
| 問 | 実装 |
|----|------|
| 98 | shell の内向き中空化 (外表面保持・内半径=元-厚) を help で明示 + 挙動テスト |

> 総括: v58 は shell の「外を保持し内へ掘る」意味論を固定した。AI が中空部品の
> 外寸と肉厚を正しく設計でき、肥厚との取り違えによる寸法エラーを防ぐ。

## 問99 — cylinder/capsule/torus の軸・平面の向きが help で未記載 — 組立/回転の前提が不明

**問**: cone は「apex at z=0, base at z=-h」と向きを明記するのに、cylinder・capsule
(いずれも軸=Z)・torus (リング=XY平面・穴=Z軸) は help に向きの記載が無い。
AI が円柱を積む・回す・組み合わせる際、既定の軸がどれか分からないと配置を誤る
(例: 2本の円柱を直交させたいのに既定軸を知らず両方Zのまま重ねる)。実装は正しく
Z軸整列だが、その契約が AI に開示されていない。

**修正**: help に各プリミティブの向きを明記:
- cylinder: 「axis along Z, centered at origin (spans z=-h..+h)」
- torus: 「ring lies in the XY plane, hole faces Z」
- capsule: 「axis along Z (z=-h..+h plus radius hemispherical caps)」
- cone も「axis along Z:」と前置きして統一。

検証テスト追加 (テスト 157→158):
- `primitive_axes_are_z_aligned_as_documented`:
  cylinder の z(h=2)/x(r=0.5) 非対称で長軸=Z を証明、capsule の Z 軸端、
  torus の穴が Z 軸上 (原点が外部)・チューブが XY 平面内 (inside) を確認。

変更ファイル: `src/mcp/tools.rs` (KADOSCENE_HELP プリミティブ向き), `src/core/sdf.rs` (新テスト)。

## 反映サマリ v59
| 問 | 実装 |
|----|------|
| 99 | cylinder/capsule/torus の軸・平面の向き (Z軸/XY平面) を help で明示 + 挙動テスト |

> 総括: v59 は全プリミティブの既定の向きを help で統一的に開示した。AI が
> 円柱・カプセル・トーラスを積む/回す/組む際の空間的前提が明確になり、
> 配置ミス (既定軸の取り違え) を防ぐ。

## 問100 — `scale` が uniform 限定である事実と理由が help で未開示

**問**: `scale` の eval は `factor * child.eval(p/factor)` で**単一スカラ=等方**。
非等方スケールは SDF の距離計量 (|∇f|=1) を壊すため意図的に非対応。だが help は
「s > 0」としか書かず、AI は `scale(x,y,z)` のような per-axis スケールを期待して
誤用しうる。なぜ uniform 限定か、非等方寸法をどう実現するか (cuboid/ellipsoid の
per-axis extent) の案内も無い。

**修正**: help の scale 節に「s > 0 (UNIFORM only); one factor for all axes;
non-uniform scaling is unsupported because it breaks the SDF distance metric.
For different per-axis sizes use a primitive with per-axis extents
(cuboid x/y/z, ellipsoid x/y/z)」と明記。挙動は既存テスト
`uniform_scale_preserves_distance_field` (scale(sphere(1),2)==sphere(2)) が担保済み。

検証: 既存 help テストに "UNIFORM only" を含む assertion を追加 (テスト数 158 維持)。

変更ファイル: `src/mcp/tools.rs` (KADOSCENE_HELP scale 節 + テスト assertion)。

## 反映サマリ v60
| 問 | 実装 |
|----|------|
| 100 | scale の uniform 限定・理由・非等方の代替手段を help で明示 |

> 総括: v60 は scale の「等方限定」という SDF 由来の制約と、その回避策
> (per-axis プリミティブ) を開示した。AI が非等方スケールを期待して
> 行き詰まるのを防ぐ。

## 問101 — `smooth_difference` のオペランド順序が help で未記載 (hard difference との非対称)

**問**: hard `difference` は help で「(a minus b)」とオペランド順序を明示するが、
`smooth_difference` は記載が無い。AI は a-b か b-a か (どちらを削るか) を
類推に頼ることになる。挙動 (a から b を滑らかに削る) は eval/テスト
`smooth_operations_via_script` で既に担保されているが、help の開示が非対称。

**修正**: help の smooth_difference 行に「(a minus b, blended)」を追記し、
hard difference と開示を揃える。挙動は既存テスト (a の排他領域 -0.9 は残存・
b の領域 0.5 は削除) が回帰ガード済み。

変更ファイル: `src/mcp/tools.rs` (KADOSCENE_HELP smooth_difference 行)。

## 反映サマリ v61
| 問 | 実装 |
|----|------|
| 101 | smooth_difference のオペランド順序 (a minus b) を help で明示 |

> 総括: v61 は smooth_difference のオペランド順序開示を hard difference と
> 対称化した。問96-101 で全変形/ブーリアン操作の意味論が help に出揃った。

## 問102 — OVERHANG 方向性の検証を試み、コードの正しさを確認 + tools/list↔dispatch 整合の回帰ガード追加

**問 (当初仮説)**: OVERHANG 検出は build_dir と逆向き (下向き) の面のみを対象とし、
上向き面を除外すべき。既存テストは球で「build_dir が使われる」ことしか示さず、
`|n·bd|` 型の符号バグ (上向き面も誤検出) を検知できないのでは。

**調査結果 (コードは正しい)**: 検出ロジックは `worst` を 0 から開始し最小 `nd`
(= 最も下向き) のみを採る。上向き面 (`nd>0`) は決して選ばれず、方向性は正しい。
当初テスト (cone で +Z/-Z の worst 角を比較) は失敗したが、これは**閉形状は必ず
極値点を持ち、その面が軸方向 (±Z) を向くため worst ≈ 90° になる**という事実に
よる交絡で、コードのバグではない (球が両方向で 90° なのも同理由)。worst-angle は
方向性を分離できないため、当該テストは破棄。

**代わりに得た真の知見と対処**: 調査中、より確実に検証可能な構造的不変条件を発見:
**tools/list が広告する全ツールは call_tool でディスパッチ可能でなければならない**。
広告されているのに未配線だと AI は「unknown tool」という混乱するエラーを受け取る。
`tools_list_has_eight_tools` は個数しか見ておらず、この整合性は未検証だった。

検証テスト追加 (テスト 158→159):
- `every_advertised_tool_is_dispatchable`:
  tool_list() の全ツール名を call_tool で呼び、「unknown tool」を返さないことを確認
  (export は一時パス+後始末)。ツール追加時の配線忘れを検知する回帰ガード。

変更ファイル: `src/mcp/tools.rs` (新テスト), `src/verify/check.rs` (交絡テスト破棄)。

## 反映サマリ v62
| 問 | 実装 |
|----|------|
| 102 | OVERHANG 方向性のコード正当性を確認 + tools/list↔dispatch 整合の回帰ガード追加 |

> 総括: v62 は「ドキュメントでなく挙動を問う」視点で OVERHANG の方向性を検証し、
> コードが正しいこと (上向き面除外) と、worst-angle が方向性を分離できない数学的理由を
> 確認した。副産物として、より確実に守るべき構造不変条件 (広告ツール↔実装の整合) を
> 回帰テスト化した。誤った前提のテストは破棄し、検証可能な不変条件のみを固定する。

## 問103 — NEGATIVE_VOLUME が validate スキーマ/help の issue code リストから欠落

**問**: validate スキーマ説明は「All issue codes:」と謳いつつ 7 個しか列挙せず、
validator が実際に emit する `NEGATIVE_VOLUME` (裏返しメッシュ) が**漏れていた**。
help の issue-codes 節からも欠落。AI が NEGATIVE_VOLUME を受け取ってもどちらの
リファレンスにも無く意味を解釈できない (問79「issue code 完全性」が実は不完全だった)。
emit 側 (check.rs) と文書側 (tools.rs) の真実源が分離していたため気付かれなかった。

**修正**:
1. check.rs に `pub const ALL_ISSUE_CODES: &[&str]` (全 8 コードの単一の真実源) を追加。
2. validate スキーマ説明に NEGATIVE_VOLUME を追加 (「All issue codes」を真に完全化)。
3. help issue-codes 節に NEGATIVE_VOLUME 行を追加。
4. 文書ドリフトを検知する回帰テストを追加 (問102 と同じ「リスト↔文書」整合)。

検証テスト追加 (テスト 159→160):
- `issue_codes_are_fully_documented`:
  `ALL_ISSUE_CODES` の全コードが KADOSCENE_HELP と validate スキーマ説明の双方に
  含まれることを確認。将来コード追加時に文書更新漏れを CI が検知する。

変更ファイル: `src/verify/check.rs` (ALL_ISSUE_CODES const), `src/verify/mod.rs` (re-export),
`src/mcp/tools.rs` (スキーマ+help に NEGATIVE_VOLUME, 新テスト)。

## 反映サマリ v63
| 問 | 実装 |
|----|------|
| 103 | NEGATIVE_VOLUME を文書化 + 全 issue code の単一真実源 (ALL_ISSUE_CODES) と文書整合の回帰ガード |

> 総括: v63 は emit 側と文書側で分離していた issue code の真実源を一本化し、
> 漏れていた NEGATIVE_VOLUME を文書化した。問102 (ツール) に続き、問103 (issue code) でも
> 「実装が広告するものは全て文書化される」という整合性を回帰テストで保証する。

## 問104 — JSON 評価器とテキスト DSL の op 集合パリティが未検証

**問**: シーンは JSON ({"op":...}) とテキスト DSL (sphere(1)) の両記法で書け、同一の
SDF 木へ落ちる設計 (問59)。だが両パーサが**同じ op 集合を扱う保証 (テスト) が無い**。
片方に op を追加してもう片方を忘れると、AI が一方の記法で書いたときだけ
「unknown function/op」になる隠れた非対称が生じる。

**調査結果 (現状はパリティOK)**: eval.rs (JSON) と dsl.rs の op を全列挙して比較。
当初 grep で 8 op が DSL に欠落と出たが、これは `"union"|"intersection"|"difference"`
等の複数パターンアームを正規表現が拾えなかった誤検知で、実際は両者とも同一の 25 op を
扱う (バグ無し)。しかし回帰ガードが無かった。

**対処**: 全 25 op の DSL↔JSON 等価性を網羅する回帰テストを追加。各 op で
DSL 形と JSON 形が同一の場 (サンプル点で eval 一致) を生むことを `assert_same` で確認。
これは (a) 両パーサのカバレッジ・パリティ、(b) DSL→JSON→SDF パスの忠実性、の双方を固定する。

検証テスト追加 (テスト 160→161):
- `all_ops_parse_identically_in_dsl_and_json`:
  プリミティブ8・ブーリアン6・変形11 の全 25 op を DSL/JSON 両形で評価し場の一致を確認。

変更ファイル: `src/script/dsl.rs` (新テスト)。

## 反映サマリ v64
| 問 | 実装 |
|----|------|
| 104 | JSON 評価器↔テキスト DSL の全 25 op パリティ・忠実性を回帰テストで固定 |

> 総括: v64 は二つの入力記法 (JSON/DSL) が同一 op 集合を扱い同一結果を生むことを
> 全 op で保証した。問102 (ツール)・問103 (issue code) に続く3つ目の「リスト↔実装整合」
> ガードで、AI が記法を問わず一貫した結果を得られることを CI で担保する。

## 問105 — 再現性契約が MCP ツール経路で end-to-end に未検証

**問**: Kado の中核主張は決定性 (問5) / 内容 digest による再現性 (問61) で、問90-93 で
digest と解像度を全ツールが開示するようにした。だが「同一スクリプト・同一解像度なら
同一 digest」を**実際の AI 利用経路 (run_script→validate) で独立セッション間に渡って**
保証するテストが無かった。`polygonize_is_byte_deterministic` は抽出関数単体のみを見ており、
セッション状態・ツールディスパッチ・JSON 整形を含む end-to-end の決定性は未固定。

**対処**: MCP ツール経路の再現性を end-to-end に固定する回帰テストを追加:
- 同一スクリプトを**独立した2セッション**で run_script→validate(res=36) し、digest 一致を確認。
- 同一セッションで validate を2回呼んでも digest 不変 (validate が非破壊・冪等) を確認。

検証テスト追加 (テスト 161→162):
- `run_script_to_validate_digest_is_deterministic_across_sessions`:
  difference(smooth_union(sphere,cuboid),cylinder) を2セッションで通し digest 一致、
  かつ validate 反復で不変。

変更ファイル: `src/mcp/tools.rs` (新テスト)。

## 反映サマリ v65
| 問 | 実装 |
|----|------|
| 105 | 再現性契約 (同一スクリプト→同一 digest) を MCP 経路・独立セッション間で end-to-end 固定 |

> 総括: v65 は「決定的・再現可能」という製品中核の主張を、抽出関数単体でなく
> AI が実際に通る run_script→validate 経路で、セッションをまたいで検証した。
> 問90-93 の「digest+解像度の開示」と対になり、開示した再現条件が実際に成立することを保証する。

## 問106 — 不正な tools/call の応答形が不統一 (name 欠落だけ非標準 result)

**問**: MCP `tools/call` で **name 欠落**時、`handle_tools_call` は `rpc_error`
({code, message}) を返し、それが `handle()` の `success_response` に包まれて
**JSON-RPC success の result に {code, message}** が入る — `content`/`isError` を欠く
非標準応答。一方**未知ツール名**は `call_tool` 経由で正しく {content, isError:true} を返す。
同じ「不正な tools/call」が経路により異なる応答形を生み、result.content を期待する
MCP クライアントが name 欠落時だけ破綻しうる。`get` は非オブジェクトで None を返すため
パニックは無いが、応答形の不整合は実害。

**修正**: name 欠落/非文字列も未知ツールと同じ {content, isError:true} 形で返す
`tool_error_result` ヘルパを追加し、`rpc_error` (この1箇所でしか使われず) を削除。
全 tools/call 応答が {content, isError} 形に統一される。

検証テスト追加 (テスト 162→163):
- `malformed_tools_call_returns_uniform_error_shape`:
  name 欠落・name 非文字列・arguments 非オブジェクト・未知ツール の4ケースで
  パニックせず isError:true の統一形を返すことを確認。

変更ファイル: `src/mcp/server.rs` (tool_error_result 追加, rpc_error 削除, 新テスト)。

## 反映サマリ v66
| 問 | 実装 |
|----|------|
| 106 | 不正な tools/call の応答形を {content, isError:true} に統一 (name欠落の非標準resultを解消) |

> 総括: v66 は MCP プロトコル境界の堅牢性を高めた。不正リクエストでもパニックせず、
> 全 tools/call が一貫した {content, isError} 形で応答するため、クライアントが
> エラーを一様に扱える。

## 問107 — base64 エンコーダがテストベクタで未検証 (screenshot 画像配信の基盤)

**問**: screenshot ツールは PNG を base64 文字列で返す。base64 のパディング
(末尾 1〜2 バイト余りの `=`/`==`) が誤っていると、AI/クライアントが画像を
デコードできず screenshot 機能全体が壊れる。だが `base64_encode` に**テストが皆無**で、
末尾パディングや上位ビット (バイナリ安全性) の正しさが固定されていなかった。

**調査結果 (実装は正しい)**: chunk.len() に応じて 3/2/1 バイトを正しく処理し
`=` を補う。RFC 4648 準拠。バグは無いが回帰ガードが無い。

**対処**: RFC 4648 §10 の正準テストベクタ ("" / "f" / "fo" / "foo" / "foob" /
"fooba" / "foobar" → 全パディングケース) と、上位ビット (0xFF→"/w==", 0x00→"AA==")
のバイナリ安全性、出力長が常に 4 の倍数であることを確認する回帰テストを追加。

検証テスト追加 (テスト 163→164):
- `base64_matches_rfc4648_vectors_including_padding`:
  RFC 4648 全ベクタ + バイナリ高位バイト + 4 の倍数長を確認。

変更ファイル: `src/mcp/tools.rs` (新テスト)。

## 反映サマリ v67
| 問 | 実装 |
|----|------|
| 107 | base64 エンコーダを RFC 4648 ベクタ (全パディング+バイナリ) で回帰固定 |

> 総括: v67 は screenshot 画像配信の基盤である base64 を標準テストベクタで固定した。
> パディング・バイナリ安全性の退行を CI が検知し、AI が受け取る PNG が常にデコード可能で
> あることを保証する。

## 問108 — CRC-32 / Adler-32 チェックサムが既知値で未検証 (PNG/3MF 整合性の基盤)

**問**: PNG チャンクは CRC-32 (IEEE 802.3)、zlib トレーラは Adler-32、3MF (ZIP)
エントリも CRC-32 に依存する。これらが誤ると、厳格な PNG リーダ/スライサが
画像・3MF を**破損扱いで拒否**する (寛容なビューアでは通るため気付きにくい)。だが
3 つのチェックサム関数 (image.rs の crc32/adler32, zip.rs の crc32) は**既知値での
テストが皆無**で、構造・決定性テストしか無かった。

**調査結果 (実装は正しい)**: いずれも標準アルゴリズム (CRC poly 0xEDB88320,
init 0xFFFFFFFF, 最終 NOT / Adler mod 65521)。バグは無いが回帰ガードが無い。

**対処**: 業界標準チェック値で固定:
- CRC-32("123456789") = 0xCBF43926 (両実装), CRC-32("") = 0。
- Adler-32("123456789") = 0x091E01DE (s1=478,s2=2334), Adler-32("") = 1。
image.rs の crc32 は tag+data 連結のため分割位置に依らず同値になることも確認。

検証テスト追加 (テスト 164→167, +3):
- `crc32_matches_known_vectors` (image.rs, zip.rs), `adler32_matches_known_vectors` (image.rs)。

変更ファイル: `src/render/image.rs`, `src/io/zip.rs` (各テスト)。

## 反映サマリ v68
| 問 | 実装 |
|----|------|
| 108 | CRC-32 (PNG/3MF) と Adler-32 (zlib) を業界標準チェック値で回帰固定 |

> 総括: v68 は出力ファイル整合性の最下層 (チェックサム) を標準ベクタで固定した。
> 問107 (base64) と合わせ、screenshot/export が生成するバイナリの正しさが
> エンコード・チェックサム両面で CI 保証される。

## 問109 — 全出力経路が依存する基礎不変条件 (三角形インデックス健全性) が未検証

**問**: STL/GLB/3MF/HTML の全ライタ、レンダラ、体積計算、検証は例外なく
`mesh.vertices[t[i] as usize]` で頂点を参照する。インデックスが範囲外なら**全経路が
パニック**する。`from_soup` は (a) 範囲内インデックス、(b) 退化三角形 (重複インデックス)
の除去、を契約として保証するが、この基礎不変条件を固定するテストが無かった
(STL/GLB/3MF/HTML/PNG の各形式テストはあるが、その前提となる Mesh 不変条件は未検証)。

**調査結果 (実装は正しい)**: from_soup は頂点マップから索引を構築し範囲内を保証、
3 索引が相異なる三角形のみ push する。バグは無いが、全消費者が依存する不変条件の
回帰ガードが欠落。

**対処**: 代表形状群 (sphere/cuboid/穴あき/smooth_union/中空シェル) で
全三角形インデックスが範囲内かつ非退化であることを確認する不変条件テストを追加。

検証テスト追加 (テスト 167→168):
- `triangle_indices_are_in_bounds_and_nondegenerate`:
  5 形状で全 t[i] < vertices.len() かつ t[0]≠t[1]≠t[2]≠t[0] を確認。

変更ファイル: `src/extract/mesh.rs` (新テスト)。

## 反映サマリ v69
| 問 | 実装 |
|----|------|
| 109 | 全出力経路が依存する Mesh の三角形インデックス健全性 (範囲内・非退化) を回帰固定 |

> 総括: v69 は個別出力形式 (問107/108 で固めた) の更に下層、全形式が共有する
> インデックス付きメッシュの基礎不変条件を固定した。from_soup の契約が崩れれば
> 全ライタが同時に壊れるため、最も影響範囲の広い前提を CI で保証する。

## 問110 — 回転の合成 (入れ子) の正しさが未検証 (向き付き組立の前提)

**問**: AI が向き付きの組立を作るとき、回転をネスト (rotate_x(rotate_y(...))) する。
だが既存テストは単一回転 (問51) のみで、**合成の正しさ**が未検証。合成の代数法則が
壊れると、AI が意図した姿勢と実際の姿勢がずれる。

**対処 (実装は正しい)**: 回転合成の代数不変条件を固定する回帰テストを追加:
1. **同軸加法性**: rotate_z(0.3)∘rotate_z(0.5) == rotate_z(0.8)。
2. **往復恒等**: rotate_z(θ)∘rotate_z(-θ) == 恒等。
3. **異軸の順序依存 (非可換)**: rot_x∘rot_y ≠ rot_y∘rot_x — AI が順序を入れ替えると
   結果が変わるという重要な性質を明示的に固定。

検証テスト追加 (テスト 168→169):
- `rotation_composition_is_additive_roundtrip_and_order_dependent`:
  cuboid に対し格子点で 3 法則を確認。

変更ファイル: `src/core/sdf.rs` (新テスト)。

## 反映サマリ v70
| 問 | 実装 |
|----|------|
| 110 | 回転合成の代数法則 (同軸加法・往復恒等・異軸非可換) を回帰固定 |

> 総括: v70 は単一変換でなく変換の合成挙動を検証した。特に「異軸回転は順序依存」を
> 明示テスト化し、AI が組立の姿勢を組むとき順序を誤ると結果が変わることを保証する。

## 問111 — デフォルトシーンの構造健全性 (AI の第一印象) が未検証

**問**: AI が接続直後 (run_script 前) に validate/screenshot/export を呼ぶと
**デフォルトシーン** (smooth_union(sphere(1), cuboid(0.8), 0.2)) が対象になる。
「デフォルトは健全なデモ」という前提は暗黙で、将来 `default_scene` を変更した際に
構造的に壊れたデモ (非多様体・複数ボディ・裏返し等) を出荷しても気付けない。
AI の第一印象とすべての run_script 前操作がこのシーンに依存する。

**対処 (現状は健全)**: デフォルトシーンの構造健全性を固定する回帰テストを追加。
閾値依存の OVERHANG/THIN_WALL (閉形状の下面は常に overhang なので) は対象外とし、
manifold=true・正体積・構造エラー (OPEN_MESH/NON_MANIFOLD/EMPTY_MESH/
NEGATIVE_VOLUME/MULTIPLE_BODIES) が無いことのみを保証する。

検証テスト追加 (テスト 169→170):
- `default_scene_is_structurally_sound_for_first_impression`:
  fresh Session で validate(min_wall=0,max_overhang=0) し manifold/正体積/構造エラー無しを確認。

変更ファイル: `src/mcp/tools.rs` (新テスト)。

## 反映サマリ v71
| 問 | 実装 |
|----|------|
| 111 | デフォルトシーンの構造健全性 (manifold/単一ボディ/正体積) を回帰固定 |

> 総括: v71 は「AI が最初に出会う形状」の健全性を固定した。run_script 前の全操作と
> 第一印象がデフォルトシーンに依存するため、将来のデフォルト変更が壊れたデモを
> 出荷しないことを CI で保証する。
> テスト数 141→142 + 統合 3。

## 問112 — 平行移動合成の代数法則 (加法性・往復恒等) が未検証

**問**: 回転合成 (問110) に対称な代数法則として、translate の合成挙動が未検証。
AI が `translate(v1, translate(v2, S))` を生成した場合、それが `translate(v1+v2, S)` と
等価になることは、実装が `S(p - offset)` の連鎖として正しく動くことへの依存。
往復恒等 `translate(-v, translate(v, S)) = S` も未テスト。

**調査結果 (実装は正しい)**: Translate(c, offset) の eval は `c.eval(p - *offset)` の
入れ子で正確に加算される。バグは無いが、代数法則として回帰固定が必要。

**対処**: translate 合成の 2 法則を回帰テストとして固定:
1. **加法性**: translate(v1)∘translate(v2) == translate(v1+v2) — 格子点全体で数値完全一致。
2. **往復恒等**: translate(v)∘translate(-v) == identity — 格子点全体で子と同値。

検証テスト追加 (テスト 170→171):
- `translate_composition_is_additive_and_roundtrip`: 非同一 v1/v2 ベクトルで両法則を格子点確認。

変更ファイル: `src/core/sdf.rs` (新テスト)。

## 問113 — ブーリアン等冪法則 (自己演算) が未検証

**問**: AI が操作を二重適用したり同一形状を重複参照した場合、
`union(A, A)` / `intersection(A, A)` / `difference(A, A)` の結果は何か。
既存テストは union/intersection/difference の「異形状」ペアのみで、
「同一形状を両引数に与えた場合」の代数法則が未検証。

**調査結果 (実装は代数的に正しい)**:
- union(A, A): min(f, f) = f (等冪) → A そのまま
- intersection(A, A): max(f, f) = f (等冪) → A そのまま
- difference(A, A): max(f, -f) = |f| ≥ 0 (自己差分は外部のみ = 空集合)

**対処**: 上記 3 法則を格子点全体で固定する回帰テストを追加。

検証テスト追加 (テスト 171→172):
- `boolean_idempotency_union_intersection_difference_self`:
  smooth_union(sphere, cuboid, 0.2) を A として 3 法則を格子点確認。

変更ファイル: `src/core/sdf.rs` (新テスト)。

## 問114 — issue severity JSON 値が小文字であることの全電池回帰が欠如

**問**: validate の to_json() が severity を "error"/"warning"/"info" (小文字) で
シリアライズすることは問82 で一度確認したが、その確認は1形状・1 issue のみだった。
将来 Severity enum に新バリアントを追加した場合、match アームに大文字変換
(例: `Severity::Critical => "Critical"`) が混入してもテストが通ってしまう。
全コードが実際に誘発される電池 (EMPTY/OPEN/THIN_WALL/SUSPICIOUS_SCALE 等) で
小文字であることを固定できていない。

**調査結果 (実装は正しい)**: check.rs の serialization は全バリアントで小文字固定。
バグは無いが、多様な issue コードを誘発する電池での回帰ガードが欠落。

**対処**: 5 形状×パラメータの電池で誘発された全 issue の severity が
`VALID_VALUES = ["error","warning","info"]` のいずれかであり、かつ `sev == sev.to_lowercase()`
を確認する回帰テストを追加。

検証テスト追加 (テスト 172→173):
- `issue_severity_serializes_as_lowercase_valid_value`:
  empty/open/thinwall/tiny/ok の 5 電池で全 issue severity が小文字正当値であることを確認。

変更ファイル: `src/verify/check.rs` (新テスト)。

## 問115 — sampling_box が AABB を内包する invariant の全形状電池が欠如

**問**: sampling_box は aabb の 5% マージン外包を保証するが、既存の確認は
`aabb_encloses_surface_samples` (問14) の1つの複合ツリーのみ。
`sampling_box_is_never_inverted` (問40) は反転防止のみ。primitive/変換/ブーリアン/複合/
repeat を含む代表電池での `slo <= alo` かつ `shi >= ahi` が未検証。
将来の AABB 実装変更が特定の形状クラスで包含を壊しても気づけない。

**調査結果 (実装は正しい)**: sampling_box は aabb に 5% マージンを加えた後
max(lo,hi) 正規化する。バグは無いが代表電池での invariant 固定が欠落。

**対処**: 10 形状電池 (sphere/cuboid/cylinder/capsule/torus/translated/union/difference/
shell/repeat_n) で sampling_box lo <= aabb lo, sampling_box hi >= aabb hi,
sampling_box 非反転の 3 条件を固定する回帰テストを追加。

検証テスト追加 (テスト 173→174):
- `sampling_box_encloses_aabb_for_representative_shapes`: 10 形状で 3 invariant を確認。

変更ファイル: `src/core/sdf.rs` (新テスト)。

## 反映サマリ v72
| 問 | 実装 |
|----|------|
| 112 | translate 合成の加法性・往復恒等を格子点全体で回帰固定 |
| 113 | union/intersection/difference の自己演算等冪法則を回帰固定 |
| 114 | issue severity JSON 値が全電池で小文字かつ正当な値のみであることを回帰固定 |
| 115 | sampling_box ⊇ AABB の invariant を 10 形状電池で回帰固定 |

> 総括: v72 は変換代数 (問112)・ブーリアン代数 (問113)・シリアライズ規約 (問114)・
> 幾何インフラ (問115) の 4 つの独立した前提をそれぞれ代表電池で固定した。
> 問110 (回転合成) と対称に translate 合成を、問82 (severity 小文字) を電池で強化し、
> 問14 (AABB 包含) を全形状に拡張した。
> テスト数 170→174 + 統合 3。

## 問116 — 抽出ホットパスの不要なヒープ確保 (長所短所の洗い出し: 性能)

**長所短所の俯瞰**:
- **長所**: カーネルは std のみ・決定的・全経路がインデックスメッシュ不変条件に依存し
  テストで強く守られている。MT 抽出は曖昧ケースが無く水密性が構造的に保証される。
- **短所**: `emit_tet` は四面体ごと (= res³ セル × 6) に `Vec<usize>` を最大 2 個
  ヒープ確保していた。res=48 で約 4M 四面体 → 数百万回の小確保。決定性・正しさには
  影響しないが、抽出は最頻の重い処理でありアロケータ負荷が無駄。

**問**: この `Vec` は本当に必要か。四面体の角は常に 4 個で、内/外の振り分けは固定長
配列で十分。`filter().collect()` の昇順を保てば出力はバイト不変のはず。

**対処**: 内/外インデックス収集を `[usize; 4]` + カウンタのスタック配列に置換。
収集順 (0..4 昇順) は元の実装と完全一致するため、edge_vertex の評価順・巻き順が不変。

**検証 (バイト不変の証明)**: リファクタ前に sphere(res16)・穴あき(res20) のダイジェストを
測定し、リファクタ後も同一であることを golden テストで固定。決定性契約 (問5) を
観測可能な形で守る。

検証テスト追加 (テスト 174→175):
- `extraction_digest_is_byte_stable_golden`:
  sphere(res16) digest=0x13a377110ebca030 (頂点1586/三角3168) と
  穴あき(res20) digest=0xabb6848b19e4319a (頂点3036/三角6072) を golden 固定。

変更ファイル: `src/extract/marching_tetrahedra.rs` (emit_tet リファクタ + golden テスト)。

## 反映サマリ v73
| 問 | 実装 |
|----|------|
| 116 | 抽出ホットパスの Vec ヒープ確保を固定長スタック配列に置換 (バイト不変を golden 固定) |

> 総括: v73 は初の純粋性能改善。決定性契約があるため「最適化が出力を変えない」ことを
> golden ダイジェストで観測可能に固定し、安心して内部実装を変えられる土台を作った。
> テスト数 174→175 + 統合 3。

## 問117 — JSON シリアライザが非有限値で不正な JSON を吐く (入出力の非対称)

**長所短所の俯瞰**:
- **長所**: パーサは問20 で非有限値 (1e400→+inf 等) を入力段階で遮断しており、
  不正値が SDF へ伝播するのを防いでいる。
- **短所 (非対称)**: その防御は**入力側のみ**。出力側 (シリアライザ) は無防備で、
  `Value::Number(NaN/±Inf)` を Display すると `NaN`/`inf`/`-inf` を吐く。これは
  **不正な JSON** であり、MCP 応答を受け取る AI クライアントのパーサ
  (本パーサ自身も問20 で非有限を拒否する) を壊す。内部計算 (体積・寸法・角度・
  厚み等) が万一 NaN/Inf を生んで `json::n(...)` 経由で応答に混入すると、
  応答全体が parse 不能になる。

**問**: 入力を守って出力を守らないのは一貫性を欠く。シリアライザは、いかなる
内部状態でも valid JSON を吐く契約を持つべきではないか。

**対処**: シリアライザの `Value::Number` 分岐に非有限ガードを追加。serde_json と
同じく非有限を `null` に落とし、応答が常に valid JSON であることを保証する
(決定的で安全側の縮退)。問20 (入力遮断) と対称な出力側防御。

検証テスト追加 (テスト 175→176):
- `nonfinite_numbers_serialize_as_valid_json_null`:
  NaN/±Inf 単体が "null" になり再パース可能なこと、非有限混入オブジェクトでも
  有限値は保持・非有限は null になり全体が valid JSON のままであることを確認。

変更ファイル: `src/mcp/json.rs` (シリアライザ修正 + 回帰テスト)。

## 反映サマリ v74
| 問 | 実装 |
|----|------|
| 117 | JSON シリアライザの非有限値を null に落とし、応答が常に valid JSON である契約を保証 |

> 総括: v74 はプロトコル境界の堅牢性バグを修正。問20 (入力で非有限拒否) と対称に
> 出力側も守り、内部計算が万一非有限を生んでも MCP 応答が壊れない安全側縮退を入れた。
> テスト数 175→176 + 統合 3。

## 問118 — MCP トランスポート層に Content-Length 上限が無い (確保前 DoS)

**長所短所の俯瞰**:
- **長所**: DSL 評価器は問16 で多層のリソース上限 (ソース 1 MiB・ノード 5 万・深さ 64・
  repeat 256) を持ち、悪意ある入力を計算段階で遮断する。JSON パーサも深さ上限を持つ。
- **短所 (層の隙間)**: それらの上限は**本文を全部メモリに読んだ後**にしか効かない。
  `read_message` は信頼できないクライアントの `Content-Length` を無検査で
  `vec![0u8; len]` に渡す。`Content-Length: 999999999999` 一発でテラバイト級の確保を
  試み OOM クラッシュ (= DoS) を起こせる。確保はパース前なので問16 では防げない。

**問**: 計算層を何重にも守っているのに、その手前のトランスポート層 (バイト確保) が
無防備なのは防御の一貫性を欠く。確保の**前**に上限を課すべきではないか。

**対処**: `MAX_MESSAGE_BYTES = 16 MiB` を新設し、`vec![0u8; len]` 確保の前に
`Content-Length` を検査。超過は `InvalidData` で即拒否。本文は JSON-RPC エンベロープ +
エスケープでスクリプト生バイト (問16 の 1 MiB) より膨らむため、十分な余裕を見て 16 MiB。

検証テスト追加 (テスト 176→177):
- `oversized_content_length_is_rejected_before_allocation`:
  上限+1 は確保前に "exceeds limit" で拒否、上限ちょうどは size gate を通過 (本文不足の
  EOF になる)、通常フレームは正常パースされる (正当経路の回帰) ことを確認。

変更ファイル: `src/mcp/server.rs` (上限定数 + 確保前チェック + 回帰テスト)。

## 反映サマリ v75
| 問 | 実装 |
|----|------|
| 118 | MCP トランスポート層に Content-Length 上限を新設し、確保前 OOM/DoS を遮断 |

> 総括: v75 は防御の層の隙間を埋めた。問16 (計算層) は本文読込後にしか効かないため、
> その手前のバイト確保段階に独立した上限を置き、巨大ヘッダによる OOM を確保前に止める。
> 入力 (問20)・計算 (問16)・トランスポート (問118) の 3 層で多重防御が揃った。
> テスト数 176→177 + 統合 3。

## 問119 — Vec3/math の最下層演算が無テスト (全SDF評価の基盤が未固定)

**長所短所の俯瞰**:
- **長所**: SDF 評価・座標変換・法線計算など全アルゴリズムが `Vec3` 上に構築されており、
  決定性 (問5) のために FMA を意図的に回避した固定演算順序で記述されている。
- **短所 (発見)**: `src/core/math.rs` は **テストが1件も無い**。`dot`・`cross`・`length`・
  `mix`・`clamp` が直接テストされていない。上位の SDF テスト群が間接的に行使するが、
  `cross` の巻き順変更 (右手系→左手系) や `mix` 式の反転などの破壊的変更が、上位の
  比較テストを「両側が同じように壊れる」ため検出できない可能性がある。

**問**: 全SDF評価・ラスタライズ・メッシュ抽出が依存する最下層が無テストは
「基礎の割れ目」ではないか。直接テストで固定すべきでは。

**対処**: math.rs に 17 の直接単体テストを追加し、全演算の仕様を値レベルで固定:
- dot の可換性・直交性ゼロ・自己内積 = 長さ²  
- cross の右手則 (X×Y=+Z)・反可換性・自己外積=ゼロ・平行四辺形面積
- add/sub/mul/div/neg の成分別挙動
- abs/min/max/max_scalar/max_component の境界条件
- mix (t=0/0.5/1/外挿) と clamp (以下/以上/ちょうど)

検証テスト追加 (テスト 177→194):
- 17 テスト in `core::math::tests`。

変更ファイル: `src/core/math.rs` (直接単体テスト)。

## 問120 — ZIP ローカルヘッダと中央ディレクトリの CRC が別ループで独立計算される

**長所短所の俯瞰**:
- **長所**: ZIP の CRC-32 は問108 で標準ベクタにより正しさが固定されている。
- **短所 (発見)**: `build_zip` は各エントリの `crc32(data)` を 2 回呼んでいた。
  第1パス (ローカルヘッダ) と第2パス (中央ディレクトリ) で独立計算するため、
  (a) 大きな 3MF では無駄な倍計算、(b) 将来の変更でどちらか一方だけを修正すると
  ローカル/中央の CRC 不一致が生じ ZIP が壊れる (スライサが展開を拒否)。

**問**: 一致を「偶然」ではなく「構造的に」保証できないか。

**対処**: 第0パスで CRC を一括計算してキャッシュし、両ループで共有。両ヘッダが
同一の計算結果を参照することで一致を構造的に保証する。

検証テスト追加 (テスト 194→195):
- `local_header_and_central_directory_crc_are_identical`:
  2エントリの ZIP を構築し、実際のバイト列でローカルヘッダ CRC = 中央ディレクトリ CRC
  かつ独立計算と一致することを確認。

変更ファイル: `src/io/zip.rs` (CRC キャッシュ + 構造的一致テスト)。

## 反映サマリ v76
| 問 | 実装 |
|----|------|
| 119 | math.rs の全 Vec3/スカラ演算を直接単体テストで固定 (17 テスト) |
| 120 | ZIP ローカルヘッダと中央ディレクトリの CRC を CRC キャッシュで構造的に一致保証 |

> 総括: v76 は「最下層の無テスト」と「二重計算の不一致リスク」という 2 つの構造的弱点を
> 解消した。math.rs は全 SDF 評価の基盤でありながら 0 テストだったため、17 の直接テストで
> 全演算を値レベルで固定した。ZIP の CRC キャッシュは一致を偶然ではなく構造的に保証する。
> テスト数 177→195 + 統合 3。

## 問121 — 失敗した run_script が undo 状態を破壊しない不変条件が未固定

**長所短所の俯瞰**:
- **長所**: `run_script` は eval 失敗時に prev_scene 保存の**前**に早期 return するため、
  失敗したスクリプトは現在シーンも undo 履歴も変更しない (正しい実装)。undo は
  single-level で問67/問74 のテストが基本動作を固定している。
- **短所 (発見)**: その「失敗が undo 状態を壊さない」という重要な不変条件を固定する
  テストが無い。もし将来 prev_scene の保存を eval 成功の**前**に動かす (もっともらしい
  リファクタ事故)、失敗した run_script が undo 履歴を現在シーンで上書きし、直前の
  成功した変更を取り消せなくなる — しかも既存テストはすべて通る (失敗経路を試さないため)。

**問**: 「失敗は無害」という契約は実装の行順に暗黙依存している。明示テストで固定すべきでは。

**対処 (現状は正しい)**: 失敗経路を通る回帰テストを追加:
1. run_script(A=sphere r=3) 成功 → scene=A, undo履歴=default
2. run_script(invalid op) 失敗 → scene は A のまま, undo履歴も default のまま
3. undo → A ではなく default (A の前) に戻る

ステップ2で undo 履歴が壊れていれば、ステップ3で A のままになりアサートが落ちる。

検証テスト追加 (テスト 195→196):
- `failed_run_script_preserves_undo_state_and_scene`:
  失敗した run_script の後もシーン不変・undo が A の前に正しく戻ることを確認。

変更ファイル: `src/mcp/server.rs` (失敗経路の回帰テスト)。

## 反映サマリ v77
| 問 | 実装 |
|----|------|
| 121 | 失敗した run_script が現在シーン・undo 履歴を破壊しない不変条件を回帰固定 |

> 総括: v77 はステートフルな undo ロジックの暗黙の契約 (「失敗は無害」) を明示テスト化した。
> 実装は正しいが、その正しさが行順 (eval 成功後に prev 保存) に依存しているため、
> 失敗経路を通るテストでリグレッションを防ぐ。
> テスト数 195→196 + 統合 3。

## 問122 — 公開入口 eval_any の自動判別 (JSON/DSL 振り分け) が未テスト

**長所短所の俯瞰**:
- **長所**: DSL↔JSON のパリティ (両表現が同じ場へ落ちること) は dsl.rs で
  `eval_dsl` vs `eval_scene` を直接比較する形で厚くテストされている (問59)。
- **短所 (発見)**: しかし MCP `run_script`・CLI が実際に呼ぶのは `eval_any` で、これは
  「先頭非空白が `{` なら JSON、さもなくば DSL」という**振り分け**を行う。この振り分け
  ロジック自体が無テスト。もし条件が反転 (例: `!starts_with('{')`) すれば、すべての
  `run_script` が誤経路に流れ全 JSON 入力が壊れるが、既存テストは下位関数を直接呼ぶため
  検出できない。

**問**: 公開入口の振り分けは最も多用される経路。その分岐を公開 API レベルで固定すべきでは。

**対処**: `eval_any` の 4 つの不変条件を固定する直接テストを追加:
1. 同一シーンの JSON 版と DSL 版が eval_any 経由で同じ場になる (sphere・difference)。
2. JSON 前の空白・改行があっても JSON と判別される (trim_start の検証)。
3. 識別子始まりは DSL 経路へ正しく流れる。
4. どちらの経路でも不正入力・空文字列は Err を返しパニックしない。

検証テスト追加 (テスト 196→200):
- `dispatches_json_and_dsl_to_same_field`, `leading_whitespace_before_brace_is_still_json`,
  `identifier_start_routes_to_dsl`, `malformed_input_returns_error_not_panic`。

変更ファイル: `src/script/mod.rs` (公開入口の振り分けテスト)。

## 反映サマリ v78
| 問 | 実装 |
|----|------|
| 122 | 公開入口 eval_any の JSON/DSL 自動判別を 4 不変条件で回帰固定 |

> 総括: v78 は最も多用される公開入口の振り分けロジックを固定した。下位のパリティは
> テスト済みだったが、実際の呼び出し経路 (eval_any の `{` 判別) は無テストだった。
> 振り分けの反転・trim 忘れ・パニックを公開 API レベルで防ぐ。
> テスト数 196→200 + 統合 3。

## 問123 — レンダラの投影行列の数値的正しさが未検証

**問**: screenshot の既存テストは「非ブランク」「決定的」のみを確認する。だが転置ミス・
符号反転・基底の歪みがあっても、**安定した誤画像**は非ブランクかつ決定的になる。
「画素が出て安定していればカメラは正しい」という暗黙の仮定は、誤りを見逃さないか。
screenshot の KPI は「AI が向き・形状を視認する」(問66 グノモン) ことなので、投影の
幾何学的正しさは出力の意味に直結する。

**調査結果 (実装は正しい)**: look_at は target を視線軸へ、perspective は軸を NDC 原点へ
写す。数学的に target → スクリーン中央、+up 方向 → 中央より上 になることを確認。

**対処**: パイプライン全体 (view×proj×透視除算×ndc_to_screen) の数学的核心を固定する
回帰テストを追加:
1. カメラ注視点はスクリーン中央へ投影される (clip w>0 で前方も確認)。
2. target の真上 (+up) の点は中央より上 (スクリーン y が小さい) へ投影される (y 反転検証)。
3. look_at の上 3×3 は正規直交基底 (各行 unit・相互直交) — 剛体変換の保証。

検証テスト追加 (テスト 200→203):
- `camera_target_projects_to_screen_center`, `point_above_target_projects_above_center`,
  `look_at_basis_is_orthonormal`。

変更ファイル: `src/render/raster.rs` (投影数値の回帰テスト)。

## 問124 — 拡張子→形式の振り分けが CLI と MCP で二重実装 (発散リスク)

**問**: export の「.glb→GLB / .3mf→3MF / .html→HTML / 他→STL」という拡張子判定が、
`cli/main.rs` と `tools::tool_export` で**独立に if-else 実装**されている。形式を追加・
変更するとき片方だけ直すと両入口がサイレントに食い違う。これは問120 (ZIP CRC の二重計算)
と同じ「同じ判断を複数箇所で持つと将来 diverge する」構造的弱点ではないか。どちらが正本か。

**対処**: `io::ExportFormat` enum に判定・ラベル・書き出しを一元化し、CLI と MCP の両方が
これを呼ぶようにした。`from_path` (拡張子判定)・`label` (応答ラベル)・`write` (ライタ振り分け)
を単一の真実源とし、発散を構造的に不可能にする。両入口の if-else を削除。

検証テスト追加 (テスト 203→207):
- `from_path_dispatches_by_extension` (全拡張子+未知フォールバック),
  `from_path_is_case_insensitive` (.GLB 等大文字),
  `label_matches_format`, `write_produces_format_specific_bytes` (各形式の固有マジック確認)。

変更ファイル: `src/io/mod.rs` (ExportFormat 新設+テスト), `src/mcp/tools.rs`,
`src/cli/main.rs` (両入口を一元化、重複 if-else 削除)。

## 反映サマリ v79
| 問 | 実装 |
|----|------|
| 123 | レンダラ投影パイプラインの数学的核心 (target→中央・up→上・正規直交基底) を回帰固定 |
| 124 | 拡張子→形式の振り分けを io::ExportFormat に一元化し CLI/MCP の発散を構造的に防止 |

> 総括: v79 は「安定した誤り」(問123: 非ブランクだが投影が誤った画像) と「重複による発散」
> (問124: CLI と MCP の独立した拡張子判定) という 2 つの異なる弱点を解消した。問123 は
> 振る舞いテストが見逃す数値的正しさを、問124 は問120 と同じ重複削減の哲学を適用した。
> テスト数 200→207 + 統合 3。

## 問125 — 品質を掲げるのに lint 未強制で警告が無音蓄積、CI も無い

**問**: Kado は「依存ゼロ・決定的・セキュリティ重視」を掲げるが、`cargo clippy` を
かけると 12 件 (lib 8 + テスト 4) の警告が出る。CI も存在しない。「高品質」を標榜しながら
lint を機械的に強制しないのは、主張と実態の乖離ではないか。警告は無音で蓄積し続ける。
そして最も重要な問い: これらを修正して**バイト単位の決定性契約**を壊さないか。

**調査結果 (全 12 件は出力不変で修正可能)**: 全て読みやすさ/慣用句の問題で、計算値には
触れない。MSRV 1.94 は `is_multiple_of`(1.87)・`div_ceil`(1.73) を支持。
- lib: index ループ→enumerate (問116 抽出コード)・`%4!=0`→`is_multiple_of`・
  `loop+match`→`while let`・base64 容量 `(n+2)/3`→`div_ceil`・冗長 `as u32` キャスト×2・
  doc リスト整形・恒等 map 削除。
- test: `c != &cam.bg`→`c != cam.bg` ×3・複雑型に `type MeshFactory` 別名。

**対処**: 全 12 件を解消。抽出コード変更後も `extraction_digest_is_byte_stable_golden`
(問116) が通り、出力がバイト不変であることを証明。

検証: `cargo clippy --all-targets -- -D warnings` が exit 0、全 207 テスト緑、
golden ダイジェスト不変。

**CI による構造的強制 (保留)**: 本来は `.github/workflows/ci.yml` を新設し
`cargo test` と `cargo clippy --all-targets -- -D warnings` を CI で強制したい
(問120/124 の「逸脱を構造的に不可能にする」哲学)。だが現在の GitHub App には
`workflows` 権限が無く、ワークフローファイルを push できない。BACKLOG とし、
権限付与後に追加する。それまでは push 前のローカル `cargo clippy --all-targets -- -D warnings`
を運用ルールとする。(fmt 未整形も別 BACKLOG)

変更ファイル: `src/extract/marching_tetrahedra.rs`, `src/io/gltf.rs`, `src/mcp/server.rs`,
`src/mcp/tools.rs`, `src/render/image.rs`, `src/render/raster.rs`, `src/script/mod.rs`,
`src/verify/check.rs` (lint 修正)。

## 反映サマリ v80
| 問 | 実装 |
|----|------|
| 125 | clippy 警告 12 件を出力不変で解消 (CI 強制は workflows 権限不足で BACKLOG) |

> 総括: v80 は「主張と実態の乖離」(品質を掲げるが lint 未強制) の半分を埋めた。警告 12 件を
> 解消し、抽出コードの変更は golden ダイジェストでバイト不変を証明。CI による再発防止は
> App の workflows 権限不足で push できず BACKLOG とした。
> テスト数 207 + 統合 3 (新規テストなし; lint 修正)。

---

## 問126 — sampling_box の 1e-3 最小余白が未テスト (ゼロ AABB 形状)

**問**: `sampling_box` は `max(0.05*diag, 1e-3)` の余白を加える。AABB が点
(例: 半径 0 の球) の場合 `diag = 0` となり `m = 1e-3` が唯一の防護線となる。
この最小値が誤って削除されると `polygonize` がゼロ幅ボックスを受け取り
ステップ幅 = 0 / ゼロ除算が起きる。だが「1e-3 最小余白が点 AABB に適用される」
という不変条件を固定するテストが存在しなかった。

**仮定の見直し**: 「sampling_box の非反転テストがあれば十分」は誤り。
ゼロ幅ボックスは反転しないため反転テストを通過してしまう。

**実装 (sdf.rs)**:
- `sampling_box_applies_minimum_margin_for_zero_aabb`: `Sdf::Sphere { radius: 0.0 }` で
  各軸の幅 ≥ 1e-3 を確認し、`polygonize` がパニックなく空メッシュを返すことを検証。

## 問127 — Union-Find が N > 2 の独立成分で正しいことが未確認

**問**: `body_components` の既存テストは 1 ボディ・2 ボディ・1 ボディ+1 空洞の
3 ケースのみ。Union-Find (経路分割 + 小 root 優先) が N = 3 以上の成分で
正しく動作するかどうかを固定するテストがなかった。

**仮定の見直し**: 「アルゴリズムが正しいから 2 成分テストで十分」は主張であり証拠ではない。
N > 2 では Union-Find の経路圧縮がより深い木で動作し、成分ごとの符号付き体積集計も
3 エントリを持つ HashMap を正確に処理しなければならない。

**実装 (mesh.rs)**:
- `three_disjoint_solids_are_three_bodies`: 3 つの離れた球の和を res=32 で抽出し
  `body_components() == (3, 0)` を確認。is_edge_manifold も同時に保証。

## 問128 — 3MF XML 書き出しが非有限座標で "NaN"/"inf" を出力する

**問**: `build_model_xml` は `v.x, v.y, v.z` を Rust の `{}` フォーマットで
直接 XML に埋め込む。`f64::NAN` や `f64::INFINITY` は "NaN"/"inf" と書き出され、
XML テキストとして無効な浮動小数点表現になる。DSL/JSON の `eval.rs` がスケール因子
などを検証するため通常の SDF 抽出では非有限座標は生じないが、Sdf 構造体を直接
構築した場合 (パブリック API) にはこの保証がない。

**仮定の見直し**: 「出力層は入力が正常なら正常」は入力バリデーションが完全な場合のみ
成り立つ。公開 API は直接構築を許す以上、出力層にも防御が必要。
(JSON 数値の NaN→null 変換 (問117) と同じパターン。)

**実装 (threemf.rs)**:
- `finite_coord(v: f64) -> f64`: 非有限値を 0.0 に正規化するプライベートヘルパを追加。
- `build_model_xml` の `writeln!` を `finite_coord` 経由に変更。
- `model_xml_never_contains_nonfinite_number_strings`: 通常メッシュと手動非有限
  頂点メッシュの両方で "NaN"/"inf" が出力されないこと、かつ非有限座標が 0 に
  正規化されることを確認。

## 問130 — 空メッシュが「体積信頼可」と誤判定されないことの明示テストが欠如

**問**: `volume_reliable() = is_manifold && triangle_count > 0` の
`triangle_count > 0` ガードは空メッシュを除外するための唯一の防護線。
空メッシュは辺がないため `is_manifold = true` になるが、符号付き体積は
ガウス発散定理を閉曲面に適用するため非空メッシュでのみ意味を持つ。
このガードが削除されると AI/利用者が空メッシュの体積 0.0 を信頼してしまう。
だが「空メッシュで volume_reliable = false」を明示するテストが存在しなかった。

**仮定の見直し**: 「is_manifold && tri_count > 0 の impl を読めば分かる」は
テストではない。実装が変わったとき (例: 条件を `is_manifold` 単体に簡略化)
サイレントに壊れる。

**実装 (check.rs)**:
- `empty_mesh_volume_is_never_reliable`: 非重複 SmoothIntersection (問40) で
  空メッシュを生成し `volume_reliable() == false` と `EMPTY_MESH` issue の
  両方を確認。

## 反映サマリ v81
| 問 | 実装 |
|----|------|
| 126 | `sampling_box` ゼロ AABB 形状の 1e-3 最小余白不変条件テスト (sdf.rs) |
| 127 | `body_components` N=3 独立成分テスト — Union-Find 多成分正確性 (mesh.rs) |
| 128 | 3MF 非有限座標を `finite_coord` で 0 正規化 + テスト (threemf.rs) |
| 130 | 空メッシュ `volume_reliable=false` 明示テスト (check.rs) |

> 総括: v81 は「未テストの防護線」4 件を固定した。
> sampling_box の最小余白ガード (問126)、Union-Find の多成分正確性 (問127)、
> 3MF の非有限座標サニタイズ (問128)、空メッシュの体積信頼判定 (問130)。
> clippy --all-targets -D warnings = 0 warnings。
> テスト数 211 ユニット + 統合 3 = 214 合計。

---

## 問131 — arg_samples の `dim * samples ≤ MAX_IMAGE_DIM` 不変条件が未テスト

**問**: `arg_samples` はコメントで `dim * samples ≤ MAX_IMAGE_DIM` を保証すると謳うが、
この不変条件を検証するテストがなかった。隣の `arg_dim` は `image_dims_are_clamped`
テストで上下限を固定されているのに対し、`arg_samples` の cap 計算
`(MAX_IMAGE_DIM / width.max(1)).max(1)` は一切テストされていなかった。
`render(…, width*samples, height*samples)` が `MAX_IMAGE_DIM^2 ≈ 16M px ≈ 48 MiB` を
超えるバッファを確保しないことを保証するのはこの計算のみ。

**仮定の見直し**: 「arg_dim がテストされているから arg_samples も安全」は誤り。
二つの引数解析器は独立した関数であり、一方のテストが他方の正しさを証明しない。

**実装 (tools.rs)**:
- `arg_samples_invariant_dim_times_samples_fits_max_image_dim`:
  デフォルト・各軸最大寸法・両軸最大寸法の組み合わせで
  `width * result ≤ MAX_IMAGE_DIM && height * result ≤ MAX_IMAGE_DIM` を確認。
  MAX_IMAGE_DIM × MAX_IMAGE_DIM では samples=4 要求が 1 にキャップされることを確認。

## 問132 — HTML ビューアが空メッシュで WebGL 視錐台を正しく退化させないことが未確認

**問**: `encode_html` は `mesh.bounds()` が `None` (空メッシュ) のとき
`(lo, hi) = (Vec3::ZERO, Vec3::ZERO)` にフォールバックし `radius = max(0, 1e-3) = 1e-3`
を使う。WebGL の `persp()` は near = `MESH.radius * 0.05` を使うため、radius=0 だと
near=0 → 投影行列が数値崩壊する。1e-3 最小余白が唯一の防護線だが、テストがなかった。

**仮定の見直し**: 「通常用途で空メッシュのビューアを開くことはない」は正しくない。
検証失敗後の空メッシュを誤って HTML に書き出した場合にビューアが表示できないのは
ユーザーエクスペリエンスの問題になる。

**実装 (html.rs)**:
- `empty_mesh_produces_valid_html_with_nonzero_radius`:
  空メッシュでプレースホルダがすべて置換され、`radius:0.0010` (1e-3) と
  `center:[0.0000,0.0000,0.0000]` が埋め込まれることを確認。

## 問133 — STL `face_normal` の退化三角形処理がテストなし

**問**: `face_normal(a, b, c)` は法線の長さが 0.0 のとき `Vec3::ZERO` を返す。
`from_soup` は重複インデックスを除去するが **共線頂点** (面積ゼロ) は除去しない。
つまり `face_normal` の ZERO パスは Mesh を直接構築した場合に到達可能だが、
この挙動を確認するテストが存在しなかった。

**仮定の見直し**: 「from_soup が退化三角形を取り除く」は重複インデックスに限った話。
共線三角形はインデックスが相異なるため除去されず、STL に 0 ノーマルとして書き出される。
STL 仕様では 0 ノーマルは許容されるが、実装者が変えようとした際に回帰テストがない。

**実装 (stl.rs)**:
- `face_normal_is_unit_for_valid_triangle_and_zero_for_degenerate`:
  XY平面の三角形が +Z 単位法線を返すこと、共線三角形と一致点三角形が
  `Vec3::ZERO` を返すことを確認。

## 問135 — smooth_union はブレンド域外で hard_union と厳密一致することが未確認

**問**: `smooth_union(a, b, k)` の実装は `|da - db| > k` のとき `h` が [0,1] に
クランプされ `k * h * (1-h) = 0` になる (h=0 または h=1)。よって smooth = hard が
**数値誤差ゼロで厳密**に成立するはず。既存テストは `k→0` の収束と
soft ≤ hard の大小関係を確認したが、「ブレンド域外での厳密一致」は未確認だった。

**仮定の見直し**: 「k→0 テストがあれば十分」は誤り。k→0 は全域での近似収束であり、
特定の k (例 k=0.3) においてブレンド域外が厳密ゼロ差になることは異なる主張。

**実装 (sdf.rs)**:
- `smooth_union_exactly_equals_hard_outside_blend_zone`:
  中心間距離 5 の非重複球 (k=0.3) で `|da-db| > k` の代表点を選び、
  `soft.eval(p) == hard.eval(p)` が 1e-14 以内で成立することを確認。

## 反映サマリ v82
| 問 | 実装 |
|----|------|
| 131 | arg_samples 不変条件テスト: dim×samples ≤ MAX_IMAGE_DIM (tools.rs) |
| 132 | 空メッシュ HTML の radius=1e-3 フォールバック確認 (html.rs) |
| 133 | STL face_normal 退化三角形 Vec3::ZERO テスト (stl.rs) |
| 135 | smooth_union ブレンド域外での hard との厳密一致テスト (sdf.rs) |

> 総括: v82 は「コメントで主張されるが証拠のない不変条件」4件を固定した。
> arg_samples のメモリ上限 (問131)、HTML ビューア空メッシュ安全性 (問132)、
> STL 退化法線処理 (問133)、smooth_union の blend 領域外収束 (問135)。
> clippy --all-targets -D warnings = 0 warnings。
> テスト数: 215 ユニット + 統合 3 = 218 合計。

---

## 問136 — sampling_box 電池に Cone/RoundedBox/Ellipsoid が欠如

**問**: `sampling_box_encloses_aabb_for_representative_shapes` (問115) は10形状を
テストするが Cone/RoundedBox/Ellipsoid の 3 プリミティブが含まれていなかった。
これら 3 つは AABB 計算に特有のロジックを持ち (Cone は頂点 z=0 ・底面 z=-h、
RoundedBox はフィレット分の縮小、Ellipsoid は多軸スケール) 電池からの漏れは
退行を無音化する。

**実装 (sdf.rs)**:
- `sampling_box_encloses_aabb_for_representative_shapes` に Cone/RoundedBox/Ellipsoid を追加し 13 形状電池に拡大。

## 問137 — undo_script ツールのユニットテストが皆無

**問**: `undo_script` は単一段undo(single-level)を実装しており、
ツール説明にも「no previous script → error」「undo already applied → error」
と記載されているが、実際の `call_tool("undo_script", …)` を呼ぶ
ユニットテストが 1 件も存在しなかった。
問121 (`failed_run_script_preserves_undo_state_and_scene`) は
`prev_scene` フィールドを直接検査するだけで、undo ツール経路は未カバー。

**実装 (tools.rs)**:
- `undo_script_restores_scene_then_exhausts_single_level`:
  (1) undo before run_script → error、(2) run_script → undo (シーン復元・"undo ok")、
  (3) 2回目 undo → "nothing to undo" error を確認。
- `undo_script_after_failed_run_does_not_corrupt_undo_state`:
  eval_any Err 時は early return で prev_scene が更新されない (失敗 run_script は
  undo 状態を変えない) ことを undo 呼び出しで実証。

## 問138 — validate と validate_with_field(None, ...) の整合性が未テスト

**問**: `validate(mesh, w, o)` は `validate_with_field(mesh, None, w, o, Vec3(0,0,1))`
の 1 行ラッパー。build_dir のデフォルトが誤って変更されても
既存テストは検知しない (両関数は独立して使われているが相互比較テストがなかった)。

**実装 (check.rs)**:
- `validate_is_consistent_with_validate_with_field_default_args`:
  同一メッシュ・同一パラメータで両経路の triangle_count/is_manifold/digest/issues
  が全て一致することをテスト。

## 問139 — sandbox が空白のみのパスを拒否するテストが欠如

**問**: `sandbox_write_path` は `requested.trim().is_empty()` で空白のみのパスを拒否するが、
既存テストは `""` (空文字) のみ確認し `"   "` や `"\t\n"` を確認していなかった。

**実装 (tools.rs)**:
- 既存 `sandbox_rejects_traversal_and_absolute` に `"   "` と `"\t\n"` を追加。

## 反映サマリ v83
| 問 | 実装 |
|----|------|
| 136 | sampling_box 電池に Cone/RoundedBox/Ellipsoid を追加し 13 形状に拡大 (sdf.rs) |
| 137 | undo_script 2テスト: 正常undo・失敗後undo の両パスを実証 (tools.rs) |
| 138 | validate == validate_with_field(None,...) の整合性回帰テスト (check.rs) |
| 139 | sandbox 空白パス拒否テスト追加 (tools.rs) |

> 総括: v83 は「ツールとして実装されているが直接テストされていない経路」を埋めた。
> undo_script (問137) は実装から漏れていた最も重要なテストギャップ。
> validate 整合性 (問138) は暗黙のデフォルト依存に対する退行ガード。
> clippy --all-targets -D warnings = 0 warnings。
> テスト数: 218 ユニット + 統合 3 = 221 合計。

---

## 問140 — encode_glb(空メッシュ) の正常系テストが欠如

**問**: `encode_glb` には空メッシュへの安全ガード (min/max → [0,0,0]) があり
コメントにも言及されているが、この経路を実際に通すテストが存在しなかった。
非空メッシュのテストのみが4件あり、頂点数0・三角形数0の場合に
GLBヘッダが正常か、JSONがパース可能か、accessor count が 0 になるかが未検証。

**実装 (gltf.rs)**:
- `empty_mesh_produces_valid_parseable_glb`:
  `Mesh::default()` → GLB magic/version 正常、JSON chunk が parse 可能、
  accessor 数 = 2、両 accessor の count = 0 を確認。
  `encode_glb` の空ガード (lines 50-53) を初めてカバーするテスト。

## 問141 — from_soup(&[]) の戻り値と下流関数の動作が未テスト

**問**: `Mesh::from_soup` はドキュメントに「空スープ → 空メッシュ」とあるが
その経路のユニットテストが存在しなかった。また `body_components`・`signed_volume`・
`bounds`・`is_edge_manifold` が empty mesh を受け取ったとき
それぞれどう振る舞うかも暗黙の前提として残っていた。

**実装 (mesh.rs)**:
- `from_soup_with_empty_input_returns_empty_mesh`:
  `from_soup(&[])` → vertices/triangles が空、`body_components() == (0,0)`、
  `signed_volume() == 0.0`、`bounds() == None`、`is_edge_manifold() == true`
  (辺なし = trivially manifold) をすべて確認。

## 問142 — sampling_box 電池に Mirror が欠如

**問**: sampling_box 電池 (問136 で 13 形状に拡大) は
Cone/RoundedBox/Ellipsoid は含むが `mirror_x/y/z` が存在しなかった。
Mirror は内部で Translate+Sdf::Mirror の合成だが `aabb()` 実装も独立しており
未確認だった。

**実装 (sdf.rs)**:
- sampling_box_encloses_aabb 電池を14形状に拡大:
  `Sdf::sphere(0.5).translate(Vec3::new(1.5, 0.0, 0.0)).mirror_x()` を追加。
  Mirror後のAABBが sampling_box によって完全に内包されることを確認。

## 問143 — Vec3::ZERO build_dir で overhang チェックが黙ってスキップされることが未テスト

**問**: `validate_with_field` は `build_dir.length() < 1e-12` の場合
オーバーハングチェックをスキップするが、このサイレント挙動を確認するテストが
存在しなかった。ゼロベクトルが誤って build_dir に渡されたとき、
OVERHANG issue が誤って報告されないことが保証されていなかった。

**実装 (check.rs)**:
- `zero_build_dir_silently_skips_overhang_check_without_crash`:
  `build_dir = Vec3::ZERO` および `Vec3::new(0,0,1e-14)` で
  validate_with_field を呼び出し、`OVERHANG` issue が出ないこと、
  かつパニックしないことを確認。

## 反映サマリ v84
| 問 | 実装 |
|----|------|
| 140 | encode_glb 空メッシュテスト: GLBヘッダ・JSON・accessor count を確認 (gltf.rs) |
| 141 | from_soup 空入力テスト: 全下流関数の空メッシュ挙動を固定 (mesh.rs) |
| 142 | sampling_box 電池に Mirror を追加し 14 形状に拡大 (sdf.rs) |
| 143 | zero/極小 build_dir で OVERHANG がスキップされることを確認 (check.rs) |

> 総括: v84 は「ガードコードは書いたがテストは書かなかった」パターンを埋めた。
> encode_glb の空メッシュガード (問140)・from_soup の空入力 (問141) は
> 実装時にコメントで言及されながら検証されていなかった典型例。
> Mirror (問142) は電池の系統的な漏れ、build_dir=0 (問143) はサイレント
> 挙動の退行ガード。
> clippy --all-targets -D warnings = 0 warnings。
> テスト数: 221 ユニット + 統合 3 = 224 合計。

---

## 問144 — JSON パーサが末尾カンマを拒否することがテストで固定されていない

**問**: JSON 仕様 (RFC 8259) は `[1,2,]` や `{"a":1,}` のような末尾カンマを禁止するが、
パーサはカンマ後に `parse_value()` を再呼び出しする構造上、`]`/`}` を
`parse_value_inner` 内で "unexpected byte" として自然に拒否する。
この動作はテストがなく、リファクタリングで無音に許容されるようになっても検知できない。

**実装 (json.rs)**:
- `trailing_comma_is_rejected`:
  `[1,2,]` および `{"a":1,}` がどちらもエラーを返すことを固定。
  正常系 (`[1,2]`, `{"a":1}`) は通ることも確認。

## 問145 — JSON オブジェクトの重複キーが last-wins になることが未テスト

**問**: `parse(r#"{"a":1,"a":2}"#)` は BTreeMap::insert の動作により
後の値 ("a":2) が前の値 ("a":1) を無音で上書きする。
MCP リクエストに重複フィールドが混入した場合 (例: 二重パラメータインジェクション)
どの値が使われるかが未固定。

**実装 (json.rs)**:
- `duplicate_object_keys_last_wins`:
  `{"a":1,"b":2,"a":99}` を parse すると "a"=99 (後の値) になることを確認。
  セキュリティ動作 (last-wins) を明示的に文書化・固定。

## 問146 — MAX_NODES ノード数上限が eval_scene では SOURCE 上限に隠れて未テスト

**問**: `eval_scene` は `MAX_SOURCE_BYTES` (1 MiB) を先にチェックするため、
50,000 ノードに必要な JSON ソース (各ノード ≥ 20 バイト → ≥ 1 MB) では
SOURCE 上限が先に発動し MAX_NODES に到達しない。
よって `MAX_NODES` 強制コードは `eval_scene` 経路では事実上デッドコードだった。

**実装 (eval.rs)**:
- `node_budget_is_shared_and_enforced_for_wide_trees`:
  `eval_value` で深さ17の完全二分木 (131,071 ノード) を Value として渡し
  "too large" エラーが返ることを確認。深さ14 (16,383 ノード) は受理されることも確認。
  `budget: &mut Budget` が全再帰呼び出しで共有されることを実証。

## 問147 — \uXXXX サロゲートコードポイントが U+FFFD に変換されることが未テスト

**問**: `parse_string` の `\uXXXX` 処理は `char::from_u32(cp).unwrap_or('\u{FFFD}')` を使う。
UTF-16 サロゲート (U+D800-U+DFFF) は `char::from_u32` が None を返すため
置換文字 U+FFFD に変換される。この動作 (パニックしない・定義済みの縮退) が未テスト。
MCP クライアントが `😀` 形式でサロゲートペアを送った場合の動作が不明だった。

**実装 (json.rs)**:
- `unicode_surrogate_escape_becomes_replacement_char`:
  `"\uD800"` (孤立サロゲート) → `"\u{FFFD}"` を確認。
  `"😀"` (サロゲートペア, 😀) → `"\u{FFFD}\u{FFFD}"` (2つの置換文字) を確認。
  パニックしないこと・定義済みの縮退であることを固定。

## 反映サマリ v85
| 問 | 実装 |
|----|------|
| 144 | JSON 末尾カンマ拒否テスト (json.rs) |
| 145 | JSON 重複キー last-wins 固定 (json.rs) |
| 146 | eval_value 経由で MAX_NODES を実際にテスト (eval.rs) |
| 147 | \uXXXX サロゲート → U+FFFD 変換テスト (json.rs) |

> 総括: v85 は「動作は正しいがテストがなく退行が検知不能」パターンを埋めた。
> MAX_NODES (問146) は eval_scene 経路では SOURCE 上限に隠れており、
> eval_value を直接呼ぶことで初めて実証できた実質的なデッドコード発見。
> サロゲートエスケープ (問147) はパニック経路ではないが RFC 8259 準拠の
> 定義済み縮退として明示することで将来の回帰を防ぐ。
> clippy --all-targets -D warnings = 0 warnings。
> テスト数: 225 ユニット + 統合 3 = 228 合計。

---

## 問148 — プリミティブ aabb() の数値が未テスト (退行で縮小しても無検知)

**問**: `aabb()` の各プリミティブ実装は数値コメントを持つが、実際に期待値を返すかを
確認するテストが存在しなかった。退行で Cylinder の `half_height` → `height` 混同や
Torus の `major+minor` → `major-minor` 取り違えが起きても、AABB が縮小するだけで
サイレントに不完全なメッシュが出力される。
また `cylinder(radius, half_height)` の第2引数が `height` ではなく `half_height` で
あることも、テストがないと利用者に伝わらない。

**発見 (副次的)**: テスト作成中に `cylinder(0.5, 2.0)` の Z AABB が ±1.0 ではなく
±2.0 であることを確認し、API ドキュメントの誤解を防ぐコメントを追加した。

**実装 (sdf.rs)**:
- `aabb_exact_values_for_primitives`:
  Sphere(1.5)/Cylinder(0.5, 2.0)/Torus(2.0, 0.5)/Cone(1.0, 2.0) の
  aabb() 返り値が期待数値 (±radius, ±half_height 等) と一致することを固定。

## 問149 — scale(-1) を直接構築すると aabb() が反転するが sampling_box() の正規化が未テスト

**問**: `eval.rs` は `s <= 0` を拒否するが、`Sdf::sphere(1.0).scale(-2.0)` と
直接構築できる (eval 層をバイパス)。この場合 `aabb()` は `(lo * -2, hi * -2)` で
lo.x > hi.x の反転ボックスを返す。`sampling_box()` の正規化が反転後も
`lo <= hi` を保証することの明示テストが欠如していた。

**実装 (sdf.rs)**:
- `scale_negative_factor_sampling_box_is_normalized`:
  `Sdf::sphere(1.0).scale(-2.0)` の `sampling_box()` が全軸 `lo <= hi` を
  返すことを確認。polygonize が負ステップで壊れないための防護線を固定。

## 問150 — TETS 6 四面体が単位立方体を充填することの数学的検証が未テスト

**問**: `TETS: [[usize; 4]; 6]` は「0-6 対角を共有する6四面体分割。
立方体を隙間なく充填する」とコメントされているが、この性質を検証するテストが
存在しなかった。TETS を誤って変更しても watertight テストでしか検知できず、
どの四面体が問題かを特定できない。

**実装 (marching_tetrahedra.rs)**:
- `tets_volumes_sum_to_unit_cube_volume`:
  各四面体の符号付き体積 = `(b-a)·((c-a)×(d-a)) / 6` を計算し、
  6 四面体の合計が 1.0 (単位立方体体積) と一致することを確認。
  体積和が 1 でなければ隙間か重複のいずれかが存在する。

## 反映サマリ v86
| 問 | 実装 |
|----|------|
| 148 | primitive aabb() 数値固定テスト: cylinder half_height バグを副次的に発見 (sdf.rs) |
| 149 | scale(-1) 直接構築後の sampling_box 正規化を固定 (sdf.rs) |
| 150 | TETS 体積和 = 1 の数学的検証テスト (marching_tetrahedra.rs) |

> 問151 (read_message SIZE 境界テスト) は既に問118 の実装時に追加済みであることを確認。
> 総括: v86 は「コメントに書いてあるが検証されていない数学的性質」パターンを埋めた。
> 問148 はテスト作成中に cylinder API の誤解 (height vs half_height) を副次的に発見。
> 問149 は eval.js バリデーションをバイパスした Sdf 直接構築に対するロバスト性を確認。
> 問150 は TETS 分割の必要条件 (体積 = 1) を一行計算で確認する最小テスト。
> clippy --all-targets -D warnings = 0 warnings。
> テスト数: 228 ユニット + 統合 3 = 231 合計。

---

## 問151 — DSL 裸の数値がトップレベルで拒否されることのテストが欠如

**問**: `parse_dsl("1.5")` は DSL パーサが数値を引数文脈で受け入れるため
`Ok(Value::Number(1.5))` を返す。その後 `eval_value` が "missing op field" でエラーを返す。
この連携した拒否動作のテストが存在しなかった。
AI エージェントや生成スクリプトが `"1.5"` を形状式として送った場合、
意味のあるエラーが返ることが保証されていなかった。

**実装 (dsl.rs)**:
- `bare_numeric_top_level_is_rejected`:
  `eval_dsl("1.5")`, `eval_dsl("0")`, `eval_dsl("-3.14")` がエラーを返すことを確認。

## 問153 — 回転の往復恒等が Z 軸のみテストで X・Y 軸が未確認

**問**: `rotation_composition_is_additive_roundtrip_and_order_dependent` (問110) は
`rotate_z(θ).rotate_z(-θ)` の往復恒等のみ確認している。
X・Y 軸は `rotate_point` 内で異なる行列行 (axis=0: Y-Z 回転行列、axis=1: X-Z 回転行列)
を使うため独立したテストが必要。

**実装 (sdf.rs)**:
- `rotation_roundtrip_holds_for_x_and_y_axes`:
  `rotate_x(θ).rotate_x(-θ)` と `rotate_y(θ).rotate_y(-θ)` が非対称直方体形状で
  恒等変換になることを非自明な角度 (0.7 rad) でグリッドテスト。

## 問154 — Repeat snap の半周期境界での丸め動作が未テスト

**問**: `snap()` は `(v/period).round().clamp(-n, n)` を使う。Rust の `f64::round()` は
「ゼロから遠い方向」に丸める (round-half-away-from-zero)。`period=2.0, v=1.0` では
`0.5.round() = 1.0` → 隣接セル (x=2.0) に snap される。
この挙動がバンカー丸め (round-half-to-even) と異なることと、その結果が
整合的であることが未固定だった。

**実装 (sdf.rs)**:
- `repeat_snap_at_half_period_maps_to_neighbor_cell`:
  `period=2.0` で中間点 `x=1.0` が外部 (両球から距離 0.7)、
  `x=-1.0` も同じ距離であることを確認。隣接セルの球 (x=2.0) は内部であることも確認。

## 問155 — 未知演算子が入れ子でも拒否されることのテストが欠如

**問**: `malformed_dsl_is_rejected` テストは `wobble(1)` (トップレベル) のみ確認。
`union(wobble(1), sphere(1))` や `translate(0,0,0, sphire(1))` のように
有効な関数呼び出しの引数に未知演算子が含まれる場合のエラー伝播が未テストだった。

**実装 (dsl.rs)**:
- `unknown_operator_rejected_both_at_top_and_nested`:
  `sphire(1)` (タイポ)、`union(sphire(1), sphere(1))`、
  `translate(0,0,0, wobble(1))` がすべてエラーを返すことを確認。

## 反映サマリ v87
| 問 | 実装 |
|----|------|
| 151 | DSL 裸数値のトップレベル拒否テスト (dsl.rs) |
| 153 | rotate_x/rotate_y の往復恒等グリッドテスト (sdf.rs) |
| 154 | repeat snap 半周期境界の丸め動作固定 (sdf.rs) |
| 155 | 未知演算子の入れ子拒否テスト (dsl.rs) |

> 総括: v87 は「正しい動作だがテストがないため退行が無音」パターンをさらに埋めた。
> rotate_x/y の往復恒等 (問153) は Z 軸が問110 でテスト済みのため見落とされがちな盲点。
> repeat snap (問154) は Rust の丸め動作 (round-half-away-from-zero) という言語依存仕様を固定。
> DSL の入れ子拒否 (問155) は AI エージェントがタイポで生成した入れ子式のエラー経路。
> clippy --all-targets -D warnings = 0 warnings。
> テスト数: 232 ユニット + 統合 3 = 235 合計。

---

## 問156 — cross(v, kv) 平行ベクトルがゼロになることのテストが欠如

**問**: `cross_self_is_zero` は `v × v = 0` のみ確認。`v × (kv)` (k ≠ 1) は
別の計算経路を通るが未テスト。`stl.rs` の `face_normal` は外積でゼロを検出し
退化三角形を判定するため、この性質の退行テストが重要。

**実装 (math.rs)**:
- `cross_collinear_vectors_yield_zero`:
  `v × 2v`、`v × (-v)`、`v × 0.001v` がいずれも `Vec3::ZERO` になることを確認。

## 問157 — ellipsoid の極端な縦横比 (1000:1) での近似が NaN にならないことが未テスト

**問**: IQ 近似式は `k1 = length²(p/r²) / r` という値を使い、縦横比が大きいと
`k1` が f64 精度で極小になりうる。`k1 → 0` 時に `k0*(k0-1)/k1` が Inf になる
可能性があるが、符号が厳密であるという保証のみでこの経路が未検証だった。

**実装 (sdf.rs)**:
- `ellipsoid_extreme_asymmetry_is_finite_and_correct_sign`:
  `radii = (1000.0, 0.001, 0.5)` で中心・X軸表面・X軸内部の評価が
  すべて有限値かつ正しい符号を持つことを確認。

## 問159 — volume_reliable() の論理積ガードが片側独立にテストされていない

**問**: `volume_reliable() = is_manifold && triangle_count > 0` の両条件は
`validate()` が常に整合的に計算するため、片方が誤って true になっても
既存テストは検知できない。`Report` を直接構築してガードの各辺を独立に固定する
テストが欠如していた。

**副次的変更**: `Report` 構造体に `#[derive(Clone)]` を追加 (テスト用直接構築のため)。

**実装 (check.rs)**:
- `volume_reliable_conjunction_requires_both_conditions`:
  `{ triangle_count: 0, is_manifold: true }` → false、
  `{ triangle_count: 100, is_manifold: false }` → false、
  `{ triangle_count: 100, is_manifold: true }` → true を各々確認。

## 問160 — arg_samples の実際の削減量が未固定 (不変条件テストは削減を検証しない)

**問**: `arg_samples_invariant` は `width*samples ≤ MAX_IMAGE_DIM` を確認するが、
「samples が実際に削減された」かどうかを確認しない。`min(cap_w)` 節を削除しても
外側の `arg_dim` ガードが成立する限り不変条件テストは通過しうる。

**実装 (tools.rs)**:
- `arg_samples_actually_reduces_when_dimension_limits_it`:
  `width=2048, samples_requested=4` → `samples=2` (削減確認)、
  `width=512, height=MAX_IMAGE_DIM, requested=4` → `samples=1`、
  `width=1024, requested=4` → `samples=4` (削減なし) を具体値でテスト。

## 反映サマリ v88
| 問 | 実装 |
|----|------|
| 156 | cross(v, kv) 平行ゼロテスト (math.rs) |
| 157 | ellipsoid 極端縦横比の NaN 非発生確認 (sdf.rs) |
| 159 | volume_reliable() 論理積ガードを片側独立テスト + Report に Clone 追加 (check.rs) |
| 160 | arg_samples 実削減量を具体値で固定 (tools.rs) |

> 総括: v88 は「不変条件テストが論理の片側しか確認しない」パターンを埋めた。
> volume_reliable の論理積 (問159) は validate() が整合性を保証するため
> 独立テストがないと半側削除が無音になる典型例。
> arg_samples 削減 (問160) は外側ガードが代替するため偽陰性を生む微妙な構造。
> ellipsoid 極端縦横比 (問157) は IQ 近似の精度境界への探索的テスト。
> clippy --all-targets -D warnings = 0 warnings。
> テスト数: 236 ユニット + 統合 3 = 239 合計。

## 問161 — scale(1.0) の恒等性が未確認

**問**: `uniform_scale_preserves_distance_field` は factor=2.0 のみを検証している。
scale(1.0) では `p / 1.0` という f64 除算が起きるが、正確に恒等写像になるか
(丸め誤差が生じないか) を確認するテストが存在しなかった。

**実装 (sdf.rs)**:
- `scale_factor_one_is_identity`:
  sphere(1.0).scale(1.0) が 4 プローブ点すべてで child.eval(p) と `==` (ビット同一) を確認。

## 問162 — count=0 が実際に軸を無効化するか未確認

**問**: `snap()` は `if per == 0.0 || n == 0 { return v; }` を持ち、count=0 で
繰り返しを無効化する。しかし「count=0 の軸は繰り返さない」という動作を直接
テストするケースが存在せず、この早期リターンが黙って消えても既存テストは通過する。

**実装 (sdf.rs)**:
- `repeat_count_zero_disables_axis_with_positive_period`:
  repeat_n(period=[2,2,2], count=[1,0,1]) で x 軸 (count=1) は同値、
  y 軸 (count=0) は y=0 と y=2.0 で値が異なることを確認。

## 問163 — smooth bool の k→0 での NaN/Inf 安全性

**問**: k=1e-6 で収束性を確認するテストはあるが、k が 1e-100〜1e-300 の
極限領域で `(da-db)/k → ±∞` になり NaN/Inf が生じないかを確認していなかった。
clamp(h, 0, 1) が吸収するはずだが、実際に吸収されているかは未検証。

**実装 (sdf.rs)**:
- `smooth_union_and_intersection_remain_finite_for_tiny_k`:
  k ∈ {1e-1, 1e-6, 1e-12, 1e-100, 1e-300} で smooth_union/intersection/difference を
  4 プローブ点で評価し、すべて is_finite() を確認。

## 問164 — STL 法線の near-degenerate 三角形での NaN 安全性

**問**: `face_normal` は `len == 0.0` を明示的にチェックするが、
len が 1e-150 オーダーの「ほぼ退化」三角形では 1/len が Inf になりうる。
既存テストは完全退化 (len=0) と正常 (len=1) のみで、中間領域を確認していない。

**実装 (stl.rs)**:
- `face_normal_near_degenerate_is_finite_and_not_nan`:
  辺長 1e-75 の三角形 (外積長 ≈ 1e-150) で face_normal を呼び、
  各成分が finite で長さが 0 か単位長であることを確認。

## 問165 — edge_vertex 結果が線分端点の間に収まることの形式的確認

**問**: `edge_vertex_clamp_produces_valid_interpolation` は 2 ケースのみで
「x が [0,1] 内」を確認するが、f64 の境界値 (f64::MAX、1e-200 vs 1e200 等)
での挙動を確認していない。clamp(t, 0, 1) で守られているはずだが未検証。

**実装 (marching_tetrahedra.rs)**:
- `edge_vertex_result_lies_within_segment_endpoints`:
  8 種の (va, vb) ペア (同符号正負、異符号、ゼロ端、極端倍率差、f64::MAX) で
  結果 x が [0.0, 1.0] に収まることを確認。

## 反映サマリ v89
| 問 | 実装 |
|----|------|
| 161 | scale(1.0) 恒等性をビット同一で確認 (sdf.rs) |
| 162 | count=0 が軸を実際に無効化することを確認 (sdf.rs) |
| 163 | smooth bool k→1e-300 で finite を確認 (sdf.rs) |
| 164 | STL face_normal near-degenerate で NaN/Inf なし (stl.rs) |
| 165 | edge_vertex 結果が常に線分上 (marching_tetrahedra.rs) |

> 総括: v89 は「コード内の早期リターン/クランプ/ガード節が実際に機能するか」を
> 独立テストで固定した。scale(1.0) 恒等性・count=0 軸無効・smooth bool k→0 安全性は
> 既存テストでは検知できない削除に対して盲点になっていた。
> clippy --all-targets -D warnings = 0 warnings。
> テスト数: 241 ユニット。

## 問166 — 空入力・空白のみ入力の JSON パース

**問**: `malformed_literals_are_rejected` は `nul`/`tru` 等の誤綴りを確認するが、
完全に空の入力 `parse("")` や空白のみ `parse("   ")` を確認していない。
MCP フレームが空ボディを送った場合に黙って null 等に化けないか未検証。

**実装 (json.rs)**:
- `empty_and_whitespace_only_input_is_rejected`:
  `parse("")`, `parse("   ")`, `parse("\n\t ")` がすべてエラーで
  メッセージが "unexpected" を含むことを確認。

## 問167 — 未知エスケープが文字を二重化するバグ (実バグ発見・修正)

**問**: `parse_string` の `other` 分岐 (旧 line 326) は
`s.push(other.unwrap_or(b'?') as char)` で未知エスケープ文字を push するが
**advance を呼ばない**。次ループで同じ文字が `Some(c)` 経由で再 push され、
`\q` → "qq" のように文字が二重化していた。`string_escaping` は有効エスケープ
のみ確認しており、この経路は完全に未検証だった。

**修正 (json.rs)**: `other` 分岐を `Some(c)` / `None` に分割し、
未知エスケープは `invalid escape \X` エラー、末尾バックスラッシュは
`unterminated string: trailing backslash` エラーとする (malformed literal 等と
同様の厳密拒否)。二重化バグを根絶。

**実装 (json.rs)**:
- `invalid_escape_is_rejected_not_silently_doubled`:
  `parse(r#""a\qb""#)` がエラー (旧挙動 "aqqb" の回帰防止)、
  `\n`/`\\`/`\"` は引き続き正常動作、末尾バックスラッシュもエラーを確認。

## 問168 — 配列の先頭・中間コンマ

**問**: `trailing_comma_is_rejected` は `[1,2,]` のみ。先頭コンマ `[,1]` や
中間二重コンマ `[1,,2]` (値欠落) は未確認。

**実装 (json.rs)**:
- `array_with_leading_or_middle_comma_is_rejected`:
  `[,1]`, `[1,,2]`, `[ , ]` がエラー、正常 `[1,2]` は通ることを確認。

## 問171/172 — Content-Length 欠落・非数値は同一に拒否

**問**: `oversized_content_length_is_rejected_before_allocation` は上限超過のみ確認。
ヘッダ欠落と非数値値 (`Content-Length: notanumber`) は未確認。後者は
`parse().ok()` が None になり「欠落」と同一経路で拒否される (この同値性も未固定)。

**実装 (server.rs)**:
- `missing_or_non_numeric_content_length_is_rejected_identically`:
  別ヘッダのみ / 非数値値の両方が `InvalidData` + "missing Content-Length" で
  拒否されることを確認。非数値が欠落と同一扱いになる契約を固定。

## 問174 — 明示的ゼロカウントの repeat 縮退

**問**: `repeat_count_without_period_is_rejected` は count>0 & period=0 を確認するが、
明示的 `nx=ny=nz=0` & period>0 のケースは検証 (cnt>0.0 ガード) を通過し
エラーにならない。snap() の n==0 で全軸無効化 → 単一形状縮退になる契約が未固定。

**実装 (eval.rs)**:
- `repeat_with_explicit_zero_counts_degenerates_to_single_shape`:
  全軸 count=0 & period=2.0 が素の sphere(0.3) と同一距離場になることを
  3 プローブ点で確認。

## 問175 — 引数 0 個の DSL 関数呼び出し

**問**: `unknown_operator_rejected_*` は未知演算子を確認するが、既知演算子の
引数 0 個呼び出し `sphere()` は未確認。`want(n)` ガードが "got 0" を返すか未検証。

**実装 (dsl.rs)**:
- `function_call_with_zero_arguments_is_rejected`:
  `sphere()` がエラー (メッセージ "got 0")、`cuboid()`/`union()`/`translate()` も
  拒否されることを確認。

## 反映サマリ v90
| 問 | 実装 |
|----|------|
| 166 | 空入力・空白のみ入力の拒否 (json.rs) |
| 167 | **実バグ修正**: 未知エスケープの文字二重化を根絶し厳密拒否 (json.rs) |
| 168 | 配列の先頭・中間コンマ拒否 (json.rs) |
| 171/172 | Content-Length 欠落・非数値の同一拒否 (server.rs) |
| 174 | 明示ゼロカウント repeat の単一形状縮退 (eval.rs) |
| 175 | 引数 0 個の DSL 関数呼び出し拒否 (dsl.rs) |

> 総括: v90 はソクラテス問答が**実バグ**を発見した回。問167 の未知エスケープ
> 二重化 (`\q` → "qq") は parse_string の `other` 分岐が advance を欠いていたため
> で、有効エスケープのみテストしていたため完全な盲点だった。厳密拒否へ修正。
> 他は JSON/MCP/DSL の error path・境界条件の網羅。
> clippy --all-targets -D warnings = 0 warnings。
> テスト数: 247 ユニット + 統合 3 = 250 合計。

## 問176 — Shell(thickness=0) が絶対値場に縮退することを未確認

**問**: Shell の数式 `d.max(-(d + thickness))` は thickness=0 で `d.max(-d) = |d|` になるが、
この縮退ケースに対するテストが存在しなかった。既存テストは thickness=0.1/0.25/0.3 のみ。

**実装 (sdf.rs)**:
- `shell_zero_thickness_equals_absolute_value_field`:
  sphere(1.0).shell(0.0).eval(p) == |sphere(1.0).eval(p)| を 5 プローブ点で確認。

## 問177 — Scale(0.0) が NaN を返すことを未文書化

**問**: Sdf::scale(0.0) は eval.rs が s<=0 を拒否するため通常は現れないが、
Sdf:: API を直接呼ぶと `0.0 * child.eval(p / 0.0)` = `0.0 * NaN` = NaN になる。
この挙動 (パニックしないが NaN) が未文書化・未固定だった。

**実装 (sdf.rs)**:
- `scale_zero_factor_eval_produces_nan_not_panic`:
  scale(0.0).eval がパニックせず NaN を返すこと、aabb は有限 (全ゼロ) であることを確認。

## 問178 — Capsule(half_height < radius) の縮退動作が未確認

**問**: capsule(half_height, radius) で half_height < radius の場合、両半球が重なる
縮退形状になる。既存テストはすべて half_height >= radius のみ。
加えて引数順 (half_height, radius) が直感と逆で混乱を招くことも発見。

**実装 (sdf.rs)**:
- `capsule_radius_exceeds_half_height_degenerates_to_sphere_like`:
  capsule(0.1, 1.0) で中心 d=-1.0、端面 d=0、遠点 d>0、
  全点 finite を確認。引数順に注釈追加。

## 問179 — body_components の決定性を反復呼び出しで未確認

**問**: body_components は Union-Find 後に HashMap に体積を集計するが、
HashMap の反復順序は標準保証されない。カウント結果が呼び出しごとに
同一であるという「問5 決定性」の主張をテストで確認していなかった。

**実装 (mesh.rs)**:
- `body_components_is_deterministic_under_repeated_calls`:
  2 球モデルで body_components を 5 回呼び出し、すべての結果が一致することを確認。

## 問182 — Offset(−大量, 微小 child) の AABB 数値安定性が未確認

**問**: offset_negative_aabb_tightens は Sphere(1.0).offset(-0.4) のみ。
Sphere(1e-10).offset(-1.0) のような child << offset 量 では
lo ≈ −1.0 + 1e-10 ≈ −1.0 と hi ≈ 1e-10 の精度損失が起きうる。
正規化後も lo<=hi かつ finite が保証されるか未確認。

**実装 (sdf.rs)**:
- `offset_negative_extreme_scale_ratio_aabb_is_finite_and_normalized`:
  Sphere(1e-10).offset(-1.0) の aabb が finite かつ lo<=hi、
  eval(Vec3::ZERO) が finite であることを確認。

## 反映サマリ v91
| 問 | 実装 |
|----|------|
| 176 | Shell(0) = |d| 絶対値場縮退 (sdf.rs) |
| 177 | Scale(0) NaN 挙動を文書化・固定 (sdf.rs) |
| 178 | Capsule half_height<radius 縮退 + 引数順発見 (sdf.rs) |
| 179 | body_components 反復決定性 (mesh.rs) |
| 182 | Offset 極端スケール比の AABB 安定性 (sdf.rs) |

> 総括: v91 は SDF プリミティブの縮退ケース (thickness=0, factor=0, hh<r) と
> body_components の反復決定性を固定した。Scale(0.0) の NaN 挙動は
> eval.rs がすでに防いでいるが Sdf:: API 直呼び時の挙動を文書化。
> Capsule の引数順 (half_height, radius) も同時に明確化。
> clippy --all-targets -D warnings = 0 warnings。
> テスト数: 252 ユニット + 統合 3 = 255 合計。

## 問183 — build_dir 短い配列の +Z フォールバックが未確認

**問**: arg_build_dir は 3 要素未満の配列を +Z にフォールバックする (問85 の契約) が、
この経路をテストで確認していなかった。[1,0] が誤って [1,0,1] に補完されないことを固定。

**実装 (tools.rs)**:
- `arg_build_dir_short_array_falls_back_to_plus_z_not_partial_fill`:
  [1,0] と [1] が +Z にフォールバック、正常な [1,0,0] はそのまま使われることを確認。

## 問184 — rotate_box の負角・X/Y 軸の範囲入れ替えが未確認

**問**: rotate_z_90deg_swaps_extents は +90°/z 軸のみ。負角 (-90°) と x/y 軸の
回転行列の行順 (sin/cos の適用) は別の式を通るため未検証だった。

**実装 (sdf.rs)**:
- `rotate_box_negative_and_per_axis_swaps_extents_symmetrically`:
  ±90° z 回転が対称箱で同一 aabb、x 軸回転で y↔z 入れ替え、
  y 軸回転で x↔z 入れ替えを具体値で確認。

## 問185 — 非重複 Intersection の反転 AABB でも eval が正しい

**問**: 非重複の hard Intersection は aabb が反転 (lo > hi) するが、eval が
正しく外部値を返すことと sampling_box が正規化することを同時に確認する
テストがなかった (sampling_box 正規化のみ、または smooth 版のみ)。

**実装 (sdf.rs)**:
- `intersection_of_nonoverlapping_shapes_inverts_aabb_but_eval_is_exterior`:
  sphere(1) ∩ sphere(1)@x=10 で aabb 反転 (lo.x>hi.x)、eval(原点)=max(-1,9)=9 (外部)、
  sampling_box 正規化を確認。

## 問186 — mirror_box の AABB 対称化 vs eval の +x 半分保持規約の乖離

**問**: mirror_box は ext=max(|lo|,|hi|) で aabb を対称化するが、eval は
child.eval(|x|,..) で **+x 半分を -x へ反射**する規約。完全に -x 側にある形状を
mirror_x すると aabb は [-3.5,3.5] に広がるが幾何は空 (|x|>=0 が child に届かない)。
この保守境界 vs 実空集合の乖離が未文書化だった。

**実装 (sdf.rs)**:
- `mirror_box_symmetrizes_aabb_but_eval_keeps_only_positive_half`:
  -x 側形状: aabb 対称 [-3.5,3.5] だが eval は全点外部 (空)。
  対照で +x 側形状は反射コピーが両側に現れることを確認。

## 問189 — Unix でのバックスラッシュパスが literal ファイル名として安全

**問**: sandbox_write_path は Path::components() で判定するが、Unix では '\\' は
パス区切りでなくファイル名の一文字。"a\\..\\escape.stl" は単一コンポーネントになり
ParentDir と解釈されず脱出しない。この platform 契約 (バックスラッシュ=安全) が未固定。

**実装 (tools.rs)**:
- `sandbox_backslash_path_is_literal_filename_on_unix_not_traversal` (#[cfg(unix)]):
  "a\\..\\escape.stl" が Ok (literal ファイル名)、"a/../escape.stl" は Err (回帰防止)。

## 反映サマリ v92
| 問 | 実装 |
|----|------|
| 183 | build_dir 短配列の +Z フォールバック (tools.rs) |
| 184 | rotate_box 負角・X/Y 軸の範囲入れ替え (sdf.rs) |
| 185 | 非重複 Intersection 反転 AABB でも eval 正しい (sdf.rs) |
| 186 | mirror AABB 対称化 vs eval 半分保持の乖離 (sdf.rs) |
| 189 | Unix バックスラッシュパスの literal 安全性 (tools.rs) |

> 総括: v92 は変換 (rotate/mirror) の AABB と eval の関係、CSG の反転 AABB、
> MCP の入力フォールバック・パスサンドボックスを固定した。問186 で mirror の
> 「aabb は対称化するが eval は +x 半分のみ保持」という規約上の乖離を明文化。
> 問189 は Unix でバックスラッシュが literal = 脱出不能であることを platform 契約として固定。
> clippy --all-targets -D warnings = 0 warnings。
> テスト数: 257 ユニット + 統合 3 = 260 合計。

## 問191 — cone の負値パラメータが未確認

**問**: `zero_or_negative_primitive_dimensions_are_rejected` は cone r=0/h=0 を確認するが、
r<0/h<0 (req_positive_f64 の `f<=0` の負側) は未確認。
同じ関数 (sphere は r<0 まで確認) なのに cone は境界 =0 のみという非対称テストがあった。

**実装 (eval.rs)**:
- `cone_negative_radius_or_height_is_rejected`:
  r=-1.0/h=-1.0/両方負値が拒否され、r>0/h>0 は通ることを確認。

## 問190 — offset の負値が意図的に許可される契約を明文化

**問**: offset は req_f64 (正値制限なし) を使い、負値 (収縮) を意図的に許可する。
eval.rs の "inflates/deflates" コメントと整合するが、テストで固定していなかった。
(agent が 問190 として「未検証」と指摘したが、実際は意図的設計 → 文書化で対応)

**実装 (eval.rs)**:
- `offset_negative_amount_shrinks_sphere_correctly`:
  offset(+0.5) が x=1.5 を表面に引き込み、offset(-0.5) がさらに遠ざけることを
  数値 (d=0, d=1.0) で確認。

## 問193 — sdf_gradient の単体テストが存在しなかった

**問**: sdf_gradient は check.rs で使われる重要な関数だが専用テストがなかった。
min_wall_probe は gl<1e-12 をフィルタするが、勾配値の正しさ自体は未確認。

**実装 (check.rs)**:
- `sdf_gradient_points_outward_on_sphere_surface`:
  球面上 (1,0,0) で勾配が有限・非ゼロ・x 方向 99% 以上を確認。
  中央差分の実装詳細 (長さ ≈ 2h) もコメントで文書化。

## 問194 — min_wall_probe の縮退境界ガードが未確認

**問**: `if diag <= 0.0 || v == 0 { return None; }` (line 427) の早期リターンパスが
既存テスト (probe_measures_shell_thickness 等) では通っていない。
ゼロ対角 (lo=hi) と空メッシュの両方で None を確認していなかった。

**実装 (check.rs)**:
- `min_wall_probe_degenerate_bbox_returns_none`:
  lo=hi=Vec3::ZERO → None、空メッシュ → None を確認。

## 問195 — DSL rounded_box の中間アリティエラーが未確認

**問**: rounded_box は 2 または 4 引数有効、それ以外は "expects 2 or 4 args, got N"。
`function_call_with_zero_arguments_is_rejected` は 0 引数を確認するが、
1 引数・3 引数の中間アリティエラーは未確認。ellipsoid も類似 (1 or 3) だが問195 は
rounded_box の多アリティ分岐を対象とする。

**実装 (dsl.rs)**:
- `rounded_box_wrong_arity_gives_clear_error`:
  `rounded_box(0.5)` → "got 1"、`rounded_box(1,0.8,0.6)` → "got 3"、
  正常 2/4 引数は通ることを確認。

## 反映サマリ v93
| 問 | 実装 |
|----|------|
| 190 | offset 負値許可の契約を数値で明文化 (eval.rs) |
| 191 | cone 負値パラメータの拒否 (eval.rs) |
| 193 | sdf_gradient の単体テスト・勾配方向確認 (check.rs) |
| 194 | min_wall_probe 縮退境界 (diag=0, v=0) の早期リターン (check.rs) |
| 195 | DSL rounded_box 中間アリティ (1, 3 引数) エラー (dsl.rs) |

> 総括: v93 は eval.rs/dsl.rs の演算子パラメータ検証とcheck.rs の内部関数を固定した。
> 問191 は零値と負値で同一ガード (f<=0.0) を使うが零値しかテストしていない非対称性。
> 問190 は逆に「許可されている」ことがテストされていなかった意図的設計の文書化。
> 問193/194 は check.rs の内部実装 (sdf_gradient, 早期リターン) の直接テスト。
> clippy --all-targets -D warnings = 0 warnings。
> テスト数: 262 ユニット + 統合 3 = 265 合計。

## 問196 — 単一三角形の edge_defects が未確認

**問**: edge_defects の c==1 (境界辺) 分岐は多三角形メッシュのみ確認。
三角形 1 枚では辺が各 1 回しか現れず boundary=3, nonmanifold=0 になる最小ケースが未固定。
また is_edge_manifold は boundary==0 かつ nonmanifold==0 のときのみ true であり、
単一三角形は開境界あり (false) という逆向きの動作も未確認だった。

**実装 (mesh.rs)**:
- `edge_defects_single_triangle_has_three_boundary_edges`:
  1 三角形スープ → boundary=3, nonmanifold=0、is_edge_manifold=false を確認。

## 問197 — downsample factor=4 の 16 画素平均が未確認

**問**: downsample_averages_blocks は factor=2 (4 画素平均) のみ。
factor=4 では n=16 で除算し、`(factor*factor) as u32` のオーバーフロー耐性も必要。

**実装 (image.rs)**:
- `downsample_factor_four_averages_sixteen_pixels`:
  4×4 → 1×1 で R チャンネル 0,16,...,240 の平均 120 を確認。

## 問200 — base64 の長い入力が有効アルファベットのみ・決定的かを未確認

**問**: base64_matches_rfc4648 は最大 8 バイト。1000 バイトの入力で
複数の chunks(3) を跨いでも有効 Base64 文字のみ出力され決定的であることが未固定。
中間に '=' が混入しないことも未確認。

**実装 (tools.rs)**:
- `base64_encode_long_input_uses_valid_alphabet_and_is_deterministic`:
  1000 バイト入力で長さ=1336、全文字が Base64 アルファベット、
  '=' は末尾のみ、同一入力で同一出力を確認。

## 問201 — zero-period 軸を持つ Repeat の sampling_box が未確認

**問**: sampling_box_encloses_aabb は repeat_n(splat(2), [1,1,1]) のみ確認。
period.x=0 (x 軸無効) の repeat で sampling_box が反転せず AABB を包含することが未固定。
diag マージンが全軸一律に加わるため zero-period 軸も若干広がる動作を文書化。

**実装 (sdf.rs)**:
- `sampling_box_with_zero_period_repeat_axis_is_not_inverted`:
  period=[0,2,2], count=[0,1,1] で sampling_box が AABB を包含し、
  反転せず (lo<=hi)、x 軸が過剰拡張しないことを確認。

## 問202 — GLB 単一三角形の POSITION accessor min/max が未確認

**問**: glb_json_describes_mesh_accurately は多頂点 sphere のみ確認。
頂点 3 つ (うち outlier 1 個) の場合に accessor の min/max ループ (lines 30-41) が
正しく最小/最大を計算することが未固定。

**実装 (gltf.rs)**:
- `glb_accessor_min_max_correct_for_single_triangle`:
  (0,0,0), (1,0,0), (0,2,3) の三角形で min=[0,0,0], max=[1,2,3] を確認。

## 反映サマリ v94
| 問 | 実装 |
|----|------|
| 196 | 単一三角形 edge_defects: boundary=3, is_edge_manifold=false (mesh.rs) |
| 197 | downsample factor=4 の 16 画素平均 (image.rs) |
| 200 | base64 1000 バイトの有効アルファベット・決定性 (tools.rs) |
| 201 | zero-period Repeat の sampling_box 非反転・AABB 包含 (sdf.rs) |
| 202 | GLB 単一三角形の accessor min/max 正確性 (gltf.rs) |

> 総括: v94 は Image/GLB の I/O パス・mesh の edge 検出・base64 長入力・
> zero-period Repeat の sampling_box を固定した。問196 は is_edge_manifold の
> 「開境界あり → false」という動作を実際に確認し、コメントで boundary==0 の
> 意味を明文化。問201 は diag マージンが全軸共通のため zero-period 軸も
> 広がるという sampling_box の保守的設計を文書化。
> clippy --all-targets -D warnings = 0 warnings。
> テスト数: 267 ユニット + 統合 3 = 270 合計。

## 問203 — STL/GLB/3MF/HTML の横断的バイト同一性 (SPEC §6)

**問**: 各形式は個別に determinism テストを持つが、同一メッシュから全形式が
バイト同一に再エンコードされることを横断的に確認するテストがなかった。
1 形式だけ決定性が壊れる回帰が検出されない。

**実装 (io/mod.rs)**:
- `all_export_formats_re_encode_byte_identically`:
  sphere メッシュから STL/GLB/3MF/HTML すべてが再エンコードでバイト同一、
  mesh.digest() も安定であることを確認。

## 問204 — サンドボックスが全エクスポート形式に一律適用 (SPEC §7.2)

**問**: sandbox_write_path は拡張子非依存だが、テストは .stl のみ。
GLB/3MF/HTML も同じトラバーサル/絶対パス拒否を受けることが未固定。
将来 1 形式だけパスチェックを飛ばす回帰を防げない。

**実装 (tools.rs)**:
- `sandbox_applies_uniformly_across_all_export_formats`:
  stl/glb/3mf/html の各拡張子で正常パス許可・`../`/絶対パス拒否を確認。

## 問205 — get_scene ツールの個別テストが存在しなかった (SPEC §5.1)

**問**: get_scene は eval/undo_script と異なり個別テストがなかった。
script= / sampling_bounds= / undo_available= の報告が未確認。

**実装 (tools.rs)**:
- `get_scene_reports_script_bounds_and_undo_state`:
  初期は undo_available=false + sampling_bounds、run_script 後は
  現在スクリプト (sphere/1.5) + undo_available=true を確認。

## 問206 — sampling_box 正規化を全反転 AABB 変種で確認 (SPEC §3.4)

**問**: sampling_box_is_never_inverted は SmoothIntersection のみ。
hard Intersection / Difference / SmoothDifference も反転 AABB を生みうるが
グループでの正規化保証が未固定だった。

**実装 (sdf.rs)**:
- `sampling_box_normalizes_for_all_inverted_aabb_variants`:
  4 変種すべてで非重複ケースの sampling_box が lo<=hi を保証することを確認。

## 反映サマリ v95 (SPEC 駆動)
| 問 | 実装 | SPEC 節 |
|----|------|---------|
| 203 | 全形式の横断的バイト同一性 (io/mod.rs) | §6 決定性 |
| 204 | 全形式のサンドボックス一律適用 (tools.rs) | §7.2 サンドボックス |
| 205 | get_scene ツールの個別テスト (tools.rs) | §5.1 ツール |
| 206 | 全反転 AABB 変種の sampling_box 正規化 (sdf.rs) | §3.4 中核契約 |

> 総括: v95 は SPEC.md 作成で浮上した「契約は宣言されているがテストで固定されて
> いない」乖離を埋めた。問203 (横断決定性)・問206 (全変種正規化) は個別テストの
> 隙間を埋める回帰防壁。問205 (get_scene) は唯一の未テスト MCP ツールを解消。
> 問204 は形式非依存サンドボックスの契約を全形式で明示。
> なお問207 (3MF unit 宣言) は既存テスト threemf::*line121 で既出のため対象外。
> clippy --all-targets -D warnings = 0 warnings。
> テスト数: 271 ユニット + 統合 3 = 274 合計。

## 問207 — カメラ背後の三角形カリングが未確認

**問**: render は clip 空間 w<=0 (near 面の裏) の三角形を continue で除外するが、
全テストが前方の sphere のみで、背面カリング経路が未検証だった。

**実装 (raster.rs)**:
- `triangle_behind_camera_is_culled`:
  eye=(0,0,1) で z=3 の三角形が全背景になることを確認。

## 問208 — 退化三角形 (面法線ゼロ) の除外が未確認

**問**: render は face_n_len==0.0 (共線=面積ゼロ) の三角形を除外するが、
polygonize は退化三角形を生まないためこの経路は未到達だった。

**実装 (raster.rs)**:
- `degenerate_collinear_triangle_is_not_rendered`:
  共線 3 頂点の手動メッシュが全背景になることを確認。

## 問209 — 透視行列の深度係数の数値が未固定

**問**: 既存テストは最終スクリーン座標のみ確認し、透視行列の z 係数
(proj[10], proj[11], proj[14]) の符号入れ替え等を検出できなかった。

**実装 (raster.rs)**:
- `perspective_matrix_depth_coefficients_are_exact`:
  near=0.01, far=1000 で proj[10]=(far+near)/(near-far)、
  proj[11]=2*far*near/(near-far)、proj[14]=-1、proj[0]=proj[5]=f を確認。

## 問210 — normalize のゼロ長フォールバック閾値が未確認

**問**: normalize は len<1e-15 で (0,0,1) へフォールバックするが、
閾値の上下での挙動が未検証だった (look_at の縮退回避に依存)。

**実装 (raster.rs)**:
- `normalize_zero_length_vector_falls_back_to_z_axis`:
  len=1e-14 は正規化、len=1e-16 と完全ゼロは (0,0,1) フォールバックを確認。

## 問213 — ZIP 中央ディレクトリのオフセット正しさが未確認

**問**: local_header_and_central_directory_crc は CRC 一致のみ確認し、
CD の +42 に格納される LFH オフセットの正しさは未検証だった。
オフセットが誤ると展開ツールがアーカイブを壊れ扱いする。

**実装 (zip.rs)**:
- `central_directory_offsets_point_to_valid_local_headers`:
  可変長名の 3 エントリで各 CD オフセットが有効な LFH 署名を指し、
  LFH のファイル名がエントリ名と一致することを確認。

## 反映サマリ v96
| 問 | 実装 |
|----|------|
| 207 | カメラ背後三角形のカリング (raster.rs) |
| 208 | 退化三角形 (面法線ゼロ) の除外 (raster.rs) |
| 209 | 透視行列の深度係数の数値固定 (raster.rs) |
| 210 | normalize ゼロ長フォールバック閾値 (raster.rs) |
| 213 | ZIP 中央ディレクトリのオフセット正しさ (zip.rs) |

> 総括: v96 はレンダラ (raster.rs) と ZIP パッケージング (zip.rs) の
> 未到達分岐・数値係数・オフセットフィールドを固定した。問207/208 は
> polygonize 経由では到達しない手動メッシュ経路 (背面カリング・退化除外)。
> 問209 は視覚的に似た画像を生む符号エラーを数値で検出する防壁。
> 問213 は CRC だけでなくオフセットの正しさを LFH 署名追跡で確認。
> なお問211 (CLI clamp) と問212 (HTML near/far) は clamp ロジックの自明な
> 再掲または既存テスト範囲のため見送り。
> clippy --all-targets -D warnings = 0 warnings。
> テスト数: 276 ユニット + 統合 3 = 279 合計。

## 問214 — 未知メソッドの通知 (id なし) が None を返すことが未確認

**問**: notification_returns_none は既知メソッド "initialized" のみ確認。
未知メソッドかつ id なし (通知) のとき handle が error でなく None を返す
(JSON-RPC 2.0: 通知には応答しない) 経路が未検証だった。

**実装 (server.rs)**:
- `unknown_method_as_notification_returns_none_not_error`:
  未知メソッド + id なしで handle が None を返すことを確認。
  対照: 未知メソッド + id あり は -32601 エラー。

## 問215 — rotate の負角・360°超が未確認

**問**: angle は req_f64 で範囲制限なし。rotate_operations_via_script は 0°/90° のみ。
負角や 360° 超が to_radians() で正しく周期的に扱われることが未検証だった。

**実装 (eval.rs)**:
- `rotate_accepts_negative_and_over_360_degree_angles`:
  -90°==270°、450°==90° を eval 値で確認。

## 問216 — HTML が非有限座標をサニタイズしていなかった (実バグ修正)

**問**: io/html.rs の mesh_arrays は `{:.4}` で座標を出力するが、3MF の finite_coord と
異なり is_finite() チェックがなかった。NaN/Inf 頂点が "NaN"/"inf" 文字列になり
埋め込み JS の MESH.positions が構文エラーになる。さらに radius も
`Inf.max(1e-3)=Inf` で素通りし "inf" になっていた (positions だけでなく radius も)。

**修正 (html.rs)**: positions/center を 0.0 へ、radius は非有限を 1e-3 へサニタイズ。
3MF の finite_coord と同方針で防御。

**実装 (html.rs)**:
- `html_sanitizes_nonfinite_coordinates`:
  NaN/Inf 頂点メッシュで HTML に "nan"/"inf" リテラルが現れず 0.0000 になることを確認。

## 問219 — dims_mm の精度桁数が未確認

**問**: summary は dims_mm=[{:.3}x...] で 3 桁固定。既存テストは "dims_mm=[2." 接頭辞
のみ確認し、精度桁数 (3 桁) を検証していなかった。

**実装 (check.rs)**:
- `summary_dims_mm_uses_exactly_three_decimal_places`:
  3 成分すべてが厳密に 3 桁小数であることをパースして確認。

## 反映サマリ v97
| 問 | 実装 |
|----|------|
| 214 | 未知メソッド通知の None 応答 (server.rs) |
| 215 | rotate 負角・360°超の周期性 (eval.rs) |
| 216 | **実バグ修正**: HTML 非有限座標/radius サニタイズ (html.rs) |
| 219 | dims_mm 精度 3 桁の固定 (check.rs) |

> 総括: v97 はソクラテス問答が再び**実バグ**を発見した回。問216 の HTML
> 非有限座標は 3MF が finite_coord で防いでいる一方 HTML は無防備で、NaN/Inf 頂点が
> 埋め込み JS を構文破壊していた。さらに radius は Inf.max(1e-3)=Inf で素通りする
> ため positions だけでなく radius のサニタイズも必要だった (agent の提案より広い修正)。
> 問214/215/219 は JSON-RPC ディスパッチ・角度周期性・出力精度の境界固定。
> なお問217/218/220 (opt_f64/overhang境界/req_child) は低価値または FP 不安定のため見送り。
> clippy --all-targets -D warnings = 0 warnings。
> テスト数: 280 ユニット + 統合 3 = 283 合計。

## 問221 — GLB の bufferView/accessor 参照配線が未確認

**問**: glb_json_describes_mesh_accurately は count のみ確認。
accessor[0]→bufferView 0 (POSITION)、accessor[1]→bufferView 1 (INDEX) の
参照配線と bufferView が厳密に 2 個であることが未検証だった。配線が入れ替わると
GLB ビューアが頂点/索引を取り違える。

**実装 (gltf.rs)**:
- `glb_accessor_bufferview_indices_are_correctly_wired`:
  bufferViews=2、accessor[0].bufferView=0、accessor[1].bufferView=1、
  各 bufferView が buffer 0 を参照することを確認。

## 問222 — GLB の byteLength 内部整合が未確認

**問**: bufferView の byteLength/byteOffset・buffers[0].byteLength・BIN チャンク
ヘッダの整合 (pos_len + idx_len == total) が未検証だった。

**実装 (gltf.rs)**:
- `glb_buffer_byte_lengths_are_internally_consistent`:
  view0.byteOffset=0、view1.byteOffset=view0.byteLength (連続配置)、
  byteLength 合計=buffer 宣言長、BIN ヘッダ長は宣言長以上かつパディング<4 を確認。

## 問224 — screenshot の全 7 ビューの動作が未確認

**問**: help は front|back|right|left|top|bottom|iso を有効ビューと宣言するが、
各ビューが実際に screenshot で成功することを確認するテストがなかった。

**実装 (tools.rs)**:
- `screenshot_accepts_all_seven_documented_views_and_rejects_unknown`:
  7 ビューすべてが画像生成に成功、未知ビューが明示エラー (問71) になることを確認。

## 問225 — 3MF XML の階層構造・ドキュメント順序が未確認

**問**: model_xml_counts_match_mesh は要素数のみ確認し、階層構造と
ドキュメント順序 (resources が build に先行) を検証していなかった。
3MF スキーマ: model→resources→object→mesh→vertices/triangles→build→item。

**実装 (threemf.rs)**:
- `model_xml_nesting_and_document_order_is_3mf_conformant`:
  必須 6 要素の存在、名前空間宣言、10 要素のドキュメント順序を確認。

## 反映サマリ v98
| 問 | 実装 |
|----|------|
| 221 | GLB bufferView/accessor 参照配線 (gltf.rs) |
| 222 | GLB byteLength 内部整合 (gltf.rs) |
| 224 | screenshot 全 7 ビューの動作 + 未知拒否 (tools.rs) |
| 225 | 3MF XML 階層・ドキュメント順序 (threemf.rs) |

> 総括: v98 は出力フォーマット (GLB/3MF) の構造的不変条件と MCP screenshot の
> ビュー網羅を固定した。問221/222 は GLB の参照配線・サイズ整合という
> 「個別の数値は正しいが相互参照が壊れると検出されない」構造ギャップ。
> 問225 は要素数だけでなくドキュメント順序を固定 (3MF スキーマ適合)。
> 問223 (グリッド網羅性) は assertion が緩く、問226 (CLI exit code) は
> main() が exit() を呼ぶため単体テスト困難なため見送り。
> clippy --all-targets -D warnings = 0 warnings。
> テスト数: 284 ユニット + 統合 3 = 287 合計。

## 問226 — 負ゼロ (-0.0) のシリアライズ挙動が未確認

**問**: Display は `n.fract()==0.0 && n.abs()<1e15` で整数出力するため -0.0 は "0" になり
符号ビットが失われる。-0.0 == +0.0 なので幾何・算術に影響はなく出力も決定的だが、
この良性挙動を固定するテストがなかった。

**実装 (json.rs)**:
- `negative_zero_serializes_as_zero_and_is_numerically_equivalent`:
  -0.0 → "0"、再パースで +0.0、出力が決定的であることを確認 (意図された良性挙動)。

## 問227 — 科学記法入力のビット保存往復が未確認

**問**: パーサは科学記法 (1.5e-3) を受理するが Display は十進 (0.0015) で出力する。
文字列形式は変わるが f64 ビット列は保存される (Rust の Display↔parse 往復保証) ことが
未検証だった。AI が科学記法を送っても数値が壊れないことを固定。

**実装 (json.rs)**:
- `scientific_notation_input_roundtrips_bit_identically_via_decimal`:
  5 種の科学記法入力で parse→to_string→parse がビット同一であることを確認。

## 問228 — smooth_union の中点ブレンド補正値が未確認

**問**: smooth_union の多項式 `mix(db,da,h) - k*h*(1-h)` は da==db のとき h=0.5、
補正項 = k*0.25 (最大ブレンド)。既存テストは収束 (k→0) とブレンドゾーン外 (|da-db|>k)
のみで、中点での厳密な補正値を確認していなかった。

**実装 (sdf.rs)**:
- `smooth_union_of_shape_with_itself_subtracts_quarter_k_everywhere`:
  同一形状同士の smooth_union が全点で d - k*0.25 になることを 4 点で確認。

## 問229/230 — cone の apex 厳密ゼロと底面ディスク被覆が未確認

**問**: cone_surface_and_sign は apex を `.abs()<EPS` で確認し底面は内部 1 点のみ。
apex が厳密に 0.0 (== 0.0、符号付きゼロ含む)、底面のエッジ・内部・外側が正しいことを
固定する。なお apex は実装上 -0.0 になりうるため `== 0.0` で比較 (bit-exact は不可)。

**実装 (sdf.rs)**:
- `cone_apex_is_exactly_zero_and_base_disk_is_complete`:
  apex == 0.0、底面エッジ (1,0,-2)≈0、内部 (0.5,0,-2)≈0、外側 (1.1,0,-2)>0 を確認。

## 反映サマリ v99
| 問 | 実装 |
|----|------|
| 226 | 負ゼロのシリアライズ良性挙動 (json.rs) |
| 227 | 科学記法のビット保存往復 (json.rs) |
| 228 | smooth_union 中点ブレンド補正 d-k*0.25 (sdf.rs) |
| 229/230 | cone apex 厳密ゼロ + 底面ディスク被覆 (sdf.rs) |

> 総括: v99 は数値・代数的正しさを固定した。問229 で agent は apex を bit-exact +0.0 と
> 仮定したが、実装をトレースすると s=-0.0 → signum(-0.0)=-1 で result が -0.0 になりうる
> ため、bit-exact (to_bits 比較) は失敗する。`== 0.0` (符号付きゼロを包含) を採用し罠を回避。
> 問228 は同一形状同士の smooth_union が全点 d-k*0.25 になる性質で多項式を厳密検証。
> 問231 (2-inside 巻き順) は既存 watertight テストの再掲のため見送り。
> clippy --all-targets -D warnings = 0 warnings。
> テスト数: 288 ユニット + 統合 3 = 291 合計。

## 問231 — エクスポート段が eval_set パイプラインで未検証

**問**: 既存の eval_set 統合テストは script→mesh→validate までで、ユーザ/AI が実際に行う
最終段 (export = フォーマット直列化) を横断的に通していなかった。各エンコーダ
(STL/GLB/3MF/HTML) の単体テストは単一の sphere メッシュのみで、CSG・smooth・repeat・
mirror・rotate・torus 等の多様な実モデルでのエンコーダ退行は検出できなかった。

**実装 (tests/eval_set.rs)**:
- `eval_set_models_export_to_all_formats_with_valid_structure`:
  13 課題 × 4 形式を出力し、各形式の構造的妥当性 (STL ヘッダ/三角形数、
  GLB マジック/JSON parse/accessor count、3MF ZIP 署名、HTML doctype/プレースホルダ
  全置換/非有限リテラルなし) と決定性 (2 回エンコードでバイト同一) を確認。

## 反映サマリ v100
| 問 | 実装 |
|----|------|
| 231 | エクスポート段の全形式×全モデル統合テスト (tests/eval_set.rs) |

> 総括: v100 はマイクロギャップ探索の収穫逓減を踏まえ、より高価値な
> エンドツーエンド統合テストへ転換した。CLI (main.rs) は exit() を直接呼ぶ薄い
> グルーで、中核ロジックは全てライブラリ側で検証済みのため単体テストは低価値と判断。
> 代わりに「ユーザが実際に行う完全なパイプライン (script→mesh→**export**)」を
> 13 の多様なモデルで通すことで、単一 sphere メッシュの単体テストでは見えない
> エンコーダ退行 (多様体・大頂点数・特殊形状) を一点で検知する防壁を追加した。
> clippy --all-targets -D warnings = 0 warnings。
> テスト数: 288 ユニット + 統合 4 = 292 合計。

## 問232 — 敵対的モデルの水密性ストレステスト (バグ探索の実証的補強)

**背景**: v101 でバグ探索エージェントを起動し、aabb の回転境界・MT の勾配符号・
eval のパラメータ強制・check のオーバーハング数学を行単位でトレース調査した結果、
**実バグなし (clean bill of health)** という結論を得た。これをコード読解だけで
信用せず、**実証的に**検証する。

**問**: Marching Tetrahedra の限界に近い敵対的入力 (薄壁・深いネスト CSG・極端
アスペクト比・ほぼ接する smooth_union の鞍点・細い穴・変換合成) で、水密性
(edge-manifold) と向き一貫性 (正の符号付き体積) が保たれるか? 解像度依存の
退行 (低解像度での非多様体化) はないか?

**実証結果**: 6 モデル × 解像度 {24, 48, 64} の全 18 ケースで manifold=true・vol>0。
薄壁シェル (0.08mm) は体積が解像度依存 (0.63→0.91→0.93 と収束) だが水密性は不変。
これは SPEC §9 の既知の限界「ステップより薄いフィーチャは体積過少だが水密」を裏付ける。

**実装 (tests/stress_probe.rs)**:
- `adversarial_models_stay_watertight_across_resolutions`:
  6 敵対的モデル × 3 解像度で edge-manifold かつ vol>0 を確認。
- `under_resolved_thin_shell_stays_watertight_even_when_volume_is_underestimated`:
  res=20 (step > 壁厚) の薄壁シェルでも水密性が保たれることを明示的に固定。

## 反映サマリ v101
| 問 | 実装 |
|----|------|
| 232 | 敵対的モデルの水密性ストレステスト (tests/stress_probe.rs) |

> 総括: v101 はバグ探索の結論をコード読解で終わらせず実証で裏付けた回。
> エージェントの「実バグなし」を 18 ケースの敵対的抽出で empirical に確認し、
> 同時に SPEC §9 の既知限界 (薄壁の体積過少 vs 水密性維持) を回帰テスト化した。
> 「読んで安全」より「走らせて安全」を一段強い保証として残す。
> clippy --all-targets -D warnings = 0 warnings。
> テスト数: 288 ユニット + 統合 6 = 294 合計。

## 問233 — README とCIが未整備 (Plan.md §114/§117 の未達成果物)

**背景**: v101 までで実装・テスト・SPEC は成熟したが、プロジェクト全体を俯瞰すると
Plan.md が明示的に列挙する成果物のうち **README.md (§114)** と
**.github/workflows/ci.yml (§117: Lint→Test→audit→SBOM)** が欠落していた。
AI-First ツールでありながら、ユーザ/AI エージェントが利用法を把握する入口
(README) も、手動実行している品質ゲートを自動化する CI もなかった。

**実装**:
- `README.md`: 設計の柱 (外部送信ゼロ/単一バイナリ/決定的/ヘッドレス)、
  CLI コマンド表、MCP ツール表、DSL 例 (JSON/テキスト両形式)、アーキテクチャ図、
  決定性の範囲、SPEC へのリンクを記載。selftest の出力値は実測 (f(origin)=-1) で固定。
- `.github/workflows/ci.yml`:
  - `quality` ジョブ (ubuntu/macos/windows マトリクス): clippy -D warnings →
    cargo test --all-targets → release build。各テスト内の 2 回実行バイト一致
    アサーションがプラットフォームごとの自己再現性 (問5) を担保。
  - `no-external-deps` ジョブ: `cargo tree` の依存が kado 自身 1 件のみであることを
    検証 (ADR-003/問4 のサプライチェーンゲート・SBOM の最小形)。外部 crate 混入で CI 失敗。

**設計判断**: `cargo fmt --check` は CI に**含めない**。コードベースは意図的な
手整形 (テスト表の整列・DSL JSON 文字列の可読性優先) が約 30 ファイルに渡り、
fmt デフォルトと乖離する。fmt 強制は大規模 diff と可読性低下を招くため、
実質的な品質ゲート (clippy/test/build/no-deps) のみを CI 化した。

## 反映サマリ v102
| 問 | 実装 |
|----|------|
| 233 | README.md + CI ワークフロー (Plan §114/§117 の成果物) |

> 総括: v102 はテストのマイクロギャップ探索から離れ、プロジェクトの俯瞰的な
> 弱点 (入口ドキュメントと品質ゲート自動化の欠落) を埋めた。Plan.md が成果物として
> 列挙しながら未達だった README/CI を整備し、手動の品質確認 (clippy/test) を
> マトリクス CI で自動化。ADR-003 の「外部依存ゼロ」を cargo tree で構造的に強制する
> サプライチェーンゲートも追加した。クロスプラットフォーム自己再現性も CI で運動する。
> テスト数: 288 ユニット + 統合 6 = 294 合計 (変更なし)。

> 注 (v102 追補): `.github/workflows/ci.yml` は GitHub App の `workflows` 権限制約により
> 自動 push できなかった。ファイルは作業ツリーに生成済みで内容は検証済み
> (clippy/test/release build/no-deps すべてローカル通過)。リポジトリ管理者が
> 手動コミットすることで有効化される。README.md は正常に反映済み。

## 問234 — Plan.md §114 の必須成果物 (LICENSE/SECURITY/CONTRIBUTING) が未整備

**問**: Cargo.toml は `license = "MIT"` を宣言するが **LICENSE ファイルが存在しない**
(コンプライアンス上の実欠陥: GitHub がライセンス認識せず、crates.io 公開時に警告)。
さらに Plan.md §114 が列挙する SECURITY.md / CONTRIBUTING.md も欠落していた。
セキュリティを中核価値とするツールでありながらセキュリティモデルの公開文書がなく、
std-only・決定性・手整形といった非自明な制約を伝えるコントリビュータ向け文書もなかった。

**実装**:
- `LICENSE`: MIT (Cargo.toml の宣言と一致、2026 Kado contributors)。
- `SECURITY.md`: セキュリティモデル (外部送信ゼロ・書込サンドボックス・DSL サンドボックス・
  リソース上限・数値の安全な縮退)、想定脅威/非脅威、脆弱性報告手順、対応方針
  (修正は必ず回帰テストで固定)。SPEC §7 の不変条件を公開文書化。
- `CONTRIBUTING.md`: 鉄則 (外部crate禁止・決定性・バグ修正のテスト固定・無効入力の
  事前拒否・リソース上限)、品質ゲート、`cargo fmt` を一括適用しない理由 (意図的手整形)、
  アーキテクチャ文書への導線。

## 反映サマリ v103
| 問 | 実装 |
|----|------|
| 234 | LICENSE / SECURITY.md / CONTRIBUTING.md (Plan §114 成果物) |

> 総括: v103 は Plan.md §114 が列挙する必須成果物のうち未整備だった 3 点を補完した。
> LICENSE 欠落は Cargo.toml の license 宣言と矛盾する実コンプライアンス欠陥であり優先度高。
> SECURITY.md は本プロダクトの中核価値 (信頼できない AI 入力を扱う設計) を公開文書化し、
> CONTRIBUTING.md は std-only・決定性・手整形という「知らないと踏む」制約を明文化した。
> これでプロジェクトの基盤文書 (README/SPEC/LICENSE/SECURITY/CONTRIBUTING/ADR/CI) が
> 一通り揃った。テスト数: 288 ユニット + 統合 6 = 294 合計 (変更なし)。

## 問235 — 新機能: 平面カット (cut) — FDM 印刷の平坦底面・断面

**ソクラテス問答**:
- *製品の核心の仕事は?* AI が部品を生成し、**FDM 3D 印刷**向けに検証・出力する。
- *ほぼ全ての印刷部品が必要とするのに Kado が直接表現できないものは?* **平坦な底面**。
  FDM 部品はベッド密着とサポート回避のため平らな底が要る。Kado には平面でのカットがなかった。
- *difference + 巨大 cuboid で代用できないか?* 部分的には可能だが footgun: AI が cuboid を
  「十分大きく」配置せねばならず、形状が超えると無言で誤カット。半空間は**構成上無限で厳密**。
- *SDF/std/決定性に適合?* 半空間は最も単純な SDF (`dot(p,n)-offset`)、カット=交差=`max`。
  ただし単独平面は無限 AABB (sampling_box の footgun)。
- *頑健な設計は?* `cut` を**子への単項修飾** `Cut(child,n,offset)` にする。AABB=子の AABB
  (カットは材料を削るのみ→保守的かつ有界)。無限 AABP footgun なし、FDM 用途に直結、合成可能。

**実装** (全スタック貫通):
- `core/sdf.rs`: `Cut(Box<Sdf>, Vec3, f64)` variant + eval (`max(child, dot(p,n)-offset)`) +
  aabb (`child.aabb()`) + builder `cut(normal, offset)` (法線を単位化し offset も同スケール補正)。
- `script/eval.rs`: `{"op":"cut","nx","ny","nz","offset"?,"shape"}`。ゼロ法線・非有限 offset を拒否。
- `script/dsl.rs`: `cut(nx,ny,nz,shape)` / `cut(nx,ny,nz,offset,shape)` の 2 アリティ。
- `mcp/tools.rs`: help にカット例 (平坦底面・断面) を追記。
- `docs/SPEC.md`: §3.3 変換表に cut を追加。

**テスト**:
- sdf: `cut_removes_half_space_the_normal_points_into` / `cut_normal_is_normalized_so_distance_field_is_metric` /
  `cut_aabb_is_bounded_by_child_not_infinite`。
- eval: `cut_via_script_flattens_base_and_rejects_zero_normal` (offset 省略=0、ゼロ法線/欠落拒否)。
- dsl: cut の 4/5 引数 DSL↔JSON 等価性 (`assert_same`)。
- 統合: eval_set に「flat-based dome (cut for FDM)」を追加 → 水密・健全・全形式出力・決定的を確認。
- E2E 実測: 球を z=0 でカット → 水密な半球 (bbox z:0.000→1.000・PASS・0 errors)。

## 反映サマリ v104
| 問 | 実装 |
|----|------|
| 235 | 新機能 cut (平面カット/半空間交差) — core/eval/dsl/tools/spec/tests 全スタック |

> 総括: v104 はソクラテス問答で**新機能**を導出・実装した回。「FDM 部品は平坦な底面が要る」
> という製品ドメインの本質的要求から出発し、difference+cuboid 代用の脆さ (無言誤カット) を
> 退けて、子への単項修飾という頑健な設計 (有界 AABB・厳密半空間) に到達した。
> 全スタック (enum→eval→aabb→builder→JSON→DSL→help→SPEC) を貫通し、各層 + 統合 + E2E で固定。
> テスト数: 292 ユニット + 統合 4 = 296 合計。

## 問236 — 改良: flatten (cut の最頻用ケースの安全な別名)

**長所短所分析 (cut 出荷後)**:
- *長所*: 汎用平面カットを厳密 SDF・有界 AABB で実現、全形式で水密。
- *短所 #1 (最重要)*: cut の**法線方向が誤りやすい**。最頻用の「底面を平らに」は
  `nz=-1` (法線が下を向く) が正解で、直感的な `nz=+1` だと逆 (底を残し上を削る) になる。
  最も多用される操作が最もミスしやすい — AI が無言で誤った形状を作る footgun。

**改良**: 意図明示型 op `flatten` を追加。`flatten(at)` は z=at (既定 0) で底を切り
z>=at を残す。名前が動作を語るため法線方向の取り違えが起きない。
新 core variant は作らず `cut((0,0,-1), -at)` に lower する (表面積最小)。
汎用の `cut` は任意平面・断面用に残す (安全な別名 + 汎用プリミティブの 2 層構成)。

**実装**:
- `script/eval.rs`: `{"op":"flatten","at"?,"shape"}` → `child.cut((0,0,-1), -at)`。非有限 at 拒否。
- `script/dsl.rs`: `flatten(shape)` [at=0] / `flatten(at, shape)`。
- `mcp/tools.rs`: help に flatten (推奨ショートカット) を追記。
- `docs/SPEC.md` §3.3 / `README.md`: flatten を追加し flatten を推奨記法として提示。

**テスト**:
- eval: `flatten_keeps_above_plane_and_equals_explicit_cut` — z>=at を残す、at 省略=0、
  at=0.3 で底上げ、**flatten(0.3)==cut((0,0,-1),-0.3) をビット一致**で確認、非有限 at 拒否。
- dsl: flatten の 1/2 引数 DSL↔JSON 等価性。
- E2E: JSON `flatten` と DSL `flatten(sphere(1))` が同一 digest (b7c8515ae2ad1088)・水密。

## 反映サマリ v105
| 問 | 実装 |
|----|------|
| 236 | 改良 flatten — cut 法線方向 footgun の解消 (意図明示型別名) |

> 総括: v105 は出荷した cut 機能の長所短所を分析し、最重要の短所 (法線方向の
> 取り違え footgun) を改良した。新 variant を増やさず既存 cut への lower で実現し、
> 「安全な別名 (flatten) + 汎用プリミティブ (cut)」の 2 層構成に。flatten==cut の
> ビット一致テストで等価性を保証。テスト数: 293 ユニット + 統合 4 = 297 合計。

## 問237 — 改良: オーバーハング検査の平坦底面・ベッド支持面の偽陽性を解消

**長所短所分析 (flatten 出荷後)**:
- *短所 #2*: flatten/cut で作った**平坦底面が OVERHANG として誤検知**される。下向き面を
  一律にオーバーハング扱いしていたため、印刷可能な平坦底面 (法線 -Z・最下層) が
  「90° overhang」警告を出す偽陽性。AI が非問題を「直そう」として誤誘導される。

**改良 (物理的に正しい支持判定)**: オーバーハング = 下向き面で**直下に支えがない**もの。
2 段で支持面を除外:
- (a) **ベッド接地面**: 重心が最下層 (min_proj から造形高さ 1% 以内) → ベッドが支える。
  平坦底面は厳密に min_proj。
- (b) **直下に材料**: SDF 併用時、重心の真下・ベッド直上 (min_proj+bed_eps の高さ) を
  標本化し形状内部 (eval<0) なら、ベッドから材料が立ち上がり支えている → 除外。
  平坦底面と壁が交わる鋭い凸エッジ (rim) の MT 遷移三角形を正しく支持済みと扱う。
  固定ステップ降下だと低い面でベッド下へ突き抜けるため、ベッド直上の固定高さで標本化。

**実装 (check.rs)**: オーバーハングループに min_proj/max_proj 算出 + (a)(b) 除外を追加。
SDF は validate_with_field の既存 `sdf` 引数を利用 (MCP validate / CLI check は Some を渡す)。

**テスト**:
- `flat_printable_base_is_not_flagged_as_overhang`: 平坦底面ドームは OVERHANG なし、
  対照の素の球 (真の底面オーバーハング・下に材料なし) は依然検出 (偽陰性なし)。
- 既存の overhang テスト (球の南半球検出・build_dir 反映・閾値0スキップ) は全て不変で通過。
- E2E: flatten ドームの check が OVERHANG 警告ゼロ (以前は 90° 警告)。

**残る既知事項**: 鋭い凸 rim は THIN_WALL 探針で薄肉判定されうるが、ナイフエッジ rim は
実際に印刷上の細フィーチャであり妥当な警告のため本変更の対象外 (別懸案・偽陰性リスク高)。

## 反映サマリ v106
| 問 | 実装 |
|----|------|
| 237 | オーバーハング検査のベッド支持面 (平坦底面・rim) 偽陽性解消 (check.rs) |

> 総括: v106 は flatten 出荷後の短所 #2 (平坦底面の OVERHANG 偽陽性) を物理的に正しい
> 支持判定で解消した。ヒューリスティックな高さバンドではなく「直下に材料/ベッドの支えが
> あるか」を SDF で問う頑健な判定にし、偽陰性 (真のオーバーハング見逃し) を作らないことを
> 対照テストで保証。テスト数: 294 ユニット + 統合 6 = 300 合計。

## 問238 — 新視点: 物理挙動 (重心と転倒安定性)

**ソクラテス問答 (新視点の導出)**:
- *Kado はどんなレンズで見られてきたか?* 幾何的正しさ (水密性)・製造可能性 (DFM: 薄肉/
  オーバーハング)・決定性・セキュリティ。すべて**「作れるか?」**を問う。
- *Kado が一度も問わないことは?* **「作った後、物理的な物体として成立するか?」** —
  質量・重心の位置・自立するか。幾何的に完璧で製造可能でも**転倒**しうる。
- *最も普遍的に計算できる物理量は?* **重心 (COM)**。そこから質量 (体積×密度) と、
  決定的に重要な**転倒安定性** (COM の鉛直線がベース足元に入るか) が導ける。
- *制約に適合?* COM は表面メッシュから発散定理で計算可 (signed_volume と同系統)、
  決定的・std のみ。
- *なぜ今コヒーレントか?* 直前に追加した cut/flatten が**平坦な底面を作る** →
  「その底面で自立するか?」が自然な次の問い。安定性は製造可能性検査の物理挙動版。

**実装**:
- `extract/mesh.rs`: `center_of_mass() -> Option<Vec3>`。四面体分割の体積重み付き重心。
  決定的 (固定順序 f64 加算)。退化/空は None。
- `verify/check.rs`: 安定性検査 (#7)。COM の横方向投影がベース接地面 (最下層頂点) の
  フットプリント bbox から外れたら `UNSTABLE` 警告。bbox は真の支持多角形 (凸包) の
  外接近似 → 「外」は転倒の十分条件 = 偽陽性なしの保守的警告。
- `ALL_ISSUE_CODES` に UNSTABLE 追加 + MCP help / validate スキーマに記載 (文書ドリフト防止)。

**テスト**:
- mesh: COM が原点中心球で原点、平行移動球で移動先、上半球で z=3R/8=0.375 (理論値)。
- check: `top_heavy_offset_part_is_flagged_unstable_but_centered_dome_is_not` —
  偏重心のタワーは UNSTABLE、対称ドームは安定。
- E2E: トップヘビー部品の check が UNSTABLE 警告、対称ドームは警告ゼロ。

## 反映サマリ v107
| 問 | 実装 |
|----|------|
| 238 | 新視点「物理挙動」— 重心 (center_of_mass) + 転倒安定性 (UNSTABLE) |

> 総括: v107 はソクラテス問答で**新しい視点 (レンズ)** を導入した回。これまで「作れるか
> (製造可能性)」しか問わなかったところに「作った後に物理的に成立するか (自立するか)」という
> 直交する評価軸を追加した。cut/flatten が作る平坦底面と自然に接続し、重心という普遍的物理量
> から転倒安定性を決定的に判定する。bbox 近似により偽陽性のない保守的警告とした。
> テスト数: 298 ユニット + 統合 6 = 304 合計。

## 問239 — Report に重心 (COM) を公開する

**ソクラテス問答**:
- *UNSTABLE 判定の根拠数値は AI エージェントに見えるか?* — 見えない。check.rs は
  COM を計算して転倒判定を行うが、その座標は Report の内部変数として消費されるだけで
  JSON にも summary にも現れない。
- *どんな不利益があるか?* — AI/利用者は「なぜ UNSTABLE か」を再現できない。重心が
  (1.5, 0, 0.8) にあって足元 bbox が (−0.15, −0.15)-(0.15, 0.15) だと知れれば、
  「足元を広げる or 頭を移動する」という自己修正指示が具体的に書ける。今は「重心が外れた」
  という事実のみ。
- *解消方法は?* — `Report::center_of_mass: Option<Vec3>` として公開する。
  empty/退化メッシュは None、通常の中実メッシュは Some([x,y,z])。
  to_json に `"center_of_mass"` キーを追加し、summary に `com=[x.xxx,y.xxx,z.xxx]` を付加。

**実装**:
- `verify/check.rs`: `Report` に `center_of_mass: Option<Vec3>` フィールド追加。
- `validate_with_field`: COM を UNSTABLE ブロック外で計算して `center_of_mass` に格納。
  UNSTABLE チェックは `center_of_mass` を参照 (compute-once)。
- `Report::summary()`: `Some` なら末尾に ` com=[x.xxx,y.xxx,z.xxx]` を付加。
- `Report::to_json()`: `"center_of_mass"` キーを追加 (Some→[x,y,z]、None→null)。
- 空メッシュの早期リターンと、手動 Report 構築テストに `center_of_mass: None` を追加。

**テスト**:
- `report_exposes_center_of_mass_in_json_and_summary`: 中実球の COM が JSON に
  3要素配列で含まれ to_json が往復可能。summary に "com=" が含まれる。
  空メッシュは JSON で null。
- 既存 UNSTABLE テスト (`top_heavy_offset_part_is_flagged_unstable_but_centered_dome_is_not`)
  は全て不変で通過。

## 問240 — mesh-only OVERHANG と field-aware OVERHANG の非対称性を文書化

**ソクラテス問答**:
- *`validate()` と `validate_with_field(_, Some(&sdf), _)` の OVERHANG 結果は常に同じか?*
  — 同じではない。問237 で追加した支持判定 (b) は `sdf=Some(...)` のときのみ実行される。
  `validate()` = `validate_with_field(_, None, _)` は SDF がないため rim 三角形の
  「直下に材料あり」チェックができない。
- *これはバグか設計上の制限か?* — 設計上の制限。SDF なしにメッシュだけから「直下に材料が
  あるか」を判定するのは原理上困難 (メッシュ交差判定が必要で高コスト)。
  ただしテストで文書化しないと利用者が予期せぬ偽陽性で混乱する。
- *文書化の方法?* — `mesh_only_validate_flags_dome_rim_overhang_that_field_aware_suppresses`
  テストで「field-aware は抑制、mesh-only は警告あり」を固定する。
  API 境界とその制限がテストで明示されることが仕様書の代わりになる。

**実装**: `verify/check.rs` に上記テスト追加のみ (check.rs のロジック変更なし)。

## 反映サマリ v108
| 問 | 実装 |
|----|------|
| 239 | `Report::center_of_mass: Option<Vec3>` — COM を JSON/summary に公開 |
| 240 | mesh-only vs field-aware OVERHANG 非対称性テスト (`validate` の制限を文書化) |

> 総括: v108 は「情報の非対称」を解消した回。UNSTABLE の判定根拠 (COM 座標) が
> Report 外へ漏れなかった問題を修正し、AI エージェントが自己修正ループで「重心がどこに
> あるか」を直接参照できるようにした。あわせて validate/validate_with_field の OVERHANG
> 挙動差を明示テストで固定し、利用者が mesh-only モードの制限を事前に理解できるようにした。
> テスト数: 302 ユニット + 統合 6 = 308 合計。

## 問241 — 新視点: 製造プロセス中の安定性 (高アスペクト比と揺れリスク)

**ソクラテス問答**:
- *Kado が捉えていない印刷失敗の軸は?*
  v107 で「印刷後の転倒 (UNSTABLE)」を追加した。しかし失敗モードはもう1つある —
  **印刷中の失敗**。物理挙動の時間軸が違う。
- *FDM 印刷中の最大失敗原因は?* → 高くて細い形状がノズルに当たって揺れ、
  最終的に層間剥離または転倒する。ノズルは XY 面内を高速移動するため、
  背の高い形状への慣性衝撃が繰り返される。
- *これは定量化できるか?* → **高さ / 横幅比 (アスペクト比)**。
  FDM コミュニティの経験則: 比 > 8 でリスク増大。
  `max_proj - min_proj` (高さ) と、bd に垂直な頂点 bbox の最長辺 (横幅) の比。
- *UNSTABLE との違いは?* → UNSTABLE = **印刷後に静置した物体が転倒する (物理挙動)**。
  HIGH_ASPECT_RATIO = **印刷プロセス中にノズルとの動的相互作用で失敗する (製造挙動)**。
  両者は直交する: 広い底面で UNSTABLE でない部品でも、ドーム上に細いアンテナが立つと
  HIGH_ASPECT_RATIO になりうる。
- *build_dir との整合性は?* → 同じ形状でも横向きに印刷すれば比が逆転する。
  OVERHANG/UNSTABLE と同様に build_dir 依存にすることで一貫性を保つ。

**実装**:
- `verify/check.rs`: `ALL_ISSUE_CODES` に `"HIGH_ASPECT_RATIO"` 追加。
- `validate_with_field` に検査 #8 追加 (UNSTABLE の後)。
  各頂点を bd 方向に投影して height (max-min) と横方向 bbox を計算し、
  `height / lateral_max > 8.0` で `HIGH_ASPECT_RATIO` 警告を出す。
  `lat(p) = p - bd*(p·bd)` で bd に垂直な横方向成分を抽出。
  build_dir=ZERO (閾値未満) ならスキップ (他の bd 依存チェックと同一の守衛)。
- `mcp/tools.rs`: validate 説明と KADOSCENE_HELP カタログに HIGH_ASPECT_RATIO を追加
  (問103 の文書ドリフト防止ガード `issue_codes_are_fully_documented` が強制)。

**テスト**:
- `tall_thin_cylinder_is_flagged_high_aspect_ratio`:
  cylinder(r=0.2, hh=2.5) → height=5, lateral=0.4 → 比率 12.5 > 8 → 警告あり。
- `wide_flat_part_does_not_flag_aspect_ratio`:
  cuboid(2,2,0.1) → height=0.2, lateral=4 → 比率 0.05 < 8 → 警告なし。
- `aspect_ratio_check_respects_build_direction`:
  同一形状 (cuboid(2,0.2,0.2)) を +X ビルド (比率 10 > 8 → 警告) と
  +Z ビルド (比率 0.1 < 8 → 警告なし) で対比。build_dir 依存を証明。

## 反映サマリ v109
| 問 | 実装 |
|----|------|
| 241 | 新視点「製造プロセス安定性」— HIGH_ASPECT_RATIO 検査 (高さ/横幅比 > 8) |

> 総括: v109 はソクラテス問答で 3 つ目の「時間軸」視点を確立した回。
> これまで Kado の DFM 検査は「できあがった形状の印刷可能性」(薄肉/オーバーハング)
> と「印刷後の物理的自立」(UNSTABLE) を見ていた。v109 は**印刷プロセス中の動的安定性**
> という第 3 の時間断面を追加した。UNSTABLE が「静力学」なら HIGH_ASPECT_RATIO は
> 「動力学 (慣性衝撃)」に対応し、製造プロセスの全時間軸をカバーする:
>   1. 印刷中 (HIGH_ASPECT_RATIO)
>   2. 取り出し直後 (OVERHANG → 変形なし)
>   3. 使用時 (UNSTABLE → 転倒なし)
> テスト数: 303 ユニット + 6 統合 = 309 合計。

## 問242 — 新視点: 検証問題の空間注釈 (KadoError.location)

**ソクラテス問答**:
- *OVERHANG が警告を出す。AI はどこを修正すればいいか?*
  → 「わからない。75° の面があることは知っているが、形状全体のどこにあるかは
  目視検査しないとわからない。」
- *THIN_WALL プローブは最小肉厚を '探して' いるはずだが、その座標は?*
  → 「計算しているが、戻り値は `Option<f64>` — スカラーのみ。座標を捨てている。」
- *UNSTABLE は COM を Report に公開したが、issue オブジェクト自体は?*
  → 「issue は cause 文字列に埋め込んでいない。AI が issue リストをループする際に
  report フィールドを別途参照する必要がある。統一されていない。」
- *全 issue が location を持てれば AI の自己修正ループがどう変わるか?*
  → `for issue in issues { zoom_to(issue.location?); apply_fix(issue.code) }` と書ける。
  issue.code だけで判断でき、code→fix→location が1オブジェクトに揃う。
- *抽象化のレベルは?* → `KadoError` struct に `location: Option<Vec3>` を追加する。
  空間的意味を持たない issue (EMPTY_MESH, NON_MANIFOLD 等) は None。
  OVERHANG: 最悪三角形の重心。THIN_WALL: プローブが最小を検出した表面頂点。
  UNSTABLE: 重心 (COM)。これらはすべてすでに計算されており、捨てていただけ。

**実装**:
- `verify/check.rs`: `KadoError` に `location: Option<Vec3>` フィールドを追加。
  `error()`/`warn()` は `location: None` で初期化し、`with_location(Vec3) -> Self`
  (Builder パターン) で付与できるようにする。
- `min_wall_probe`: 戻り値を `Option<f64>` → `Option<(f64, Vec3)>` に変更。
  `min_t` が更新されるたびに起点頂点 `p` も記録する。
- THIN_WALL ブロック: プローブが平均より薄い値を検出した場合、
  プローブ頂点を `THIN_WALL.location` に設定。
- OVERHANG ループ: `worst_centroid` を追跡し、更新時に `nd < worst` と同時に
  `worst_centroid = centroid`。`worst < -max_cos` で `OVERHANG.with_location(worst_centroid)`。
- UNSTABLE ブロック: `UNSTABLE.with_location(com)` — COM が問題の空間的起点。
- `Report::to_json()`: issue オブジェクトに `("location", e.location.map_or(null, vec3))` を追加。
  全 issue に location キーが存在し、None なら null。

**テスト (4本)**:
- `overhang_issue_carries_spatial_location`: 球の OVERHANG.location が下半球 (z < -0.5) にある。
  JSON にも location 配列が含まれること。
- `thin_wall_issue_carries_probe_location_in_fin`: フィン形状の THIN_WALL.location が
  フィン領域 (|y| < 0.15) にある。
- `unstable_issue_carries_com_as_location`: トップヘビー部品の UNSTABLE.location が
  重心と一致し x > 0.5 (頭側)。report.center_of_mass と完全一致。
- `issues_without_spatial_context_have_null_location_in_json`: OPEN_MESH など空間的でない
  issue の JSON location が null。全 issue に location キーが存在することも確認。
- `probe_catches_local_thin_fin_that_mean_misses` (既存): プローブ位置もフィン内を確認するよう強化。

## 反映サマリ v110
| 問 | 実装 |
|----|------|
| 242 | 新視点「空間注釈」— `KadoError.location: Option<Vec3>` + OVERHANG/THIN_WALL/UNSTABLE が座標を運ぶ |

> 総括: v110 は「情報の次元」を追加した回。スカラー (角度・肉厚・体積) だった
> issue 情報に空間次元 ([x,y,z]) を追加し、AI エージェントが「どこを直すか」を
> issue オブジェクト1つから直接知れるようにした。
> OVERHANG は最悪三角形の重心、THIN_WALL はプローブ最小点頂点、UNSTABLE は COM を
> それぞれ location に設定。min_wall_probe の戻り型変更 (f64 → (f64, Vec3)) は
> 既存テストを4箇所更新するだけで完結した。
> テスト数: 307 ユニット + 6 統合 = 313 合計。

## 問243 — スキーマドリフト修正: ツール定義と実装の乖離

**ソクラテス問答**:
- *v110 で `KadoError.location` を追加したが、MCP ツール定義の validate 説明は?*
  → 「古いまま。`location` が JSON に出力されているのにツール定義には記載がない。
  AI エージェントが validate を呼ぶとき、スキーマを見ても location の存在を知れない。」
- *`Report` に `center_of_mass` を v108 で追加したが、ツール説明の JSON スキーマは?*
  → 「center_of_mass も記載なし。volume_reliable も欠落していた。」
- *スキーマドリフトの根本問題は?*
  → 実装 (check.rs の to_json) と文書 (tools.rs のツール説明) が別々に管理され、
  一方を変更した際に他方を更新するリマインダーがない。
  `issue_codes_are_fully_documented` ガードはコードが列挙されているかを見るが、
  JSON フィールドが listed されているかは見ない。
- *補完すべき防壁は?*
  → `to_json_carries_all_schema_fields` テスト: SPEC §5 / ツール定義に記載した
  全フィールドを実際の `to_json()` 出力が持つことを pin する。
  スキーマ変更時に先に実装 → テスト失敗 → ツール定義を更新するという
  「テストが文書を駆動する」フローが自然に成立する。
- *具体的に何が欠落していたか?*
  → ツール description の validate JSON スキーマ:
  `volume_reliable` フィールドが未記載。
  `center_of_mass` が未記載。
  issue オブジェクトの `location` が未記載。
  KADOSCENE_HELP カタログの issue codes セクション:
  各 issue の location の意味 (どの座標か) が未記載。
  SPEC.md §5.1 の validate エントリも同様に古かった。

**実装**:
- `mcp/tools.rs`: validate ツール description の JSON スキーマ文字列に
  `volume_reliable`, `center_of_mass:[x,y,z]|null`,
  issue オブジェクトの `location:[x,y,z]|null` を追記。
  KADOSCENE_HELP カタログ (issue codes セクション) に location の意味テーブルを追加:
  OVERHANG → 最悪三角形重心, THIN_WALL → プローブ最小点頂点,
  UNSTABLE → COM, others → null。
- `verify/check.rs`: `to_json_carries_all_schema_fields` テストを追加。
  SPEC §5 / ツール定義に記載した全 Report フィールド
  (ok/triangles/manifold/volume/volume_reliable/bbox/dims_mm/center_of_mass/digest/issues)
  と全 issue フィールド (severity/code/cause/hints/location) が
  実際の JSON 出力に存在することを assert。
- `docs/SPEC.md`: §5.1 validate エントリを最新の JSON スキーマに更新。
  §5.3 (新設) issue `location` フィールドの意味表を追加。
  テスト数を 308 ユニット + 6 統合 = 314 合計に更新。

**テスト**:
- `to_json_carries_all_schema_fields`:
  OVERHANG が出る球メッシュを polygonize し、validate を実行。
  Report JSON の全フィールドと issue JSON の全フィールドをループ assert。

## 反映サマリ v111
| 問 | 実装 |
|----|------|
| 243 | スキーマドリフト修正 — ツール定義・SPEC に location/center_of_mass/volume_reliable を追記 + pin テスト |

> 総括: v111 は「情報の整合性」を修正した回。v108-v110 で追加した機能 (center_of_mass,
> location, volume_reliable) が実装済みだったにもかかわらず、ツール定義と SPEC が
> 古いスキーマを示し続けていた。AI エージェントがツール定義を読んで validate を呼ぶと
> 実際の出力に含まれるフィールドを「存在しない」と誤認するリスクがあった。
> `to_json_carries_all_schema_fields` テストにより今後のスキーマ変更時にこのドリフトを
> 自動検出できる体制を確立した。
> テスト数: 308 ユニット + 6 統合 = 314 合計。
