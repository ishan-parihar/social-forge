#!/usr/bin/env python3
"""Seed C-1 drag fixtures: one plain post and one 2-member group, in the
CURRENT week on non-past days.

Written because SQLite holds a `Uuid` as BLOB(16), and a TEXT uuid in a uuid
column is a runtime decode error that takes the whole calendar endpoint down
(AGENTS.md 5.1.1 / 9.33 / 9.44). A fixture seeder that writes the wrong type
does not fail loudly - it 500s every reader of the table.

Idempotency: deletes rows whose content starts with `C1` before seeding, so
repeated runs converge instead of accumulating.
"""
import datetime
import sqlite3
import uuid

DB = 'data/switchpost.db'
APP_USER_ID = bytes([0x22] * 16)  # DEFAULT_USER_ID, 16 bytes of 0x22

# The calendar marks a DAY past when the day is before today, so the last
# two columns of the displayed week are the safe ones (today and tomorrow).
PLAIN = [('C1PLAIN 0', 10), ('C1PLAIN 1', 12)]
# Both group members INSIDE the displayed week: the week grid shows 7 days,
# and the two non-past columns are today and tomorrow - a member seeded two
# days out is not rendered at all, so "both members moved" is unmeasurable.
GROUP = [('C1GROUP 0', 9), ('C1GROUP 1', 9)]

db = sqlite3.connect(DB)


def x_integration_id() -> bytes:
    row = db.execute(
        "SELECT id FROM integrations WHERE lower(provider_identifier) = 'x' LIMIT 1"
    ).fetchone()
    if row is None:
        raise SystemExit('no x integration')
    v = row[0]
    return v if isinstance(v, (bytes, bytearray)) else uuid.UUID(str(v)).bytes


def today() -> datetime.date:
    return datetime.datetime.now(datetime.timezone.utc).date()


def iso_ts(y, m, d, hh) -> int:
    return int(
        datetime.datetime(y, m, d, hh, tzinfo=datetime.timezone.utc).timestamp()
    )


def clean():
    db.execute("DELETE FROM posts WHERE content LIKE 'C1%'")
    db.commit()


def insert(content, ts, group_id=None, sequence=None):
    pid = uuid.uuid4()
    cols = ['id', 'user_id', 'integration_id', 'content', 'media', 'settings',
            'scheduled_at', 'state', 'idempotency_key']
    vals = [pid.bytes, APP_USER_ID, XID, content, '[]', '{}', ts, 'queued',
            uuid.uuid4().bytes]
    if group_id is not None:
        cols += ['group_id', 'sequence']
        vals += [group_id.bytes, sequence]
    q = f"INSERT INTO posts ({', '.join(cols)}) VALUES ({', '.join('?' * len(vals))})"
    db.execute(q, vals)
    return pid


if __name__ == '__main__':
    XID = x_integration_id()
    clean()
    t = today()
    tom = t + datetime.timedelta(days=1)
    for i, (content, hh) in enumerate(PLAIN):
        d = t if i == 0 else tom
        insert(content, iso_ts(d.year, d.month, d.day, hh))
    gid = uuid.uuid4()
    # Both members must be visible at once: the dragged one and the one that
    # should follow it. A member two days out is off the end of the week grid.
    for i, (content, hh) in enumerate(GROUP):
        d = t if i == 0 else tom
        insert(content, iso_ts(d.year, d.month, d.day, hh), gid, i + 1)
    db.commit()
    print(f'seeded plain={[c for c, _ in PLAIN]} group={gid} (user={APP_USER_ID.hex()})')
    print('integration_id type:', db.execute(
        "SELECT DISTINCT typeof(integration_id) FROM posts WHERE content LIKE 'C1%'").fetchall())
    print('group_id type:', db.execute(
        "SELECT DISTINCT typeof(group_id) FROM posts WHERE content LIKE 'C1%'").fetchall())
