#!/usr/bin/env python3
"""Kado 出力の外部相互運用性チェック (docs/SPEC.md §10 / 問289・問324)。

§10 は「書き出した STL/3MF/GLB が **Kado 以外の標準ツール**で開けること」を
リリース前に確認すると定め、検証内容まで具体的に書いていた。しかし手順は
**散文としてしか存在せず**、リポジトリに実行可能なものは無かった (問324)。
実行の痕跡も再現手段も無い検査は、検査ではなく願望である。

要点は Kado の実装を**一切参照せず独立にデコードする**こと。Kado のバグを
Kado のコードで見逃さないため、Python 標準ライブラリだけで復元する。

    python3 scripts/interop-check.py [--keep]

`scripts/check.sh` には組み込まない。§10 が「Python 依存のため CI には
含めない」と既に判断しており、その判断を蒸し返さない (問295 と同じ扱い)。
"""

import json
import os
import struct
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET
import zipfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
KADO = os.path.join(ROOT, "target", "release", "kado")
# 穴あき・平坦底面つきの実用形状。単純な球より退化しにくく、法線も多様になる。
SCENE = "difference(flatten(0, sphere(10.0)), cylinder(1.6, 25.0))"

failures = []


def check(name, fn):
    try:
        detail = fn()
        print(f"  \033[32mPASS\033[0m {name}" + (f" — {detail}" if detail else ""))
    except Exception as e:  # noqa: BLE001 - 検査スクリプトなので全て報告する
        print(f"  \033[31mFAIL\033[0m {name} — {e}")
        failures.append(name)


def verify_stl(path):
    """binary STL を 84 + n*50 レイアウトから独立に復元する。"""
    data = open(path, "rb").read()
    if len(data) < 84:
        raise AssertionError(f"too short for a binary STL header: {len(data)} bytes")
    (declared,) = struct.unpack_from("<I", data, 80)
    expected = 84 + declared * 50
    if len(data) != expected:
        raise AssertionError(
            f"file length {len(data)} != 84 + {declared}*50 = {expected}"
        )
    # 全レコードの属性バイト数は 0 (色拡張を使っていないこと)。
    for i in range(declared):
        (attr,) = struct.unpack_from("<H", data, 84 + i * 50 + 48)
        if attr != 0:
            raise AssertionError(f"triangle {i} has non-zero attribute bytes: {attr}")
    # 法線と頂点がすべて有限であること (NaN/Inf は下流ツールを壊す)。
    for i in range(declared):
        vals = struct.unpack_from("<12f", data, 84 + i * 50)
        if any(v != v or v in (float("inf"), float("-inf")) for v in vals):
            raise AssertionError(f"triangle {i} contains a non-finite float")
    return f"{declared} triangles, {len(data)} bytes, all attributes 0"


def verify_3mf(path):
    """3MF を ZIP として独立に検証し、モデル XML を解析する。"""
    with zipfile.ZipFile(path) as z:
        # testzip() は全エントリの CRC-32 を独自に再計算する。
        bad = z.testzip()
        if bad is not None:
            raise AssertionError(f"CRC-32 mismatch in entry: {bad}")
        names = z.namelist()
        model_name = "3D/3dmodel.model"
        if model_name not in names:
            raise AssertionError(f"{model_name} missing; entries: {names}")
        if "[Content_Types].xml" not in names:
            raise AssertionError("[Content_Types].xml missing (not a valid OPC package)")
        root = ET.fromstring(z.read(model_name))
        unit = root.attrib.get("unit")
        if unit != "millimeter":
            raise AssertionError(f'unit must be "millimeter" (問62), got {unit!r}')
        ns = "{http://schemas.microsoft.com/3dmanufacturing/core/2015/02}"
        verts = root.findall(f".//{ns}vertex")
        tris = root.findall(f".//{ns}triangle")
        if not verts or not tris:
            raise AssertionError(f"empty mesh: {len(verts)} vertices, {len(tris)} triangles")
        # 三角形の頂点参照が範囲内であること (下流が配列外参照でクラッシュしない)。
        for t in tris:
            for k in ("v1", "v2", "v3"):
                idx = int(t.attrib[k])
                if not 0 <= idx < len(verts):
                    raise AssertionError(f"triangle references vertex {idx} of {len(verts)}")
        return f'{len(verts)} vertices, {len(tris)} triangles, unit="millimeter", CRC ok'


