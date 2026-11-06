def _segments(text):
    seg, quoted, out = [], False, []
    for ch in text:
        if ch == '"':
            quoted = not quoted
            seg.append(ch)
        elif ch == ";" and not quoted:
            out.append("".join(seg)); seg = []
        else:
            seg.append(ch)
    out.append("".join(seg))
    return out


def parse_kv(text):
    out = {}
    for part in _segments(text):
        part = part.strip()
        if not part:
            continue
        if "=" not in part:
            raise ValueError(part)
        key, value = part.split("=", 1)
        value = value.strip()
        if len(value) >= 2 and value[0] == '"' and value[-1] == '"':
            value = value[1:-1]
        out[key.strip()] = value
    return out
