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
