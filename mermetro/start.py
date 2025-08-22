import argparse
import sys
import os
from multiprocessing import Process

BASE_DIR = os.path.dirname(os.path.abspath(__file__))

if BASE_DIR not in sys.path:
    sys.path.insert(0, BASE_DIR)

from v1 import mermetro
from v2 import mermetro2


def run_v1(jsonfile, use_m):
    mermetro.start_app(jsonfile=jsonfile, multiprocessing=use_m, host='127.0.0.1', port=5000)

def run_v2(jsonfile, use_m):
    mermetro2.start_app(jsonfile=jsonfile, multiprocessing=use_m, host='127.0.0.1', port=5001)


def main():
    parser = argparse.ArgumentParser(
        description="Example: python start.py data/lokitiedosto.json --mode 2 -m"
    )
    parser.add_argument("jsonfile", help="Path to the JSON log file")
    parser.add_argument("--mode", type=int, default=0, choices=[0,1,2],
                        help="0=both, 1=only v1, 2=only v2 (default 0)")
    parser.add_argument("-m", "--multiprocessing", action="store_true",
                        help="Enables multiprocessing")
    args = parser.parse_args()

    if not os.path.isfile(args.jsonfile):
        print(f"Error: File '{args.jsonfile}' not found.\n")
        sys.exit(1)

    print("\nStarting up...")
    print(f" JSON: {args.jsonfile}")
    print(f" Mode: {args.mode}")
    print(f" Multiprocessing: {bool(args.multiprocessing)}")

    processes = []

    if args.mode in (0,1):
        p1 = Process(target=run_v1, args=(args.jsonfile, args.multiprocessing))
        p1.start()
        processes.append(("v1", p1))
        print(" v1: http://localhost:5000")

    if args.mode in (0,2):
        p2 = Process(target=run_v2, args=(args.jsonfile, args.multiprocessing))
        p2.start()
        processes.append(("v2", p2))
        print(" v2: http://localhost:5001")

if __name__ == "__main__":
    main()