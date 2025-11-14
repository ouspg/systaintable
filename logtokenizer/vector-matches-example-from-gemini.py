import numpy as np
from itertools import combinations

# --- Setup from previous step ---
log_vectors = [
    [101, 205, 301, 410, 500, 602], # Index 0
    [101, 205, 302, 410, 500, 603], # Index 1
    [101, 206, 301, 411, 500, 602], # Index 2
    [700, 800, 900, 100, 110, 120], # Index 3
    [101, 205, 301, 410, 500, 602], # Index 4 (Duplicate of 0)
    [101, 205, 302, 410, 500, 603], # Index 5 (Duplicate of 1)
    [101, 205, 305, 410, 500, 603]  # Index 6 (Differs from 1 at index 2: 305 vs 302)
]
vectors = np.array(log_vectors)
MAGIC_TOKEN = 0 # Define the magic token for variables

def find_single_difference_pairs(all_vectors):
    """Finds all unique pairs of vectors (indices) that have a Hamming Distance of exactly 1."""
    num_vectors = len(all_vectors)
    single_diff_pairs = []
    
    for i, j in combinations(range(num_vectors), 2):
        # Calculate Hamming Distance
        distance = np.sum(all_vectors[i] != all_vectors[j])
        
        if distance == 1:
            diff_position = np.where(all_vectors[i] != all_vectors[j])[0][0]
            single_diff_pairs.append({'indices': (i, j), 'diff_position': diff_position})
            
    return single_diff_pairs

def create_pattern_rule(vector_a, diff_position, magic_token):
    """Creates a new rule vector by replacing the variable position with the magic token."""
    # Create a copy of the vector to modify
    rule_vector = np.copy(vector_a)
    
    # Replace the token at the difference position with the magic token
    rule_vector[diff_position] = magic_token
    
    return rule_vector.tolist()

def match_vector_to_rule(vector, rule_vector, magic_token):
    """
    Checks if a vector matches the rule.
    A match occurs if:
    1. The vector's token matches the rule's token, OR
    2. The rule's token is the magic token (meaning the vector's token is a variable).
    """
    is_match = True
    
    for v_token, r_token in zip(vector, rule_vector):
        if r_token != magic_token and v_token != r_token:
            # Found a mismatch where the rule requires a specific token (not a variable)
            is_match = False
            break
            
    return is_match

# --- Main Pattern Generation and Matching Process ---

# 1. Find the single-difference pairs
single_diff_pairs = find_single_difference_pairs(vectors)

# Check if any pairs were found
if not single_diff_pairs:
    print("No pairs with Hamming Distance 1 were found to generate a rule.")
else:
    # 2. Select the first pair found
    first_pair_info = single_diff_pairs[0]
    i, j = first_pair_info['indices']
    diff_pos = first_pair_info['diff_position']
    
    vector_i = vectors[i]
    
    print(f"**First single-difference pair found:** Indices ({i}, {j})")
    print(f"  Vectors differ at index: {diff_pos}")
    print("---")
    
    # 3. Create the pattern rule
    rule_vector_list = create_pattern_rule(vector_i, diff_pos, MAGIC_TOKEN)
    rule_vector = np.array(rule_vector_list) # Convert back to array for efficient matching
    
    print(f"**Generated Pattern Rule (Magic Token = {MAGIC_TOKEN}):**")
    print(rule_vector_list)
    print("---")
    
    # 4. Rematch the rule against all original vectors
    matching_indices = []
    for k in range(len(vectors)):
        current_vector = vectors[k]
        
        if match_vector_to_rule(current_vector, rule_vector_list, MAGIC_TOKEN):
            matching_indices.append(k)
            
    print(f"**Vectors matching the new Rule:**")
    print(f"Indices: {matching_indices}")
    print("The following vectors share the exact same pattern (fixed tokens in fixed positions):")
    for index in matching_indices:
        print(f"  Index {index}: {vectors[index].tolist()}")