# Copyright 2025 The Safety Net Authors
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

import argparse
import sys
import os
import json

# cargo llvm-cov --all-features --workspace --json
if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "-p",
        "--percent",
        dest="percent",
        required=False,
        help="Minimum code coverage per line per source file",
        type=float,
        default=80.0,
    )
    parser.add_argument(
        "-w",
        "--whitelist",
        dest="whitelist",
        nargs="+",
        help="Files to whitelist from coverage checks",
        type=str,
        default=["main.rs"],
    )
    parser.add_argument(
        "input", nargs="?", type=argparse.FileType("r"), default=sys.stdin
    )
    parser.add_argument(
        "output", nargs="?", type=argparse.FileType("w"), default=sys.stdout
    )
    args = parser.parse_args()

    whitelisted = set(args.whitelist)
    data = json.load(args.input)
    data = data["data"]
    percent = args.percent
    passed = True

    print(f"### Code Coverage Summary ({percent:.2f}%)", file=args.output)

    for datum in data:
        files = datum["files"]
        for record in files:
            filePassed = True
            name = record["filename"]
            stem = (
                os.path.basename(os.path.dirname(name)) + "/" + os.path.basename(name)
            )
            if stem in whitelisted:
                continue
            lineCoverage = record["summary"]["lines"]["percent"]
            if lineCoverage < percent:
                print(
                    f"#### {name}: Only {lineCoverage:.2f}% by line", file=args.output
                )
                print(f"```rust", file=args.output)
                passed = False
                with open(name, "r") as f:
                    lines = f.readlines()
                    covered = set()
                    startSeg = True
                    lastLine = None
                    for segment in record["segments"]:
                        line = max(segment[0] - 1, 0)

                        if (
                            lastLine is not None
                            and line not in covered
                            and line != lastLine + 1
                        ):
                            startSeg = True

                        if startSeg:
                            print(
                                f"// {stem}:{line}",
                                file=args.output,
                            )
                            startSeg = False

                        executed = segment[2] != 0
                        if not executed:
                            txt = lines[line].strip("\n").removeprefix(" }")
                            if (
                                line not in covered
                                and len(lines[line].strip(" \n").removeprefix("}")) > 0
                            ):
                                print(
                                    f"{txt}",
                                    file=args.output,
                                )

                        covered.add(line)
                        lastLine = line
                print(f"```", file=args.output)

    if passed:
        print("### All files passed", file=args.output)
    sys.exit(0 if passed else 1)
