#!/usr/bin/env python3
"""Convert Tenhou sanma XML logs to mjai MJSON format.

Two-step pipeline:
1. Download tenhou.net/6 JSON via mjai-reviewer's download (or provide pre-downloaded JSON)
2. Parse the tenhou.net/6 JSON and emit mjai JSONL

Usage:
    # From XML (downloads tenhou6 JSON automatically):
    python3 tenhou6_to_mjai.py --xml path/to/file.xml --out output.mjson

    # From pre-downloaded tenhou6 JSON:
    python3 tenhou6_to_mjai.py --json path/to/tenhou6.json --out output.mjson

    # From game ID (downloads from tenhou.net):
    python3 tenhou6_to_mjai.py --id 2026011603gm-00b9-0000-2b45278f --out output.mjson
"""

import json
import sys
import argparse
import gzip
import subprocess
import re
from pathlib import Path

# Tenhou sanma compact tile encoding:
# 11=1m, 19=9m (only 1m and 9m in sanma)
# 21-29 = 1p-9p
# 31-39 = 1s-9s
# 41-47 = E,S,W,N,P,F,C
# 51=red5m(doesn't exist in sanma), 52=red5p, 53=red5s
# 60 = tsumogiri marker (not a tile)

def tile_to_mjai(tid: int) -> str:
    """Convert tenhou sanma compact tile ID to mjai tile string."""
    if tid == 51:
        return "5mr"
    if tid == 52:
        return "5pr"
    if tid == 53:
        return "5sr"
    tens = tid // 10
    ones = tid % 10
    if tens == 1:
        return f"{ones}m"
    elif tens == 2:
        return f"{ones}p"
    elif tens == 3:
        return f"{ones}s"
    elif tens == 4:
        honors = ["", "E", "S", "W", "N", "P", "F", "C"]
        return honors[ones]
    else:
        raise ValueError(f"Unknown tile ID: {tid}")


def decode_naki(s):
    """Decode tenhou6 naki string. Returns mjai event dict(s)."""
    # All tile IDs are 2 characters in this encoding
    # Format follows convlog conventions:
    # Pon (in tsumo array, len=7): position of 'p' determines source
    #   p at 0: from kamicha "pXXYYZZ" → called=XX, consumed=[YY,ZZ]
    #   p at 2: from toimen  "XXpYYZZ" → called=YY, consumed=[XX,ZZ]
    #   p at 4: from shimocha "XXYYpZZ" → called=ZZ, consumed=[XX,YY]
    # Daiminkan (in tsumo array, len=9): position of 'm' determines source
    # Kakan (in dahai array, len=9): position of 'k' determines source
    # Ankan (in dahai array, len=9): 'a' at position 6 "XXYYZZaWW"
    # Reach (in dahai array, len=3): "rXX"
    return s  # placeholder, actual parsing done inline


def parse_tile_pair(s, start):
    """Parse 2-char tile ID from string at given position."""
    return int(s[start:start+2])


