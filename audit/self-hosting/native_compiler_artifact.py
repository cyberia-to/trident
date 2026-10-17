"""Independent output reader; Joy owns canonical artifact/profile admission."""

P = 18446744069414584321


def decode(path):
    """Small independent reader for the already admitted canonical output DAG."""
    data = path.read_bytes()
    assert data[:8] == b"NOXDAG01"
    root, count, cursor, nodes = data[8:40], int.from_bytes(data[40:44], "little"), 44, {}
    for _ in range(count):
        particle = data[cursor:cursor + 32]
        width = data[cursor + 32]
        cursor += 33
        payload = data[cursor:cursor + width]
        cursor += width
        assert particle not in nodes
        if width == 8:
            value = int.from_bytes(payload, "little")
            assert value < P
        else:
            assert width == 64
            value = (nodes[payload[:32]], nodes[payload[32:]])
        nodes[particle] = value
    assert cursor == len(data) and particle == root
    return nodes[root]


def record(tag, *fields):
    body = 0
    for value in reversed(fields):
        body = (value, body)
    return tag, body
