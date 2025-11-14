import argparse
import ast
import json
import sys

def insert_mutables(tokens: dict) -> dict:
    tokens[1] = '"timestamp"'
    return tokens

def load_tokens(fname: str) -> dict:
    tokens, tokenstoint = dict(), dict()
    with open(fname, encoding='UTF-8') as fobj:
        _, tokenstoint = json.load(fobj)
    for key, val in tokenstoint.items():
        tokens[val] = key
    return insert_mutables(tokens)


def main(args: argparse.Namespace) -> None:
    tokens = load_tokens(args.tokenfile)
    for line in sys.stdin:
        line = json.loads(line)
        outline = {}
        for key, val in zip(line[::2], line[1::2]):
            outline[ast.literal_eval(tokens[key])] = ast.literal_eval(tokens[val])
        print(json.dumps(outline))

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument("-f", "--tokenfile", help="Tokenfile")
    parser.add_argument("-d", "--debug", action="store_true",
                        default=False, help="Debug")
    args = parser.parse_args()
    main(args)

