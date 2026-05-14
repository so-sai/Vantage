from .memory import MemoryPool

# vantage:
#   invariant: DeterministicOrdering
#   reason: Task execution order must be reproducible
class Scheduler:
    def __init__(self):
        self.pool = MemoryPool()
        self.tasks = []

    def spawn(self, name):
        self.pool.allocate(len(name))
        self.tasks.append(name)

    def run_all(self):
        for task in self.tasks:
            print(f"Running: {task}")
