#!/usr/bin/env python3
"""Trains the learned ranker (spec §10.13) into engine/predict/learned.bin, in one command:

    python3 engine/tools/train_learned.py            # plan + data + train (the whole pipeline)
    python3 engine/tools/train_learned.py --step train   # retrain on the data already generated

Steps:
  plan   samples text from corpora/derived/*/ (each source weighted by its meta.json `weight`) and
         corpora/ (chat), deterministically (--seed), into corpora/learned/plan.tsv. Sentences, word
         lists and mixed Latin + Hangul tokens; each line typed in one of the planner's styles.
  data   types the plan through the engine (`cargo run --release --example learn_data`), with the
         current predict/ranker.bin and word lists, into corpora/learned/data/. Every word of the
         evaluation texts is left out: engine/tests/corpora-en-ko/ (with extended/),
         engine/tests/data/samsung-business.txt and corpora/eval/**/*.txt (the DART audit corpus).
  train  softmax over each word's readings (listwise logistic loss), L2, L-BFGS, fixed seed; reports
         top-1 on training and validation words (1 in 20 words, by a hash of the word) against the
         rules, and writes engine/predict/learned.bin.

Rerun the whole pipeline after ranker.bin or common-*.txt change: the features read them.
Needs NumPy and SciPy: without them it makes a virtual environment in corpora/.venv (gitignored)
and installs them there, then reruns itself with it. Nothing is installed system-wide.
"""

import argparse
import glob
import json
import math
import os
import random
import struct
import subprocess
import sys
import time

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
ENGINE = os.path.join(ROOT, "engine")
CORPORA = os.path.join(ROOT, "corpora")
WORK = os.path.join(CORPORA, "learned")
VENV = os.path.join(CORPORA, ".venv")
OUT = os.path.join(ENGINE, "predict", "learned.bin")

# Words typed, by kind of source (before identical words merge).
BUDGET = {
    "ko sentences": 900_000,
    "en sentences": 120_000,
    "chat": 120_000,
    "mixed tokens": 260_000,
    "ko words": 150_000,
    "en words": 100_000,
    "terms": 20_000,
}
# How often each style types a line: the cheapest way, every jamo, a mix of the valid ways.
STYLES = (("o", 0.4), ("j", 0.2), ("m1", 0.14), ("m2", 0.13), ("m3", 0.13))
MAX_WORDS = 40


def ensure_numpy():
    try:
        import numpy  # noqa: F401
        import scipy  # noqa: F401
        return
    except ImportError:
        pass
    python = os.path.join(VENV, "bin", "python")
    if os.path.abspath(sys.executable) == os.path.abspath(python):
        sys.exit("numpy/scipy missing in corpora/.venv; run: corpora/.venv/bin/pip install numpy scipy")
    if not os.path.exists(python):
        print(f"making {VENV} with numpy and scipy (gitignored, nothing system-wide)", flush=True)
        subprocess.check_call([sys.executable, "-m", "venv", VENV])
        subprocess.check_call([python, "-m", "pip", "install", "-q", "numpy", "scipy"])
    os.execv(python, [python] + sys.argv)


# ---------------------------------------------------------------------------------------------
# plan

def count_lines(path):
    with open(path, "rb") as f:
        return sum(1 for _ in f)


def sample_lines(path, k, rng, transform=None):
    """k lines picked uniformly (by line number, deterministic), read in one pass."""
    if k <= 0:
        return []
    n = count_lines(path)
    if n == 0:
        return []
    chosen = set(rng.sample(range(n), min(k, n)))
    out = []
    with open(path, encoding="utf-8", errors="replace") as f:
        for i, line in enumerate(f):
            if i in chosen:
                line = line.rstrip("\n")
                if transform:
                    line = transform(line)
                if line:
                    out.append(line)
    return out


def clean_sentence(line):
    line = " ".join(line.replace("\t", " ").split())
    words = line.split(" ")
    if not line or len(words) > MAX_WORDS:
        return None
    return line


