import re

with open("src/runner/engine/pipeline.rs", "r") as f:
    code = f.read()

# move execute_action BEFORE tests

match_tests = re.search(r"#\[cfg\(test\)\]\nmod tests \{", code)
if match_tests:
    test_start = match_tests.start()
    test_block = code[test_start:code.find("async fn execute_action(")]

    exec_action_block = code[code.find("async fn execute_action("):]

    code = code[:test_start] + exec_action_block + "\n\n" + test_block

with open("src/runner/engine/pipeline.rs", "w") as f:
    f.write(code)
