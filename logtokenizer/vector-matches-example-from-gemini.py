import numpy as np
from itertools import combinations

# --- Setup ---
log_vectors = [
    [101, 205, 301, 410, 500, 602], # Index 0
    [101, 205, 302, 410, 500, 603], # Index 1 (Distance 2 from 0)
    [101, 206, 301, 411, 500, 602], # Index 2 (Distance 2 from 0)
    [700, 800, 900, 100, 110, 120], # Index 3
    [101, 205, 305, 410, 500, 603], # Index 4 (Distance 1 from 1)
    [101, 206, 302, 411, 500, 603]  # Index 5 (Distance 4 from 0)
]
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

# 1. Define the desired Hamming distance
TARGET_D = 2 
print(f"**Searching for pairs with Hamming Distance = {TARGET_D}**")

# 2. Find the pairs
matching_pairs = find_pairs_by_hamming_distance(vectors, TARGET_D)

if not matching_pairs:
    print("\nNo pairs found with the specified Hamming Distance.")
else:
    # 3. Select the first pair
    first_pair_info = matching_pairs[0]
    i, j = first_pair_info['indices']
    diff_positions = first_pair_info['diff_positions']
    vector_i = vectors[i]
    
    print("---")
    print(f"**First pair found:** Indices ({i}, {j})")
    print(f"  Vectors differ at indices: {diff_positions}")
    
    # 4. Create the pattern rule
    rule_vector_list = create_pattern_rule(vector_i, diff_positions, MAGIC_TOKEN)
    
    print(f"\n**Generated Pattern Rule (Magic Token = {MAGIC_TOKEN}):**")
    print(rule_vector_list)
    print("---")
    
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