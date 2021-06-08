#!/usr/bin/env python3
import builtins
import json
import subprocess
import sys
from os import stat, path


def postprocess(tree, root_path):
    assert(len(tree) == 1)
    stat_result = stat(root_path)
    root = tree[0]
    root["mode"] = '0{:o}'.format(stat_result.st_mode & 0o777)
    root["size"] = stat_result.st_size
    root["prot"] = ""
    if root["type"] != "directory":
        del root["contents"]
    entries = [root]
    root_dir = path.dirname(root_path) + path.sep
    int = builtins.int  # Optimization for CPython since we call `int` in a tight loop
    while entries:
        entry = entries.pop()
        del entry["prot"]
        entry["mode"] = int(entry["mode"], 8)
        entry["name"] = entry["name"].removeprefix(root_dir)
        if "contents" in entry:
            entries.extend(entry["contents"])


if __name__ == '__main__':
    if len(sys.argv) != 2:
        sys.exit("Error: please provide a path to generate a fixture from")
    root_path = path.abspath(sys.argv[1])
    tree = json.loads(subprocess.run(
        f'tree -sJpf --noreport --dirsfirst -- "{root_path}"',
        shell=True, capture_output=True).stdout)
    postprocess(tree, root_path)
    with open(f'fixtures/{path.basename(root_path)}.json', 'w') as file:
        json.dump(tree, file, sort_keys=True)
