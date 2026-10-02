"""Extract context from already production-admitted, immutable ART1/JOB1 bytes.

This is not a noun authenticator: Joy pack-job supplies that admission. The
caller binds the exact files and admission response before and after each use.
"""
from pathlib import Path

FIELD = 0xFFFFFFFF00000001


def particle(value):
    if not isinstance(value, str) or len(value) != 64:
        raise ValueError("particle length")
    try:
        raw = bytes.fromhex(value)
    except ValueError as error:
        raise ValueError("particle hex") from error
    if len(raw) != 32 or raw.hex() != value or any(
        int.from_bytes(raw[i:i + 8], "little") >= FIELD for i in range(0, 32, 8)
    ):
        raise ValueError("canonical particle")
    return value


def positive(value, bits):
    if type(value) is not int or not 0 < value < 1 << bits:
        raise ValueError("positive bounded integer")
    return str(value)


def coordinates(path):
    path = Path(path)
    if not path.is_file() or path.is_symlink() or path.stat().st_size > 16 << 20:
        raise ValueError("bounded regular compiler")
    raw = path.read_bytes()
    if len(raw) < 44 or raw[:8] != b"NOXDAG01":
        raise ValueError("compiler framing")
    count = int.from_bytes(raw[40:44], "little")
    if not 0 < count <= 196_608:
        raise ValueError("compiler node count")
    nodes, cursor = {}, 44
    for _ in range(count):
        if cursor + 33 > len(raw):
            raise ValueError("compiler record header")
        key, length = raw[cursor:cursor + 32], raw[cursor + 32]
        body = raw[cursor + 33:cursor + 33 + length]
        particle(key.hex())
        if key in nodes or length not in (8, 64) or len(body) != length:
            raise ValueError("compiler record")
        if length == 8:
            value = int.from_bytes(body, "little")
            if value >= FIELD:
                raise ValueError("compiler atom")
        else:
            value = (body[:32], body[32:])
            if any(child not in nodes for child in value):
                raise ValueError("compiler prior child")
        nodes[key] = value
        cursor += 33 + length
    if cursor != len(raw):
        raise ValueError("compiler trailing bytes")
    fields, rest = [], raw[8:40]
    for _ in range(5):
        value = nodes.get(rest)
        if not isinstance(value, tuple):
            raise ValueError("compiler ART1 shape")
        head, rest = value
        fields.append(head)
    if nodes.get(rest) != 0 or [nodes.get(p) for p in fields[:4]] != [0x41525431, 0, 1, 1]:
        raise ValueError("compiler ART1 schema")
    return dict(program_particle=raw[8:40].hex(), formula_particle=fields[4].hex(), profile=1)


def derive(coords, admission, job_particle):
    """JOB1 owns reductions/frames; host flags are ceilings, not this context."""
    if type(coords.get("profile")) is not int or coords["profile"] != 1:
        raise ValueError("compiler input profile")
    program = particle(coords["program_particle"])
    formula = particle(coords["formula_particle"])
    job = particle(job_particle)
    if admission["compiler_particle"] != program:
        raise ValueError("admitted compiler coordinate")
    if admission["job_particle"] != job:
        raise ValueError("admitted JOB coordinate")
    limits = admission["limits"]
    return [program, formula, job, "1", positive(limits["reductions"], 64),
            positive(limits["evaluator_frames"], 32)]
