class Field:
    def __init__(self, height):
        self.field = bytearray(b"          ") * height

    def _as_bytes(b):
        return b.encode() if type(b) == str else b

    _chars = dict((v, chr(v)) for v in b" GIJLOSTZ")
    _mirrors = {
        ord(" "): ord(" "),
        ord("G"): ord("G"),
        ord("I"): ord("I"),
        ord("J"): ord("L"),
        ord("L"): ord("J"),
        ord("O"): ord("O"),
        ord("S"): ord("Z"),
        ord("T"): ord("T"),
        ord("Z"): ord("S"),
    }

    @classmethod
    def from_initial(cls, initial):
        height = (len(initial) + 9) // 10
        field = Field(height)
        field[0 : len(initial)] = initial
        return field

    def __getitem__(self, idx):
        if type(idx) == int:
            idx = slice(idx, idx + 1)
        elif type(idx) == tuple:
            row, col = idx
            idx = row + 10 * col
            idx = slice(idx, idx + 1)

        return self.field[idx].decode()

    def __setitem__(self, idx, value):
        if type(value) == str:
            value = value.encode()

        if type(idx) == int:
            idx = slice(idx, idx + 1)
        elif type(idx) == tuple:
            row, col = idx
            idx = row + 10 * col
            idx = slice(idx, idx + 1)

        assert len(self.field[idx]) == len(value)
        for v in value:
            assert v in Field._chars
        self.field[idx] = value

    def lines(self):
        for i in range(0, len(self.field), 10):
            yield self.field[i : i + 10]

    def increase_lines(self, count):
        self.field.extend(b"          " * count)

    def extend(self, value):
        assert len(value) % 10 == 0
        line_count = len(value) // 10
        self.increase_lines(line_count)
        self[-len(value) :] = value

    def mirror(self):
        for i in range(0, len(self.field), 10):
            r = reversed(self.field[i : i + 10])
            self.field[i : i + 10] = [Field._mirrors[v] for v in r]
