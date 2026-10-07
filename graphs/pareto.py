#!/usr/bin/env python3
"""Plot speed-vs-size Pareto frontiers from a benchmark_results JSON file.

Usage: python3 graphs/pareto.py [results.json] [output_dir]

Defaults to the newest file in benchmark_results/ and writes two PNGs per
dataset into graphs/: one with every crate, and one restricted to formats with
evolvable schemas (fields can be added/removed without breaking old readers).
Requires matplotlib.
"""

import json
import sys
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.ticker import FuncFormatter, LogLocator


ROOT = Path(__file__).resolve().parent.parent

SURFACE = "#fcfcfb"
INK = "#0b0b0b"
INK_2 = "#52514e"
MUTED = "#a3a29c"
GRID = "#e6e5e0"
FRONTIER = "#2a78d6"
HIGHLIGHT = "#eb6834"
HIGHLIGHT_CRATE = "oxidef"

OPS = [("serialize", "Serialize time"), ("deserialize", "Deserialize time")]
SIZES = [("size", "Serialized size"), ("zstd", "Size after zstd")]

# Formats whose schemas can evolve compatibly, as configured in this benchmark.
# Value is the set of allowed variants (None = the primary result).
# Excluded: non-self-describing positional formats (bitcode, bincode-likes,
# borsh, postcard, rkyv, ...), savefile (explicit version numbers, not
# forward-compatible) and messagepack (structs are encoded as field-less
# arrays here).
EVOLVABLE = {
    "bilrost": None, "buffa": None, "prost": None, "protobuf": None, "protobuf4": None,
    "capnp": None, "flatbuffers": None, "flexbuffers": None,
    "cbor4ii": None, "ciborium": None, "minicbor": None,
    "flexon": None, "serde_json": None, "simd-json": None, "ron": None,
    "oxidef": {"extensible"}, "oxidef_old": {"extensible"},
}


def value(bench, key):
    """Return (primary, variants) for a bench entry, unwrapping the unit."""
    entry = bench.get(key)
    if entry is None:
        return None, {}
    (inner,) = entry.values()
    return inner["primary"], inner["variants"]


def points(dataset, op, size_key, only=None):
    out = []
    for crate, feature in dataset["features"].items():
        if only is not None and crate not in only:
            continue
        allowed = only[crate] if only is not None else None
        benches = feature["benches"]
        op_primary, op_variants = value(benches, op)
        size_primary, size_variants = value(benches, size_key)
        candidates = []
        if op_primary is not None:
            candidates.append((None, op_primary))
        candidates += list(op_variants.items())
        for variant, t in candidates:
            if allowed is not None and variant not in allowed:
                continue
            size = size_variants.get(variant, size_primary) if variant else size_primary
            if t is None or size is None:
                continue
            label = f"{crate} ({variant})" if variant else crate
            out.append({"crate": crate, "label": label, "time": t, "size": size})
    return out


def pareto(pts):
    """Points not dominated on (time, size), sorted by time."""
    frontier = []
    best = float("inf")
    for p in sorted(pts, key=lambda p: (p["time"], p["size"])):
        if p["size"] < best:
            frontier.append(p)
            best = p["size"]
    return frontier


def fmt_time(ns, _pos=None):
    for unit, scale in (("s", 1e9), ("ms", 1e6), ("µs", 1e3)):
        if ns >= scale:
            return f"{ns / scale:g} {unit}"
    return f"{ns:g} ns"


def fmt_bytes(b, _pos=None):
    for unit, scale in (("GB", 1e9), ("MB", 1e6), ("kB", 1e3)):
        if b >= scale:
            return f"{b / scale:g} {unit}"
    return f"{b:g} B"


def plot_panel(ax, pts, op_title, size_title):
    frontier = pareto(pts)
    on_frontier = {id(p) for p in frontier}

    ax.set_facecolor(SURFACE)
    ax.set_xscale("log")
    ax.set_yscale("log")
    ax.grid(True, which="major", color=GRID, linewidth=0.8)
    ax.set_axisbelow(True)
    for side in ("top", "right"):
        ax.spines[side].set_visible(False)
    for side in ("left", "bottom"):
        ax.spines[side].set_color(MUTED)
    ax.tick_params(colors=INK_2, labelsize=8, which="both")
    ax.yaxis.set_major_locator(LogLocator(subs=(1, 2, 5)))
    ax.xaxis.set_major_formatter(FuncFormatter(fmt_time))
    ax.yaxis.set_major_formatter(FuncFormatter(fmt_bytes))
    ax.xaxis.set_minor_formatter(FuncFormatter(lambda *_: ""))
    ax.yaxis.set_minor_formatter(FuncFormatter(lambda *_: ""))
    ax.set_xlabel(f"{op_title} (log, lower is better)", color=INK_2, fontsize=9)
    ax.set_ylabel(f"{size_title} (log, lower is better)", color=INK_2, fontsize=9)
    ax.set_title(f"{op_title} vs {size_title.lower()}", color=INK, fontsize=11, loc="left")

    # Frontier as a staircase: everything up-and-right of it is dominated.
    xs = [p["time"] for p in frontier]
    ys = [p["size"] for p in frontier]
    xmax = max(p["time"] for p in pts) * 2
    ymax = max(p["size"] for p in pts) * 1.5
    step_x = [xs[0], xs[0]]
    step_y = [ymax, ys[0]]
    for i in range(1, len(xs)):
        step_x += [xs[i], xs[i]]
        step_y += [ys[i - 1], ys[i]]
    step_x.append(xmax)
    step_y.append(ys[-1])
    ax.plot(step_x, step_y, color=FRONTIER, linewidth=2, zorder=2, drawstyle="default")

    labels = []
    for p in pts:
        is_front = id(p) in on_frontier
        is_hl = p["crate"] == HIGHLIGHT_CRATE
        color = HIGHLIGHT if is_hl else FRONTIER if is_front else MUTED
        ax.scatter(
            p["time"], p["size"], s=42 if (is_front or is_hl) else 26,
            color=color, edgecolor=SURFACE, linewidth=1.5, zorder=4 if is_hl else 3,
        )
        labels.append((not (is_front or is_hl), p))

    xmin = min(p["time"] for p in pts) / 1.6
    ymin = min(p["size"] for p in pts) / 1.15
    ax.set_xlim(xmin, xmax)
    ax.set_ylim(ymin, ymax)

    place_labels(ax, pts, labels)
    return frontier


