from typing import List, Union, Optional, Final


class Field:
    def __init__(self, a: str): ...


class Piece:
    """
    Piece in a solution, possibly broken across nonadjacent rows.
    Immutable, like a tuple.
    """

    def __init__(
        self,
        shape:       str,
        orientation: str,
        column:      int,
        rows:        List[int],
    ): ...

    shape: Final[str]
    "One of 'IJLOSTZ'"

    orientation: Final[str]
    "One of ['North', 'East', 'South', 'West']"

    column: Final[int]
    "Leftmost column the piece occupies (left bounding box)"

    rows: Final[List[int]]
    "All rows the piece occupies"


class Solution:
    """
    Collection of `Piece`s placed in a `Field`.
    """

    initial_field: Field
    pieces:        List[Piece]
    held:          Optional[str]


class Fumen:
    def __init__(self, init: Union[Field, Solution, str]): ...


class Srs4lSolver:
    pass
