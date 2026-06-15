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
> 摘出し、AI エージェントの作業安全性と DFM 評価の正確性を高めた。テスト数 132→137 + 統合 3。
