import ast
import json
import numpy as np
import sys

from itertools import combinations
from collections import defaultdict

log_vectors = [
]

for line in sys.stdin:
    data = json.loads(line)
    log_vectors.append(data)

# Convert to NumPy array
vectors = np.array(log_vectors)

MAGIC_TOKEN = 0

# ----------------------------------------------------
# --- Modified Function for Selectable Distance ---
# ----------------------------------------------------

def find_pairs_by_hamming_distance(all_vectors, target_distance):
    """
    Finds all unique pairs of vectors (indices) that have a Hamming Distance 
    exactly equal to the specified target_distance.
    """
    num_vectors = len(all_vectors)
    matching_pairs = []

    # Iterate through all unique pairs of indices (i, j)
    for i, j in combinations(range(num_vectors), 2):
        vector_i = all_vectors[i]
        vector_j = all_vectors[j]
        
        # Calculate Hamming Distance
        distance = np.sum(vector_i != vector_j)

        # Check against the selectable distance
        if distance == target_distance:
            # Find the indices where the differences occur
            diff_positions = np.where(vector_i != vector_j)[0].tolist()
            
            matching_pairs.append({
                'indices': (i, j), 
                'diff_positions': diff_positions,
                'distance': distance
            })
            
    return matching_pairs

# ----------------------------------------------------
# --- Rule Generation & Matching (Functions from before) ---
# ----------------------------------------------------

def create_pattern_rule(vector_a, diff_positions, magic_token):
    """Creates a new rule vector by replacing the variable positions with the magic token."""
    rule_vector = np.copy(vector_a)
    
    # Replace tokens at ALL difference positions with the magic token
    for pos in diff_positions:
        rule_vector[pos] = magic_token
    
    return rule_vector.tolist()

def match_vector_to_rule(vector, rule_vector, magic_token):
    """
    Checks if a vector matches the rule.
    A match requires fixed rule tokens to match and allows any token for the magic token.
    """
    for v_token, r_token in zip(vector, rule_vector):
        # Mismatch condition: The rule specifies a fixed token, but the vector has a different one.
        if r_token != magic_token and v_token != r_token:
            return False
    return True

# ----------------------------------------------------
# --- Example Usage with Target Distance = 2 ---
# ----------------------------------------------------

RULES = list()
VALUES = defaultdict(dict)
RULENO = 0

while True:
    # 1. Define the desired Hamming distance
    TARGET_D = 2 
    print(f"**Searching for pairs with Hamming Distance = {TARGET_D}**")
    
    # 2. Find the pairs
    matching_pairs = find_pairs_by_hamming_distance(vectors, TARGET_D)

    if not matching_pairs:
        print("\nNo pairs found with the specified Hamming Distance.")
        break
    else:
        # 3. Select the first pair
        first_pair_info = matching_pairs[0]
        i, j = first_pair_info['indices']
        diff_positions = first_pair_info['diff_positions']
        vector_i = vectors[i]
        for pos in diff_positions:
            VALUES[RULENO].setdefault(pos, list()).append(vectors[i][pos])
            VALUES[RULENO].setdefault(pos, list()).append(vectors[j][pos])

        print("---")
        print(f"**First pair found:** Indices ({i}, {j})")
        print(f"  Vectors differ at indices: {diff_positions}")

        # 4. Create the pattern rule
        rule_vector_list = create_pattern_rule(vector_i, diff_positions, MAGIC_TOKEN)

        print(f"\n**Generated Pattern Rule (Magic Token = {MAGIC_TOKEN}):**")
        print(rule_vector_list)
        print("---")

        RULES.append(rule_vector_list)
        RULENO += 1
        
        # 5. Rematch the rule against all original vectors
        matching_indices = []
        for k in range(len(vectors)):
            if match_vector_to_rule(vectors[k], rule_vector_list, MAGIC_TOKEN):
                matching_indices.append(k)

        print(f"**Vectors matching the new Rule:**")
        print(f"Indices: {matching_indices}")
        print("These vectors share the same pattern with variables in the identified positions.")
        for index in matching_indices:
            print(f"  Index {index}: {vectors[index].tolist()}")

        new_vectors = list()
        for i in range(len(vectors)):
            if not i in matching_indices:
                new_vectors.append(vectors[i])
            
        vectors = np.array(new_vectors)

print("Rules")
for rule in RULES:
    print(rule)


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

tokens = load_tokens("tokens.json")

print("Values")
for rule in VALUES:
    for idx in VALUES[rule]:
        for val in VALUES[rule][idx]:
            print(f"rule {rule} idx {idx} token {val} == {ast.literal_eval(tokens[val])} ")
            print(f"preceded by idx {idx-1} token {RULES[rule][idx-1]} == {ast.literal_eval(tokens[RULES[rule][idx-1]])} ")

VALS_PER_RULE = defaultdict(set)
print("Same values in different rules")
for rule in VALUES:
    for idx in VALUES[rule]:
        for val in VALUES[rule][idx]:
            VALS_PER_RULE[val].add(rule)

print(VALS_PER_RULE)
TOKENISED = list()

print("Tokenised")
for i, rule in enumerate(RULES):
    valkey = list(VALUES[i].keys())[0]
    for j in range(len(VALUES[i][valkey])):
        rulebase = list(rule)
        for idx in VALUES[i]:
            rulebase[idx] = VALUES[i][idx][j]
        print(rulebase)
        TOKENISED.append(rulebase)

print("Detokenised")
for i, rule in enumerate(TOKENISED):
    print(f"Rule {i}")
    print([ast.literal_eval(tokens[x]) for x in rule])

# TODO:
# hamming distance for different length vectors

# FIXME OUTPUT selections
# rules tokenized / detokenized
# value sets for rules
# unique values
# rule occurrence vectors
# value set similarity (set operations)
