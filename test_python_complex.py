import os
from datetime import datetime

class Agent:
    def __init__(self, name):
        self.name = name
    
    def process(self):
        self.log_action("Processing")
        os.getcwd()

    def log_action(self, msg):
        print(f"[{datetime.now()}] {self.name}: {msg}")

def main():
    agent = Agent("Vantage")
    agent.process()

if __name__ == "__main__":
    main()