def verify_glb(path):
    """GLB を glTF 2.0 のチャンク構造から独立に検証する。"""
    data = open(path, "rb").read()
    magic, version, total = struct.unpack_from("<4sII", data, 0)
    if magic != b"glTF":
        raise AssertionError(f"bad magic: {magic!r}")
    if version != 2:
        raise AssertionError(f"glTF version must be 2, got {version}")
    if total != len(data):
        raise AssertionError(f"declared length {total} != actual {len(data)}")
    chunk_len, chunk_type = struct.unpack_from("<I4s", data, 12)
    if chunk_type != b"JSON":
        raise AssertionError(f"first chunk must be JSON, got {chunk_type!r}")
    gltf = json.loads(data[20 : 20 + chunk_len].decode("utf-8"))
    if gltf.get("asset", {}).get("version") != "2.0":
        raise AssertionError(f'asset.version must be "2.0", got {gltf.get("asset")}')
    # 問290: NORMAL アクセサは単位長でなければならない。BIN チャンクから直接読む。
    bin_off = 20 + chunk_len
    bin_len, bin_type = struct.unpack_from("<I4s", data, bin_off)
    if bin_type != b"BIN\x00":
        raise AssertionError(f"second chunk must be BIN, got {bin_type!r}")
    bin_data = data[bin_off + 8 : bin_off + 8 + bin_len]
    prim = gltf["meshes"][0]["primitives"][0]
    acc = gltf["accessors"][prim["attributes"]["NORMAL"]]
    view = gltf["bufferViews"][acc["bufferView"]]
    base = view.get("byteOffset", 0) + acc.get("byteOffset", 0)
    worst = 0.0
    for i in range(acc["count"]):
        x, y, z = struct.unpack_from("<3f", bin_data, base + i * 12)
        worst = max(worst, abs((x * x + y * y + z * z) ** 0.5 - 1.0))
    if worst > 1e-3:
        raise AssertionError(f"NORMAL accessor not unit length: worst deviation {worst}")
    return f'{acc["count"]} normals, worst unit-length deviation {worst:.2e}'


def verify_html(path):
    """HTML ビューアが自己完結であること (C1「外部送信ゼロ」の出力側の担保・問324)。

    §10 は HTML に触れていないが、**同じ理由で確認すべき**ものである。
    書き出した HTML が外部 URL を参照していれば、それを開いた時点で
    ネットワークへ出てしまう——ソースにソケットが無いこと (問316) だけでは、
    生成物経由の流出を防げない。
    """
    text = open(path, encoding="utf-8").read()
    for marker in ("src=\"http", "src='http", "href=\"http", "href='http", "//cdn", "@import"):
        if marker in text:
            raise AssertionError(f"viewer references an external resource: {marker!r}")
    if "<canvas" not in text and "webgl" not in text.lower():
        raise AssertionError("viewer does not look like a WebGL page")
    return f"{len(text)} chars, no external references"


VERIFIERS = [
    ("out.stl", "STL", verify_stl),
    ("out.3mf", "3MF", verify_3mf),
    ("out.glb", "GLB", verify_glb),
    ("out.html", "HTML", verify_html),
]


def main():
    if not os.path.exists(KADO):
        print(f"release binary not found: {KADO}\nrun: cargo build --release", file=sys.stderr)
        return 2

    keep = "--keep" in sys.argv
    work = tempfile.mkdtemp(prefix="kado-interop-")
    scene = os.path.join(work, "scene.txt")
    with open(scene, "w", encoding="utf-8") as f:
        f.write(SCENE)

    print(f"scene: {SCENE}\nworkdir: {work}\n")
    for fname, label, fn in VERIFIERS:
        out = os.path.join(work, fname)
        r = subprocess.run([KADO, "export", scene, out], capture_output=True, text=True)
        if r.returncode != 0:
            print(f"  \033[31mFAIL\033[0m {label} — export failed: {r.stderr.strip()}")
            failures.append(label)
            continue
        check(label, lambda fn=fn, out=out: fn(out))

    print()
    if failures:
        print(f"\033[31m外部相互運用性チェック失敗: {', '.join(failures)}\033[0m")
        return 1
    print("\033[32m全形式が Kado 以外の実装で復元できた。\033[0m")
    if not keep:
        for fname, _, _ in VERIFIERS:
            try:
                os.remove(os.path.join(work, fname))
            except OSError:
                pass
        os.remove(scene)
        os.rmdir(work)
    return 0


if __name__ == "__main__":
    sys.exit(main())
