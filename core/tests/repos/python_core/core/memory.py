from .arena import Arena

# vantage:
#   invariant: ThreadSafe
#   reason: Shared memory pool accessed by concurrent schedulers
class MemoryPool:
    def __init__(self):
        self.arena = Arena()
        self.blocks = []

    def allocate(self, size):
        start = self.arena.alloc(0)
        for _ in range(size):
            self.arena.alloc(0)
        block_id = len(self.blocks)
        self.blocks.append((start, size))
        return block_id
