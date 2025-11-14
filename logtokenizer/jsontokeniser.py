import argparse
import json
import os
import sys
import typing

from collections import Counter

START_TOKEN = 1000


def load_tokens(fname: str) -> dict:
    if os.path.exists(fname):
        with open(fname, encoding='UTF-8') as fobj:
            return json.load(fobj)
    else:
        return START_TOKEN, {}


def save_tokens(max_token: int, tokens: dict, fname: str) -> None:
    with open(fname, 'w') as fobj:
        return json.dump([max_token, tokens], fobj)


def find_mutables(key: object):
    if not type(key) == str:
        return
    #timestamps
    if key.startswith('2025') or (key.startswith("1") and len(key) == 10):
        return 1


def add_token(max_token: int, tokens: dict, key: object) -> None:
    mutable = find_mutables(key)
    if mutable:
        return max_token, tokens, mutable
    
    key = repr(key).lower()
    if not key in tokens:
        max_token += 1
        cur_token = max_token
        tokens[key] = max_token
    else:
        cur_token = tokens[key]

    return max_token, tokens, cur_token

        
def main(args: argparse.Namespace) -> None:
    max_token = START_TOKEN
    tokens: dict[str, int] = {}
    outfile = sys.stdout
    if args.tokenfile:
        outfile = args.tokenfile
        max_token, tokens = load_tokens(args.tokenfile)

    line_lens = Counter()
    for line in sys.stdin:
        outline = []
        line = json.loads(line)
        for key in sorted(line.keys()):
            max_token, tokens, tok = add_token(max_token, tokens, key)
            outline.append(tok)
            max_token, tokens, tok = add_token(max_token, tokens, line[key])
            outline.append(tok)
        line_lens[len(outline)] += 1
        print(json.dumps(outline))

    print(line_lens.most_common())
    save_tokens(max_token, tokens, outfile)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument("-f", "--tokenfile", help="Tokenfile")
    args = parser.parse_args()
    main(args)
