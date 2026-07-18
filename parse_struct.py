import json

def read_struct():
    with open("src/tasker/config.rs", "r") as f:
        print("".join(f.readlines()[:100]))
read_struct()
