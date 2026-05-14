# @epistemic:p1
def process_data(items):
    """Process a list of items."""
    return [x * 2 for x in items]

# @epistemic:c1
class DataProcessor:
    def __init__(self, factor=1.0):
        self.factor = factor
    
    def scale(self, value):
        return value * self.factor
