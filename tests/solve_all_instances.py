from typing import Iterable
import os
import sys
import time
import subprocess
from concurrent.futures.thread import ThreadPoolExecutor
import datetime

def enumerate_files(path: str) -> Iterable[str]:
    if os.path.isfile(path):
        yield path
    else:
        for child in sorted(os.listdir(path)):
            yield from enumerate_files(os.path.join(path, child))


def execute_trial_sat(instance_file_path: str, log_file_path: str) -> tuple[str, str, float]:

    log_file = open(log_file_path, "w")
    start_time = time.time()
    try:
        result = subprocess.run(
            ["../target/release/trial_sat"],
            stdin=open(instance_file_path),
            # NOTE: バッファリングの方法次第では，log_file の末尾に status が出力される保証がないので，このやり方はまずいかも
            stdout=log_file,
            stderr=log_file,
            timeout=60,
        )
    except subprocess.TimeoutExpired:
        status = "TIMEOUT"
    else:
        status = open(log_file_path).readlines()[-1].rstrip()

    end_time = time.time()
    log_file.close()

    return instance_file_path, status, end_time - start_time
    sys.stdout.flush()


def _execute_trial_sat(io_files: tuple[str, str]) -> tuple[str, str, float]:
    return execute_trial_sat(io_files[0], io_files[1])


def main():

    result_dir_path = f"result_{datetime.datetime.now().strftime("%Y%m%d%H%M%S")}"

    os.mkdir(result_dir_path)
    log_dir_path = os.path.join(result_dir_path, "log")
    os.mkdir(log_dir_path)

    result_file = open(os.path.join(result_dir_path, "result.txt"), "w")

    io_file_paths = [
        (instance_file_path, os.path.join(log_dir_path, os.path.basename(instance_file_path).rstrip(".cnf") + ".log"))
        for instance_file_path in enumerate_files("instance") if instance_file_path.endswith(".cnf")
    ]

    for i, (instance_file_path, status, t) in enumerate(ThreadPoolExecutor(max_workers=1).map(_execute_trial_sat, io_file_paths)):
        line = "\t".join(map(str, [f"{i + 1}/{len(io_file_paths)}", instance_file_path , status ,t]))
        print(line)
        result_file.write(line)
        result_file.write("\n")
        result_file.flush()
        sys.stdout.flush()
    result_file.close()


if __name__ == "__main__":
    main()
