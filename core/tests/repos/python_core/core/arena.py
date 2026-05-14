# vantage:
#   invariant: AppendOnly
#   reason: Prevent rollback corruption in memory arena
class Arena:
    def __init__(self):
        self.items = []
        self.generation = 0

    def alloc(self, item):
        self.items.append(item)
        return len(self.items) - 1

    def get(self, index):
        if 0 <= index < len(self.items):
            return self.items[index]
        return None