def eval_files():
    files = glob.glob(os.path.join(ENGINE, "tests", "corpora-en-ko", "*.txt"))
    files += glob.glob(os.path.join(ENGINE, "tests", "corpora-en-ko", "extended", "*.txt"))
    samsung = os.path.join(ENGINE, "tests", "data", "samsung-business.txt")
    if os.path.exists(samsung):
        files.append(samsung)
    files += glob.glob(os.path.join(CORPORA, "eval", "**", "*.txt"), recursive=True)
    return sorted(files)


def plan(args):
    rng = random.Random(args.seed)
    sources = []
    for meta_path in sorted(glob.glob(os.path.join(CORPORA, "derived", "*", "meta.json"))):
        d = os.path.dirname(meta_path)
        with open(meta_path, encoding="utf-8") as f:
            weight = float(json.load(f).get("weight", 1.0))
        sources.append((os.path.basename(d), d, weight))
    eval_lines = set()
    for path in eval_files():
        with open(path, encoding="utf-8", errors="replace") as f:
            eval_lines.update(" ".join(l.split()) for l in f if l.strip())

    # (kind, source, weight, path, how to read a line)
    pools = []
    for name, d, w in sources:
        for file, kind, read in (
            ("ko.txt", "ko sentences", clean_sentence),
            ("en.txt", "en sentences", clean_sentence),
            ("mixed.txt", "mixed tokens", lambda l: l.split("\t")[0].strip() or None),
            ("ko_words.tsv", "ko words", lambda l: l.split("\t")[0].strip() or None),
            ("en_words.tsv", "en words", lambda l: l.split("\t")[0].strip() or None),
            ("terms.tsv", "terms", lambda l: " ".join(t.strip() for t in l.split("\t") if t.strip()) or None),
        ):
            path = os.path.join(d, file)
            if os.path.exists(path):
                pools.append((kind, name, w, path, read))
    chat = []
    chatbot = os.path.join(CORPORA, "ChatbotData.csv")
    if os.path.exists(chatbot):
        chat.append(("chat", "chatbot", 1.0, chatbot, lambda l: clean_sentence(" ".join(l.rsplit(",", 1)[0].split(",")))))
    nsmc = os.path.join(CORPORA, "nsmc-ratings.txt")
    if os.path.exists(nsmc):
        chat.append(("chat", "nsmc", 1.0, nsmc, lambda l: clean_sentence(l.split("\t")[1]) if l.count("\t") == 2 else None))
    pools += chat

    words_per_line = {"ko sentences": 10, "en sentences": 18, "chat": 9, "terms": 1.5}
    lines = []
    for kind, budget in BUDGET.items():
        members = [p for p in pools if p[0] == kind]
        if not members:
            continue
        sizes = {p[3]: count_lines(p[3]) for p in members}
        # Each source's share: its weight times the square root of its size.
        shares = [p[2] * math.sqrt(sizes[p[3]]) for p in members]
        total = sum(shares)
        per_line = words_per_line.get(kind, 1)
        for (k, name, w, path, read), share in zip(members, shares):
            want = int(budget * share / total / per_line) + 1
            got = sample_lines(path, want, rng, read)
            got = [g for g in got if g not in eval_lines]
            for text in got:
                r = rng.random()
                acc = 0
                for style, p in STYLES:
                    acc += p
                    if r < acc:
                        break
                lines.append((name, style, w, text))
            print(f"  {kind:13} {name:28} weight {w:4} {len(got):7} lines", flush=True)
    # Blocks of one source and style type together.
    lines.sort(key=lambda l: (l[0], l[1]))
    os.makedirs(WORK, exist_ok=True)
    path = os.path.join(WORK, "plan.tsv")
    with open(path, "w", encoding="utf-8") as f:
        for name, style, w, text in lines:
            f.write(f"{style}\t{w}\t{name}\t{text}\n")
    print(f"plan: {len(lines)} lines → {path}")


# ---------------------------------------------------------------------------------------------
# data