# Candidate label offsets (points) around a marker, in order of preference.
OFFSETS = [(5, 3, "left", "bottom"), (5, -3, "left", "top"),
           (-5, 3, "right", "bottom"), (-5, -3, "right", "top"),
           (0, 6, "center", "bottom"), (0, -6, "center", "top"),
           (8, 0, "left", "center"), (-8, 0, "right", "center")]


def place_labels(ax, pts, labels):
    """Greedy collision-free labelling: important labels first, then the rest
    wherever they fit. Labels for dominated points that can't fit are dropped."""
    fig = ax.figure
    renderer = fig.canvas.get_renderer()
    axes_box = ax.get_window_extent(renderer)
    taken = []
    for p in pts:  # keep labels off the markers themselves
        x, y = ax.transData.transform((p["time"], p["size"]))
        taken.append(matplotlib.transforms.Bbox.from_extents(x - 4, y - 4, x + 4, y + 4))
    for optional, p in sorted(labels, key=lambda l: l[0]):
        style = dict(fontsize=6.5, color=INK_2) if optional else \
            dict(fontsize=7.5, color=INK, fontweight="bold")
        for i, (dx, dy, ha, va) in enumerate(OFFSETS):
            t = ax.annotate(p["label"], (p["time"], p["size"]), xytext=(dx, dy),
                            textcoords="offset points", ha=ha, va=va, zorder=5, **style)
            bb = t.get_window_extent(renderer).expanded(1.05, 1.15)
            fits = (axes_box.contains(bb.x0, bb.y0) and axes_box.contains(bb.x1, bb.y1)
                    and not any(bb.overlaps(o) for o in taken))
            if fits or (not optional and i == len(OFFSETS) - 1):
                taken.append(bb)
                break
            t.remove()


def main():
    results = Path(sys.argv[1]) if len(sys.argv) > 1 else max(
        (ROOT / "benchmark_results").glob("*.json"))
    out_dir = Path(sys.argv[2]) if len(sys.argv) > 2 else ROOT / "graphs"
    out_dir.mkdir(parents=True, exist_ok=True)
    data = json.loads(results.read_text())

    views = [("", None, "all crates"),
             ("_evolvable", EVOLVABLE, "evolvable-schema formats only")]
    for (suffix, only, view_title), (name, dataset) in (
            (v, d) for d in data["datasets"].items() for v in views):
        fig, axes = plt.subplots(2, 2, figsize=(16, 13), facecolor=SURFACE)
        # Fixed layout up front: label placement measures pixel positions.
        fig.subplots_adjust(left=0.07, right=0.98, bottom=0.06, top=0.91,
                            wspace=0.18, hspace=0.22)
        for row, (op, op_title) in enumerate(OPS):
            for col, (size_key, size_title) in enumerate(SIZES):
                pts = points(dataset, op, size_key, only)
                frontier = plot_panel(axes[row][col], pts, op_title, size_title)
                print(f"{name}{suffix} {op}/{size_key}: " + ", ".join(p["label"] for p in frontier))

        handles = [
            plt.Line2D([], [], color=FRONTIER, marker="o", linewidth=2, markersize=6,
                       label="Pareto frontier"),
            plt.Line2D([], [], color=MUTED, marker="o", linewidth=0, markersize=5,
                       label="Dominated"),
            plt.Line2D([], [], color=HIGHLIGHT, marker="o", linewidth=0, markersize=6,
                       label=HIGHLIGHT_CRATE + (" (extensible)" if only else "")),
        ]
        fig.legend(handles=handles, loc="upper right", frameon=False, ncol=3,
                   fontsize=9, labelcolor=INK_2)
        fig.suptitle(f"`{name}` dataset: speed vs size, {view_title}", x=0.04, ha="left",
                     color=INK, fontsize=15, fontweight="bold")
        fig.text(0.04, 0.945, f"Source: {results.name}. Bottom-left is better; "
                 "points on the blue staircase are not beaten on both axes by any other.",
                 color=INK_2, fontsize=9)
        out = out_dir / f"pareto_{name}{suffix}.png"
        fig.savefig(out, dpi=130, facecolor=SURFACE)
        plt.close(fig)
        print(f"wrote {out}")


if __name__ == "__main__":
    main()