def convert_kyoku(kyoku_data, names, kyoku_idx):
    """Convert one kyoku from tenhou6 format to mjai events."""
    events = []

    seed = kyoku_data[0]  # [kyoku, honba, kyotaku]
    scores = kyoku_data[1][:3]  # only 3 players
    dora_indicators = kyoku_data[2]
    ura_indicators = kyoku_data[3]

    # Players' data: haipai, tsumo, dahai for each of 3 players
    players = []
    for p in range(3):
        base = 4 + p * 3
        haipai = kyoku_data[base]
        tsumo = kyoku_data[base + 1]
        dahai = kyoku_data[base + 2]
        players.append({"haipai": haipai, "tsumo": tsumo, "dahai": dahai})

    result_data = kyoku_data[16] if len(kyoku_data) > 16 else []

    # Determine bakaze and kyoku number
    bakaze_idx = seed[0] // 3  # 0=E, 1=S
    kyoku_num = seed[0] % 3 + 1
    bakaze = ["E", "S", "W", "N"][bakaze_idx]
    honba = seed[1]
    kyotaku = seed[2]
    oya = seed[0] % 3

    # Build tehais
    tehais = []
    for p in range(3):
        hand = [tile_to_mjai(t) for t in sorted(players[p]["haipai"])]
        tehais.append(hand)

    # start_kyoku
    events.append({
        "type": "start_kyoku",
        "bakaze": bakaze,
        "dora_marker": tile_to_mjai(dora_indicators[0]),
        "kyoku": kyoku_num,
        "honba": honba,
        "kyotaku": kyotaku,
        "oya": oya,
        "scores": scores,
        "tehais": tehais,
    })

    # Build interleaved event sequence
    # In tenhou6, each player has parallel tsumo[] and dahai[] arrays
    # They are consumed in seat order starting from oya
    # Naki in tsumo[] means "instead of normal draw, this player called"
    # Naki in dahai[] means "instead of normal discard, player did kan/reach"
    
    tsumo_idx = [0, 0, 0]
    dahai_idx = [0, 0, 0]
    dora_revealed = 1  # first dora already shown at start

    actor = oya
    last_tsumo_tile = [None, None, None]  # track for tsumogiri

    while True:
        p = actor
        ti = tsumo_idx[p]
        di = dahai_idx[p]

        # Check if this player has more actions
        if ti >= len(players[p]["tsumo"]) and di >= len(players[p]["dahai"]):
            break

        # Process tsumo (draw or naki call)
        if ti < len(players[p]["tsumo"]):
            tsumo_raw = players[p]["tsumo"][ti]
            tsumo_idx[p] += 1

            if isinstance(tsumo_raw, int):
                # Normal tsumo
                last_tsumo_tile[p] = tsumo_raw
                events.append({
                    "type": "tsumo",
                    "actor": p,
                    "pai": tile_to_mjai(tsumo_raw),
                })
            elif isinstance(tsumo_raw, str):
                # Naki call (pon/daiminkan from someone's discard)
                naki = tsumo_raw
                if 'p' in naki and len(naki) == 7:
                    # Pon
                    idx_p = naki.index('p')
                    if idx_p == 0:
                        # from kamicha (actor-1 in 3p = actor+2 mod 3)
                        target = (p + 2) % 3
                        called = parse_tile_pair(naki, 1)
                        consumed = [parse_tile_pair(naki, 3), parse_tile_pair(naki, 5)]
                    elif idx_p == 2:
                        # from toimen (actor+1 in 3p... but 3p has no toimen)
                        # In sanma, idx=2 means from the other opponent
                        target = (p + 1) % 3
                        called = parse_tile_pair(naki, 3)
                        consumed = [parse_tile_pair(naki, 0), parse_tile_pair(naki, 5)]
                    elif idx_p == 4:
                        # from shimocha (actor+1 mod 3)
                        target = (p + 1) % 3
                        called = parse_tile_pair(naki, 5)
                        consumed = [parse_tile_pair(naki, 0), parse_tile_pair(naki, 2)]
                    else:
                        raise ValueError(f"Invalid pon naki: {naki}")
                    events.append({
                        "type": "pon",
                        "actor": p,
                        "target": target,
                        "pai": tile_to_mjai(called),
                        "consumed": [tile_to_mjai(consumed[0]), tile_to_mjai(consumed[1])],
                    })
                    last_tsumo_tile[p] = None
                elif 'm' in naki and len(naki) == 9:
                    # Daiminkan
                    idx_m = naki.index('m')
                    if idx_m == 0:
                        target = (p + 2) % 3
                        called = parse_tile_pair(naki, 1)
                        consumed = [parse_tile_pair(naki, 3), parse_tile_pair(naki, 5), parse_tile_pair(naki, 7)]
                    elif idx_m == 2:
                        target = (p + 1) % 3
                        called = parse_tile_pair(naki, 3)
                        consumed = [parse_tile_pair(naki, 0), parse_tile_pair(naki, 5), parse_tile_pair(naki, 7)]
                    elif idx_m == 6:
                        target = (p + 1) % 3
                        called = parse_tile_pair(naki, 7)
                        consumed = [parse_tile_pair(naki, 0), parse_tile_pair(naki, 2), parse_tile_pair(naki, 4)]
                    else:
                        raise ValueError(f"Invalid daiminkan naki: {naki}")
                    events.append({
                        "type": "daiminkan",
                        "actor": p,
                        "target": target,
                        "pai": tile_to_mjai(called),
                        "consumed": [tile_to_mjai(c) for c in consumed],
                    })
                    last_tsumo_tile[p] = None
                else:
                    raise ValueError(f"Unknown tsumo naki: {naki}")

                # After naki, process dahai for this player then continue with same player for next tsumo
                # (naki steals the turn)
        else:
            break

        # Process dahai (discard, kan, or reach)
        if di < len(players[p]["dahai"]):
            dahai_raw = players[p]["dahai"][di]
            dahai_idx[p] += 1

            if isinstance(dahai_raw, int):
                if dahai_raw == 60:
                    # Tsumogiri
                    if last_tsumo_tile[p] is not None:
                        # Check if this is nukidora (tsumogiri of N tile)
                        if last_tsumo_tile[p] == 44:
                            events.append({
                                "type": "nukidora",
                                "actor": p,
                                "pai": "N",
                            })
                            # After nukidora, same player draws rinshan
                            continue
                        events.append({
                            "type": "dahai",
                            "actor": p,
                            "pai": tile_to_mjai(last_tsumo_tile[p]),
                            "tsumogiri": True,
                        })
                    else:
                        # After naki, 60 means discard the called tile? Shouldn't happen
                        break
                elif dahai_raw == 0:
                    # 0 after daiminkan = placeholder (no discard needed, rinshan follows)
                    # 0 otherwise = nukidora from hand
                    if last_tsumo_tile[p] is None:
                        # After naki (daiminkan), 0 is a placeholder — skip, same player continues
                        continue
                    else:
                        # Nukidora from hand (not from tsumo)
                        events.append({
                            "type": "nukidora",
                            "actor": p,
                            "pai": "N",
                        })
                        # After nukidora, same player draws rinshan
                        continue
                else:
                    # Tedashi (hand-pick discard)
                    events.append({
                        "type": "dahai",
                        "actor": p,
                        "pai": tile_to_mjai(dahai_raw),
                        "tsumogiri": False,
                    })
            elif isinstance(dahai_raw, str):
                naki = dahai_raw
                if naki.startswith('f'):
                    # Tsumogiri with explicit tile
                    tile_id = int(naki[1:])
                    # Check if this is nukidora
                    if tile_id == 44:
                        events.append({
                            "type": "nukidora",
                            "actor": p,
                            "pai": "N",
                        })
                        # After nukidora, same player draws rinshan
                        continue
                    events.append({
                        "type": "dahai",
                        "actor": p,
                        "pai": tile_to_mjai(tile_id),
                        "tsumogiri": True,
                    })
                elif naki.startswith('r'):
                    # Reach
                    tile_str = naki[1:]
                    if tile_str == "60":
                        # Tsumogiri reach
                        events.append({"type": "reach", "actor": p})
                        events.append({
                            "type": "dahai",
                            "actor": p,
                            "pai": tile_to_mjai(last_tsumo_tile[p]),
                            "tsumogiri": True,
                        })
                    else:
                        tile_id = int(tile_str)
                        events.append({"type": "reach", "actor": p})
                        events.append({
                            "type": "dahai",
                            "actor": p,
                            "pai": tile_to_mjai(tile_id),
                            "tsumogiri": False,
                        })
                    events.append({"type": "reach_accepted", "actor": p})
                    # Don't advance actor after reach — same player continues
                    actor = (p + 1) % 3
                    continue
                elif 'k' in naki and len(naki) == 9:
                    # Kakan
                    idx_k = naki.index('k')
                    if idx_k == 0:
                        pai = parse_tile_pair(naki, 1)
                        consumed = [parse_tile_pair(naki, 3), parse_tile_pair(naki, 5), parse_tile_pair(naki, 7)]
                    elif idx_k == 2:
                        pai = parse_tile_pair(naki, 3)
                        consumed = [parse_tile_pair(naki, 0), parse_tile_pair(naki, 5), parse_tile_pair(naki, 7)]
                    elif idx_k == 4:
                        pai = parse_tile_pair(naki, 5)
                        consumed = [parse_tile_pair(naki, 0), parse_tile_pair(naki, 2), parse_tile_pair(naki, 7)]
                    else:
                        raise ValueError(f"Invalid kakan: {naki}")
                    events.append({
                        "type": "kakan",
                        "actor": p,
                        "pai": tile_to_mjai(pai),
                        "consumed": [tile_to_mjai(c) for c in consumed],
                    })
                    # After kakan, same player draws rinshan (next tsumo)
                    continue
                elif 'a' in naki and len(naki) == 9:
                    # Ankan: "XXYYZZaWW"
                    consumed = [
                        parse_tile_pair(naki, 0),
                        parse_tile_pair(naki, 2),
                        parse_tile_pair(naki, 4),
                        parse_tile_pair(naki, 7),
                    ]
                    events.append({
                        "type": "ankan",
                        "actor": p,
                        "consumed": [tile_to_mjai(c) for c in consumed],
                    })
                    # After ankan, same player draws rinshan
                    continue
                else:
                    raise ValueError(f"Unknown dahai naki: {naki}")
        else:
            break

        # Advance to next player
        actor = (p + 1) % 3

    # End kyoku with result
    if result_data:
        result_type = result_data[0] if result_data else ""
        if result_type == "和了":
            # Parse hora results: alternating [deltas, detail] pairs after the type string
            i = 1
            while i + 1 < len(result_data):
                if isinstance(result_data[i], list) and isinstance(result_data[i+1], list):
                    deltas = [int(x) for x in result_data[i][:3]]
                    detail = result_data[i+1]
                    # detail format: [actor, target, actor_again, description, yaku...]
                    hora_actor = int(detail[0]) if len(detail) > 0 else 0
                    hora_target = int(detail[1]) if len(detail) > 1 else hora_actor
                    events.append({
                        "type": "hora",
                        "actor": hora_actor,
                        "target": hora_target,
                        "deltas": deltas,
                    })
                    i += 2
                else:
                    break
        elif result_type in ("流局", "流し満貫"):
            deltas = [int(x) for x in result_data[1][:3]] if len(result_data) > 1 and isinstance(result_data[1], list) else [0, 0, 0]
            events.append({"type": "ryukyoku", "deltas": deltas})
        # 九種九牌, 全員聴牌, 全員不聴 etc. — no deltas
        elif result_type in ("九種九牌", "全員聴牌", "全員不聴", "三家和了"):
            events.append({"type": "ryukyoku"})

    events.append({"type": "end_kyoku"})
    return events


