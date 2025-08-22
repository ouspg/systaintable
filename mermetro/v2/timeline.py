"""timeline.py

Chronological timeline visualization for a selected group.
Produces Mermaid `timeline` diagram syntax.

Design:
- Aggregate group entries by second (timestamp string up to seconds)
- For each second, list up to N distinct (type,value) pairs
- Long lists are truncated with an ellipsis marker
- Sections per entry type group (PERSONAL vs TECHNICAL) would make the
  diagram long; instead we output one unified section ordered by time.
- Fallback message if no timestamps.

Public API:
    generate_group_timeline(group_id, node_details, max_events=250,
                            per_timestamp_limit=8) -> str
"""
from __future__ import annotations
from datetime import datetime
from typing import Dict, Any, List, Tuple

_TIME_FORMATS = ["%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S"]


def _parse(ts: str):
    for fmt in _TIME_FORMATS:
        try:
            return datetime.strptime(ts, fmt)
        except ValueError:
            continue
    return None


def _short(val: str, max_len: int = 30) -> str:
    if len(val) <= max_len:
        return val
    return val[: max_len - 1] + '…'


# NEW: sanitize label (remove mermaid-breaking chars like additional colons/newlines)
def _sanitize(text: str) -> str:
    return text.replace(':', ' ').replace('\n', ' ').replace('\r', ' ').strip()


def generate_group_timeline(group_id: str, node_details: Dict[str, Any], *, max_events: int = 250, per_timestamp_limit: int = 8) -> str:
    group = node_details.get(group_id)
    if not group or group.get('type') != 'Group':
        return 'timeline\n    title Timeline\n    section Info\n        N_A : Group not found'

    entries: List[Dict[str, Any]] = group.get('entries', [])
    if not entries:
        return 'timeline\n    title Timeline\n    section Info\n        N_A : No entries in group'

    # Collect by second
    buckets: Dict[str, List[Tuple[str, str]]] = {}
    for e in entries:
        ts_raw = e.get('timestamp') or 'N/A'
        dt = _parse(ts_raw) if ts_raw != 'N/A' else None
        if not dt:
            continue
        # IMPORTANT: remove ':' characters from timestamp token because Mermaid timeline
        # parser treats the first ':' in a line as the field separator. Internal colons
        # break parsing -> produce syntax error. Format without colons.
        key = dt.strftime('%Y-%m-%dT%H%M%S')  # e.g. 2025-08-21T142355
        etype = _sanitize(str(e.get('type', 'Unknown')))
        val = _sanitize(str(e.get('value', '')))
        buckets.setdefault(key, []).append((etype, val))

    if not buckets:
        return 'timeline\n    title Timeline\n    section Info\n        N_A : No timestamped data'

    # Sort timestamps
    ordered_keys = sorted(buckets.keys())

    # Build mermaid timeline
    lines = ['timeline', f'    title Group {group_id} chronological activity', '    section Events']

    event_count = 0
    for ts_key in ordered_keys:
        pairs = buckets[ts_key]
        # Deduplicate by (type,value) while preserving order
        seen = set()
        unique: List[str] = []
        for etype, val in pairs:
            k = (etype, val)
            if k in seen:
                continue
            seen.add(k)
            # Use hyphen instead of colon inside label to avoid extra ':' tokens
            unique.append(_short(f"{etype}-{val}"))
            if len(unique) >= per_timestamp_limit:
                unique.append('…')
                break
        label = ', '.join(unique) if unique else 'activity'
        label = _sanitize(label)
        # Final line: <timestamp_token> : <label> (only one colon in the line)
        lines.append(f"        {ts_key} : {label}")
        event_count += 1
        if event_count >= max_events:
            lines.append(f"        {ts_key} : truncated …")
            break

    return '\n'.join(lines) + '\n'
