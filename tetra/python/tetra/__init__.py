import tetra.native
from .native import Field, QueueSet


class Fumen:
    def __init__(self, *args):
        self.native = tetra.native.Fumen(*args)

    def __repr__(self):
        return self.native.__repr__()

    def __str__(self):
        return self.native.__str__()


class Solver:
    def __init__(self, *, engine='srs-4l'):
        if engine == 'srs-4l':
            self.native = tetra.native.Srs4lSolver()
        else:
            raise ValueError('unknown engine')