def convert_tenhou6_json(data):
    """Convert full tenhou6 JSON to mjai events."""
    names = data.get("name", ["", "", "", ""])[:3]
    all_events = []

    all_events.append({
        "type": "start_game",
        "names": names,
    })

    for i, kyoku_data in enumerate(data["log"]):
        kyoku_events = convert_kyoku(kyoku_data, names, i)
        all_events.extend(kyoku_events)

    all_events.append({"type": "end_game"})
    return all_events


def main():
    parser = argparse.ArgumentParser(description="Convert tenhou sanma logs to mjai MJSON")
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--json", help="Input tenhou.net/6 JSON file")
    group.add_argument("--id", help="Tenhou game ID (will download)")
    parser.add_argument("--out", "-o", help="Output mjson file (default: stdout)")
    args = parser.parse_args()

    if args.id:
        # Download using mjai-reviewer
        import tempfile
        with tempfile.NamedTemporaryFile(suffix=".json", delete=False) as f:
            tmp_path = f.name
        result = subprocess.run([
            "/Users/cat/build/mjai-reviewer/target/release/mjai-reviewer",
            "--tenhou-id", args.id,
            "--tenhou-out", tmp_path,
            "--out-file", "/dev/null",
            "--engine", "mortal",
        ], capture_output=True, text=True)
        # mjai-reviewer will fail on parse but still saves the raw JSON
        json_path = tmp_path
    else:
        json_path = args.json

    with open(json_path) as f:
        data = json.load(f)

    events = convert_tenhou6_json(data)

    if args.out:
        with gzip.open(args.out, "wt") if args.out.endswith(".gz") else open(args.out, "w") as f:
            for ev in events:
                f.write(json.dumps(ev, ensure_ascii=False) + "\n")
    else:
        for ev in events:
            print(json.dumps(ev, ensure_ascii=False))


if __name__ == "__main__":
    main()