def data(args):
    cargo = os.path.expanduser("~/.cargo/bin/cargo")
    if not os.path.exists(cargo):
        cargo = "cargo"
    cmd = [cargo, "run", "--release", "--example", "learn_data", "--",
           "--plan", os.path.join(WORK, "plan.tsv"), "--out", os.path.join(WORK, "data")]
    for f in eval_files():
        cmd += ["--exclude", f]
    if args.threads:
        cmd += ["--threads", str(args.threads)]
    print(" ".join(cmd[:6]), "…", flush=True)
    t = time.time()
    subprocess.check_call(cmd, cwd=ENGINE)
    print(f"data: {time.time() - t:.0f} s")


# ---------------------------------------------------------------------------------------------
# train

def load(dir_):
    import numpy as np
    meta = json.load(open(os.path.join(dir_, "meta.json")))
    a = lambda name, t: np.fromfile(os.path.join(dir_, name), dtype=t)
    return meta, dict(
        row_ptr=a("row_ptr.u32", "<u4").astype(np.int64),
        row_weight=a("row_weight.f32", "<f4").astype(np.float64),
        row_valid=a("row_valid.u8", "u1").astype(bool),
        row_group=a("row_group.u8", "u1"),
        row_style=a("row_style.u8", "u1"),
        cand_ptr=a("cand_ptr.u64", "<u8").view(np.int64),
        cand_label=a("cand_label.u8", "u1"),
        cand_rank=a("cand_rank.u8", "u1"),
        cand_legacy=a("cand_legacy.f32", "<f4").astype(np.float64),
        feat_idx=a("feat_idx.u32", "<u4"),
        feat_val=a("feat_val.f32", "<f4"),
    )


