from typing import Iterable
import os
import time
import subprocess


def enumerate_files(path: str) -> Iterable[str]:
    if os.path.isfile(path):
        yield path
    else:
        for child in sorted(os.listdir(path)):
            yield from enumerate_files(os.path.join(path, child))


def execute_trial_sat(instance_file_path: str, log_file_path: str):

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

    print(instance_file_path, status, end_time - start_time)
    

def main():

    if not os.path.exists("log"):
        os.mkdir("log")

    for instance_file_path in enumerate_files("instance"):
        if not instance_file_path.endswith(".cnf"):
            continue
        log_file_path = os.path.join("log", os.path.basename(instance_file_path).rstrip(".cnf") + ".log")
        execute_trial_sat(instance_file_path, log_file_path)

if __name__ == "__main__":
    main()
