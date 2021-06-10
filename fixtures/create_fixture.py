#!/usr/bin/env python3
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
    entries = [root]
    root_dir = path.dirname(root_path) + path.sep
    _int = int  # Optimization for CPython since we call `int` in a tight loop
    while entries:
        entry = entries.pop()
        del entry["prot"]
        entry["mode"] = _int(entry["mode"], 8)
        entry["name"] = entry["name"].removeprefix(root_dir)
        contents = entry.get("contents")
        if contents:
            if entry["type"] == "directory":
                entries.extend(entry["contents"])
            else:
                del entry["contents"]


if __name__ == '__main__':
    if len(sys.argv) != 2:
        sys.exit("Error: please provide a path to generate a fixture from")
    root_path = path.abspath(sys.argv[1])
    tree = json.loads(subprocess.run(
        f'tree -sJpf --noreport --dirsfirst -- "{root_path}"',
        shell=True, capture_output=True).stdout)
    postprocess(tree, root_path)
    with open(path.join('fixtures', f'{path.basename(root_path)}.json'), 'w') as file:
        json.dump(tree, file, sort_keys=True)