def train(args):
    import numpy as np
    import scipy.optimize
    import scipy.sparse

    meta, d = load(os.path.join(WORK, "data"))
    dim = 1 << meta["bits"]
    n_rows = len(d["row_weight"])
    n_cands = len(d["cand_label"])
    print(f"data: {n_rows} rows, {n_cands} candidates, {len(d['feat_idx'])} features (table 2^{meta['bits']})", flush=True)
    # float32 and no copies: the matrix is the biggest thing in memory. Duplicate indices just add.
    X = scipy.sparse.csr_matrix((d.pop("feat_val"), d.pop("feat_idx").view(np.int32), d.pop("cand_ptr")), shape=(n_cands, dim), copy=False)
    XT = X.T  # CSC, no copy
    row_ptr = d["row_ptr"]
    sizes = np.diff(row_ptr)
    starts = row_ptr[:-1]
    y = d["cand_label"].astype(np.float64)
    valid = d["row_valid"]
    group = d["row_group"]
    # Each word weighs its count times its source's weight; the mean training row weighs 1.
    weight = d["row_weight"] / d["row_weight"][~valid].mean()
    train_weight = np.where(valid, 0.0, weight)
    train_weight_c = np.repeat(train_weight, sizes)

    free = np.zeros(dim, dtype=bool)
    free[meta["unregularized"]] = True
    reg = np.where(free, 0.0, args.l2)

    # The model corrects the rules: its score starts from theirs (clipped, in bits → nats).
    clip = meta.get("legacy_clip", 20.0)
    z0 = np.clip(d["cand_legacy"], -clip, clip) * math.log(2)
    evals = [0]

    # The correction is bounded: reach · tanh(u / reach), so the model nudges the rules' order but
    # can't overturn their strong verdicts or the user's weights (spec §10.13).
    reach = args.reach * math.log(2)

    def raw(theta):
        return (X @ theta.astype(np.float32)).astype(np.float64)

    def scores(theta):
        return z0 + reach * np.tanh(raw(theta) / reach)

    def objective(theta):
        t = np.tanh(raw(theta) / reach)
        z = z0 + reach * t
        m = np.maximum.reduceat(z, starts)
        e = np.exp(z - np.repeat(m, sizes))
        s = np.add.reduceat(e, starts)
        lse = m + np.log(s)
        pos = np.add.reduceat(z * y, starts)
        loss = np.sum(train_weight * (lse - pos)) + 0.5 * np.sum(reg * theta * theta)
        p = e / np.repeat(s, sizes)
        r = (train_weight_c * (p - y) * (1 - t * t)).astype(np.float32)
        g = (XT @ r).astype(np.float64) + reg * theta
        evals[0] += 1
        if evals[0] % 25 == 0:
            print(f"  {evals[0]} evaluations, loss {loss:.1f}", flush=True)
        return loss, g

    def right_rows(z):
        # The top reading of each row, the first on ties (candidates come in the rules' order).
        m = np.maximum.reduceat(z, starts)
        is_max = z >= np.repeat(m, sizes) - 1e-9
        first = np.minimum.reduceat(np.where(is_max, np.arange(len(z)), len(z)), starts)
        return y[first] == 1

    rules = right_rows(-d["cand_rank"].astype(np.float64))

    t = time.time()
    theta0 = np.zeros(dim)  # the rules
    res = scipy.optimize.minimize(objective, theta0, jac=True, method="L-BFGS-B",
                                  options={"maxiter": args.iterations, "maxcor": 20})
    theta = res.x
    print(f"trained: {res.nit} iterations, loss {res.fun:.1f}, {time.time() - t:.0f} s ({res.message})", flush=True)

    learned = right_rows(scores(theta))
    groups = {"k": "Korean", "e": "Latin", "m": "mixed", "o": "other"}
    for name, sel_rows in (("train", ~valid), ("valid", valid)):
        w = weight[sel_rows]
        line = [f"{name}: top-1 learned {100 * np.average(learned[sel_rows], weights=w):.2f}% vs rules {100 * np.average(rules[sel_rows], weights=w):.2f}% "
                f"(weighted; by rows {100 * learned[sel_rows].mean():.2f}% vs {100 * rules[sel_rows].mean():.2f}%, {sel_rows.sum()} rows)"]
        for code, label in groups.items():
            sel = sel_rows & (group == ord(code))
            if sel.any():
                line.append(f"  {label:8} {100 * np.average(learned[sel], weights=weight[sel]):6.2f}% vs {100 * np.average(rules[sel], weights=weight[sel]):6.2f}%  ({sel.sum()} rows)")
        print("\n".join(line), flush=True)

    # Scores in bits: log2 of the odds, like the rules' (θ is in nats).
    bits_weights = (theta / math.log(2)).astype(np.float32)
    big = np.max(np.abs(bits_weights))
    scale = float(big / 32767) if big > 0 else 1.0
    q = np.round(bits_weights / scale).astype("<i2")
    header = b"JKLR" + struct.pack("<IIIff", 2, meta["features"], meta["bits"], scale, args.reach)
    out = args.out or OUT
    with open(out, "wb") as f:
        f.write(header)
        f.write(q.tobytes())
    print(f"wrote {out}: {len(header) + q.nbytes} bytes, {int(np.count_nonzero(q))} nonzero weights, largest {big:.2f} bits")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--step", choices=["all", "plan", "data", "train"], default="all")
    ap.add_argument("--seed", type=int, default=20261027)
    ap.add_argument("--scale", type=float, default=1.0, help="multiply every word budget")
    ap.add_argument("--l2", type=float, default=300.0, help="L2 on every weight: how close the model stays to the rules")
    ap.add_argument("--reach", type=float, default=2.0, help="how far (bits) the model may move a reading from the rules' score")
    ap.add_argument("--iterations", type=int, default=400)
    ap.add_argument("--threads", type=int, default=0)
    ap.add_argument("--out", help="where to write the model (default engine/predict/learned.bin)")
    args = ap.parse_args()
    if args.scale != 1.0:
        for k in BUDGET:
            BUDGET[k] = int(BUDGET[k] * args.scale)
    if args.step in ("all", "train"):
        ensure_numpy()
    if args.step in ("all", "plan"):
        plan(args)
    if args.step in ("all", "data"):
        data(args)
    if args.step in ("all", "train"):
        train(args)


if __name__ == "__main__":
    main()
